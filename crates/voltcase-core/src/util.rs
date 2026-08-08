use crate::comtrade::{AnalogChannelDef, CfgFile};

#[derive(Debug, Clone)]
pub struct ThreePhaseGroup {
    /// Channel id with the trailing phase letter stripped, e.g. "VA"/"VB"/"VC" -> "V".
    pub base_label: String,
    pub units: String,
    pub a_index: usize,
    pub b_index: usize,
    pub c_index: usize,
}

/// Groups analog channels into three-phase sets, trying two heuristics in order —
/// real-world vendor exports use both conventions and neither alone is reliable:
///
/// 1. **Shared id prefix/suffix**: channels whose id differs only by a trailing phase
///    letter and share units, e.g. VA/VB/VC or IA/IB/IC.
/// 2. **Positional**: three consecutive channels (not already grouped by #1) whose
///    `phase` fields read A, B, C in order and share units — the convention used when
///    each phase gets its own leading index instead of a shared name, e.g.
///    `F1-IA`/`F2-IB`/`F3-IC` (seen in real relay/DFR exports).
///
/// Channels without a recognized phase, or whose group doesn't have all three phases
/// present, are left out (no group is invented from partial data). Insertion order is
/// preserved (not a `HashMap`) so results are deterministic and stable for
/// auditing/testing.
pub fn group_three_phase(channels: &[AnalogChannelDef]) -> Vec<ThreePhaseGroup> {
    let mut groups = group_by_shared_id(channels);
    let mut used: Vec<usize> = groups.iter().flat_map(|g| [g.a_index, g.b_index, g.c_index]).collect();

    let mut i = 0;
    while i + 2 < channels.len() {
        if used.contains(&i) || used.contains(&(i + 1)) || used.contains(&(i + 2)) {
            i += 1;
            continue;
        }
        let (a, b, c) = (&channels[i], &channels[i + 1], &channels[i + 2]);
        let same_units = a.units == b.units && a.units == c.units;
        let phases_in_order = phase_letter(a) == Some('A') && phase_letter(b) == Some('B') && phase_letter(c) == Some('C');
        if same_units && phases_in_order {
            groups.push(ThreePhaseGroup {
                base_label: common_prefix_label(&[&a.id, &b.id, &c.id]),
                units: a.units.clone(),
                a_index: i,
                b_index: i + 1,
                c_index: i + 2,
            });
            used.extend([i, i + 1, i + 2]);
            i += 3;
        } else {
            i += 1;
        }
    }

    groups
}

fn phase_letter(ch: &AnalogChannelDef) -> Option<char> {
    match ch.phase.as_deref().map(|p| p.trim().to_ascii_uppercase()) {
        Some(p) if p.len() == 1 => p.chars().next(),
        _ => None,
    }
}

fn group_by_shared_id(channels: &[AnalogChannelDef]) -> Vec<ThreePhaseGroup> {
    struct Entry {
        base: String,
        units: String,
        a: Option<usize>,
        b: Option<usize>,
        c: Option<usize>,
    }

    let mut entries: Vec<Entry> = Vec::new();

    for (i, ch) in channels.iter().enumerate() {
        let phase = match phase_letter(ch) {
            Some(p @ ('A' | 'B' | 'C')) => p,
            _ => continue,
        };
        let base = strip_trailing_phase_letter(&ch.id, phase);

        let pos = entries.iter().position(|e| e.base == base && e.units == ch.units);
        let entry = match pos {
            Some(idx) => &mut entries[idx],
            None => {
                entries.push(Entry {
                    base: base.clone(),
                    units: ch.units.clone(),
                    a: None,
                    b: None,
                    c: None,
                });
                entries.last_mut().unwrap()
            }
        };
        match phase {
            'A' => entry.a = Some(i),
            'B' => entry.b = Some(i),
            'C' => entry.c = Some(i),
            _ => unreachable!(),
        }
    }

    entries
        .into_iter()
        .filter_map(|e| match (e.a, e.b, e.c) {
            (Some(a), Some(b), Some(c)) => Some(ThreePhaseGroup {
                base_label: e.base,
                units: e.units,
                a_index: a,
                b_index: b,
                c_index: c,
            }),
            _ => None,
        })
        .collect()
}

fn strip_trailing_phase_letter(id: &str, phase: char) -> String {
    if id.to_ascii_uppercase().ends_with(phase) {
        id[..id.len() - phase.len_utf8()].to_string()
    } else {
        id.to_string()
    }
}

/// Longest shared leading substring of the given ids, trimmed of trailing
/// separators, falling back to the first id if there's no useful shared prefix (e.g.
/// "F1-IA"/"F2-IB"/"F3-IC" -> "F").
fn common_prefix_label(ids: &[&str]) -> String {
    let Some((&first, rest)) = ids.split_first() else {
        return String::new();
    };
    let mut prefix_len = first.len();
    for id in rest {
        let shared = first
            .char_indices()
            .zip(id.chars())
            .take_while(|((_, a), b)| a == b)
            .last()
            .map(|((idx, c), _)| idx + c.len_utf8())
            .unwrap_or(0);
        prefix_len = prefix_len.min(shared);
    }
    let trimmed = first[..prefix_len].trim_end_matches(['-', ' ', '_']);
    if trimmed.is_empty() {
        first.to_string()
    } else {
        trimmed.to_string()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuantityKind {
    Voltage,
    Current,
    Other,
}

/// Resolves an effective sampling rate in Hz for cycle-length-based analysis
/// (windowed RMS, phasor extraction): prefers a nonzero rate from the cfg's
/// sample-rate table, falling back to the average interval between consecutive
/// samples in `timestamps_us` when the declared rate is 0. Real-world files with
/// `nrates=0` legitimately declare `samp_hz=0` and expect timing to be read from the
/// per-sample timestamp field instead — the same case `comtrade::dat`'s timestamp
/// derivation already handles, so this mirrors that fallback for the analysis engine.
pub fn effective_sample_rate_hz(cfg: &CfgFile, timestamps_us: &[f64]) -> f64 {
    if let Some(samp_hz) = cfg.sample_rates.first().map(|s| s.samp_hz) {
        if samp_hz > 0.0 {
            return samp_hz;
        }
    }
    if timestamps_us.len() < 2 {
        return 0.0;
    }
    let span_us = timestamps_us[timestamps_us.len() - 1] - timestamps_us[0];
    if span_us <= 0.0 {
        return 0.0;
    }
    1_000_000.0 / (span_us / (timestamps_us.len() - 1) as f64)
}

/// Heuristic quantity classification from a channel's units string (e.g. "V"/"kV" vs
/// "A"/"kA"). Deliberately simple — real-world unit strings are inconsistent enough
/// that a fuller unit-parsing system would be over-engineering for what's needed here.
pub fn quantity_kind(units: &str) -> QuantityKind {
    let u = units.trim().to_ascii_uppercase();
    if u.ends_with('V') {
        QuantityKind::Voltage
    } else if u.ends_with('A') {
        QuantityKind::Current
    } else {
        QuantityKind::Other
    }
}
