use serde::Serialize;

use super::digital_timeline::build_timeline;
use super::event_detection::{detect_step_changes, EventMarker, StepDirection};
use super::phasor::extract_phasor;
use super::rms::{rms_at, windowed_rms};
use super::symmetrical_components::sequence_components;
use crate::comtrade::CfgFile;
use crate::util::{effective_sample_rate_hz, group_three_phase, quantity_kind, QuantityKind};

pub const DEFAULT_STEP_THRESHOLD_PCT: f32 = 20.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum Phase {
    A,
    B,
    C,
}

impl Phase {
    pub fn label(self) -> &'static str {
        match self {
            Phase::A => "A",
            Phase::B => "B",
            Phase::C => "C",
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub enum FaultKind {
    PhaseToGround(Phase),
    PhaseToPhase(Phase, Phase),
    ThreePhase,
    /// A correlated voltage-sag + current-rise step was detected but there isn't
    /// enough evidence (a matching 3-phase voltage group, or a clear phase count) to
    /// classify it further. Reported as-is rather than guessing — an LLM explaining
    /// this later should say "unclassified", not invent a fault type.
    Unclassified,
}

#[derive(Debug, Clone, Serialize)]
pub struct EventClassification {
    pub kind: FaultKind,
    pub onset_sample: usize,
    pub onset_time_us: f64,
    /// Base label (e.g. "V"/"I") of the channel group the onset was first detected
    /// in; approximate when a cluster spans multiple groups.
    pub involved_group_label: String,
    /// Fault-period RMS / pre-event RMS for the involved current channel, if any —
    /// a proxy for "multiple of nominal" using the record's own recent baseline
    /// rather than a separately configured nameplate rating.
    pub current_multiple: Option<f32>,
    pub breaker_channel_id: Option<String>,
    pub time_to_trip_us: Option<f64>,
}

struct PhaseStepEvent {
    quantity: QuantityKind,
    phase: Phase,
    channel_index: usize,
    group_label: String,
    marker: EventMarker,
}

fn collect_phase_step_events(
    cfg: &CfgFile,
    analog_samples: &[Vec<f32>],
    cycle_len: usize,
    threshold_pct: f32,
) -> Vec<PhaseStepEvent> {
    let mut events = Vec::new();

    for group in group_three_phase(&cfg.analog_channels) {
        let kind = quantity_kind(&group.units);
        if kind == QuantityKind::Other {
            continue;
        }
        for (phase, idx) in [
            (Phase::A, group.a_index),
            (Phase::B, group.b_index),
            (Phase::C, group.c_index),
        ] {
            let rms = windowed_rms(&analog_samples[idx], cycle_len);
            for marker in detect_step_changes(&rms, cycle_len, threshold_pct) {
                // Only the physically-meaningful direction per quantity: voltage sags
                // (Fall) and current inrush (Rise). This also filters out the
                // "recovery" edge that detect_step_changes reports as a second event.
                let relevant = match kind {
                    QuantityKind::Voltage => marker.direction == StepDirection::Fall,
                    QuantityKind::Current => marker.direction == StepDirection::Rise,
                    QuantityKind::Other => false,
                };
                if relevant {
                    events.push(PhaseStepEvent {
                        quantity: kind,
                        phase,
                        channel_index: idx,
                        group_label: group.base_label.clone(),
                        marker,
                    });
                }
            }
        }
    }

    events.sort_by_key(|e| e.marker.sample_index);
    events
}

/// Greedily clusters step events into "one physical event" groups: any event within
/// `cycle_len` samples of its cluster's first (earliest) event joins that cluster.
fn cluster_events(events: Vec<PhaseStepEvent>, cycle_len: usize) -> Vec<Vec<PhaseStepEvent>> {
    let mut clusters: Vec<Vec<PhaseStepEvent>> = Vec::new();
    for event in events {
        let starts_new_cluster = match clusters.last() {
            Some(last) => event.marker.sample_index.saturating_sub(last[0].marker.sample_index) > cycle_len,
            None => true,
        };
        if starts_new_cluster {
            clusters.push(vec![event]);
        } else {
            clusters.last_mut().unwrap().push(event);
        }
    }
    clusters
}

/// Checks whether a ground-involved fault is indicated at `onset_sample`, using the
/// first available three-phase voltage group. v1 simplification: records with more
/// than one 3-phase voltage source aren't distinguished — the first group found is
/// used regardless of which current group correlated with the onset.
fn check_ground_involvement(cfg: &CfgFile, analog_samples: &[Vec<f32>], fs: f64, onset_sample: usize) -> Option<bool> {
    let group = group_three_phase(&cfg.analog_channels)
        .into_iter()
        .find(|g| quantity_kind(&g.units) == QuantityKind::Voltage)?;

    let f0 = cfg.line_frequency;
    let va = extract_phasor(&analog_samples[group.a_index], fs, f0, onset_sample)?;
    let vb = extract_phasor(&analog_samples[group.b_index], fs, f0, onset_sample)?;
    let vc = extract_phasor(&analog_samples[group.c_index], fs, f0, onset_sample)?;

    let seq = sequence_components(va, vb, vc);
    Some(seq.zero.magnitude() > 0.1 * seq.positive.magnitude())
}

/// Finds the earliest digital-channel deviation from its declared normal state after
/// `onset_sample` — the standard proxy for "breaker opened in response to this event"
/// when the record only carries one relay's local digital channels (see architecture
/// note on plan doc: cross-device SOE correlation is a later extension, not v1).
fn find_breaker_trip(
    cfg: &CfgFile,
    digital_samples: &[Vec<bool>],
    onset_sample: usize,
    onset_time_us: f64,
    timestamps_us: &[f64],
) -> (Option<String>, Option<f64>) {
    let mut best: Option<(String, f64)> = None;

    for (i, def) in cfg.digital_channels.iter().enumerate() {
        let timeline = build_timeline(i, &def.id, &digital_samples[i]);
        for change in &timeline.changes {
            if change.sample_index <= onset_sample || change.state == def.normal_state {
                continue;
            }
            let Some(&t) = timestamps_us.get(change.sample_index) else { continue };
            let dt = t - onset_time_us;
            if best.as_ref().is_none_or(|(_, best_dt)| dt < *best_dt) {
                best = Some((def.id.clone(), dt));
            }
        }
    }

    match best {
        Some((id, dt)) => (Some(id), Some(dt)),
        None => (None, None),
    }
}

/// The full deterministic pipeline: detect per-phase voltage-sag/current-rise steps,
/// cluster the ones that co-occur, classify each cluster's fault type from phase
/// count + zero-sequence presence, and correlate with the nearest breaker trip.
/// Nothing here is inferred by an LLM — every field is computed from the parsed
/// samples so it can be handed to an explanation layer as an already-proven fact.
pub fn classify_events(
    cfg: &CfgFile,
    analog_samples: &[Vec<f32>],
    digital_samples: &[Vec<bool>],
    timestamps_us: &[f64],
    threshold_pct: f32,
) -> Vec<EventClassification> {
    let fs = effective_sample_rate_hz(cfg, timestamps_us);
    if fs <= 0.0 || cfg.line_frequency <= 0.0 {
        return Vec::new();
    }
    let cycle_len = (fs / cfg.line_frequency).round() as usize;
    if cycle_len == 0 {
        return Vec::new();
    }

    let phase_events = collect_phase_step_events(cfg, analog_samples, cycle_len, threshold_pct);
    if phase_events.is_empty() {
        return Vec::new();
    }

    cluster_events(phase_events, cycle_len)
        .into_iter()
        .map(|cluster| {
            let onset_sample = cluster.iter().map(|e| e.marker.sample_index).min().unwrap();
            let onset_time_us = timestamps_us.get(onset_sample).copied().unwrap_or(0.0);
            let group_label = cluster[0].group_label.clone();

            let mut sag_phases = Vec::new();
            let mut rise_phases = Vec::new();
            for e in &cluster {
                match e.quantity {
                    QuantityKind::Voltage => sag_phases.push(e.phase),
                    QuantityKind::Current => rise_phases.push(e.phase),
                    QuantityKind::Other => {}
                }
            }
            let involved: Vec<Phase> = [Phase::A, Phase::B, Phase::C]
                .into_iter()
                .filter(|p| sag_phases.contains(p) && rise_phases.contains(p))
                .collect();

            let kind = match involved.as_slice() {
                [p] => match check_ground_involvement(cfg, analog_samples, fs, onset_sample) {
                    Some(true) => FaultKind::PhaseToGround(*p),
                    _ => FaultKind::Unclassified,
                },
                [p1, p2] => FaultKind::PhaseToPhase(*p1, *p2),
                [_, _, _] => FaultKind::ThreePhase,
                _ => FaultKind::Unclassified,
            };

            // Read the fault-period ratio one full cycle after onset rather than at
            // the threshold-crossing sample itself: the crossing point's RMS window
            // is still mostly pre-fault samples, so it understates the settled
            // fault current by a large margin.
            let current_multiple = cluster.iter().find(|e| e.quantity == QuantityKind::Current).map(|e| {
                let channel = &analog_samples[e.channel_index];
                let settle_index = (e.marker.sample_index + cycle_len).min(channel.len().saturating_sub(1));
                let settled_value = rms_at(channel, cycle_len, settle_index);
                settled_value / e.marker.baseline.max(1e-6)
            });

            let (breaker_channel_id, time_to_trip_us) =
                find_breaker_trip(cfg, digital_samples, onset_sample, onset_time_us, timestamps_us);

            EventClassification {
                kind,
                onset_sample,
                onset_time_us,
                involved_group_label: group_label,
                current_multiple,
                breaker_channel_id,
                time_to_trip_us,
            }
        })
        .collect()
}
