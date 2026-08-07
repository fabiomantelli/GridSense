use crate::comtrade::AnalogChannelDef;

#[derive(Debug, Clone)]
pub struct ThreePhaseGroup {
    /// Channel id with the trailing phase letter stripped, e.g. "VA"/"VB"/"VC" -> "V".
    pub base_label: String,
    pub units: String,
    pub a_index: usize,
    pub b_index: usize,
    pub c_index: usize,
}

/// Groups analog channels into three-phase sets using each channel's `phase` field
/// (A/B/C) plus a shared id prefix/suffix and matching units — e.g. VA/VB/VC or
/// IA/IB/IC. Channels without a recognized phase, or whose group doesn't have all
/// three phases present, are left out (no group is invented from partial data).
///
/// Insertion order is preserved (not a `HashMap`) so results are deterministic and
/// stable for auditing/testing, matching first-seen channel order in the .cfg file.
pub fn group_three_phase(channels: &[AnalogChannelDef]) -> Vec<ThreePhaseGroup> {
    struct Entry {
        base: String,
        units: String,
        a: Option<usize>,
        b: Option<usize>,
        c: Option<usize>,
    }

    let mut entries: Vec<Entry> = Vec::new();

    for (i, ch) in channels.iter().enumerate() {
        let phase = match ch.phase.as_deref().map(|p| p.trim().to_ascii_uppercase()) {
            Some(p) if p == "A" || p == "B" || p == "C" => p,
            _ => continue,
        };
        let base = strip_trailing_phase_letter(&ch.id, &phase);

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
        match phase.as_str() {
            "A" => entry.a = Some(i),
            "B" => entry.b = Some(i),
            "C" => entry.c = Some(i),
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

fn strip_trailing_phase_letter(id: &str, phase: &str) -> String {
    if id.to_ascii_uppercase().ends_with(phase) {
        id[..id.len() - phase.len()].to_string()
    } else {
        id.to_string()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuantityKind {
    Voltage,
    Current,
    Other,
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
