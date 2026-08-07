use serde::Serialize;

use super::event_correlation::{classify_events, EventClassification, DEFAULT_STEP_THRESHOLD_PCT};
use super::phasor::extract_phasor;
use super::summary::{analog_channel_summaries, record_summary, ChannelSummary, RecordSummary};
use super::symmetrical_components::sequence_components;
use crate::comtrade::ComtradeRecord;
use crate::util::{group_three_phase, quantity_kind, QuantityKind};

#[derive(Debug, Clone, Serialize)]
pub struct SequenceGroupResult {
    pub group_label: String,
    pub units: String,
    /// Sample index the phasors were evaluated at (currently always the record's
    /// first cycle — a steady-state/pre-event balance snapshot, not a per-cycle
    /// trend; per-cycle sequence tracking is a natural v1.1 extension).
    pub sample_index: usize,
    pub zero_magnitude: f64,
    pub positive_magnitude: f64,
    pub negative_magnitude: f64,
}

/// The complete structured output of the deterministic analysis engine: everything a
/// UI facts panel or a (v1.1, opt-in) LLM explanation layer needs, with no raw sample
/// arrays included — only named, unit-labeled, already-computed facts.
#[derive(Debug, Clone, Serialize)]
pub struct AnalysisFacts {
    pub record_summary: RecordSummary,
    pub channel_summaries: Vec<ChannelSummary>,
    pub sequence_component_groups: Vec<SequenceGroupResult>,
    pub events: Vec<EventClassification>,
}

pub fn analyze(record: &ComtradeRecord) -> AnalysisFacts {
    let fs = record.cfg.sample_rates.first().map(|s| s.samp_hz).unwrap_or(0.0);
    let f0 = record.cfg.line_frequency;
    let cycle_len = if fs > 0.0 && f0 > 0.0 { (fs / f0).round() as usize } else { 0 };

    let sequence_component_groups = if cycle_len > 0 {
        group_three_phase(&record.cfg.analog_channels)
            .into_iter()
            .filter(|g| quantity_kind(&g.units) != QuantityKind::Other)
            .filter_map(|g| {
                let va = extract_phasor(&record.analog_samples[g.a_index], fs, f0, 0)?;
                let vb = extract_phasor(&record.analog_samples[g.b_index], fs, f0, 0)?;
                let vc = extract_phasor(&record.analog_samples[g.c_index], fs, f0, 0)?;
                let seq = sequence_components(va, vb, vc);
                Some(SequenceGroupResult {
                    group_label: g.base_label,
                    units: g.units,
                    sample_index: 0,
                    zero_magnitude: seq.zero.magnitude(),
                    positive_magnitude: seq.positive.magnitude(),
                    negative_magnitude: seq.negative.magnitude(),
                })
            })
            .collect()
    } else {
        Vec::new()
    };

    let events = classify_events(
        &record.cfg,
        &record.analog_samples,
        &record.digital_samples,
        &record.timestamps_us,
        DEFAULT_STEP_THRESHOLD_PCT,
    );

    AnalysisFacts {
        record_summary: record_summary(record),
        channel_summaries: analog_channel_summaries(record),
        sequence_component_groups,
        events,
    }
}
