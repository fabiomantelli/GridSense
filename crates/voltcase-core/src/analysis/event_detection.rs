#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StepDirection {
    Rise,
    Fall,
}

#[derive(Debug, Clone, Copy)]
pub struct EventMarker {
    pub sample_index: usize,
    pub direction: StepDirection,
    /// RMS value one cycle before `sample_index`.
    pub baseline: f32,
    /// RMS value at `sample_index`.
    pub value: f32,
    pub change_pct: f32,
}

/// Flags the *onset* of a sustained cycle-over-cycle RMS deviation (a voltage sag,
/// current inrush, etc.) by comparing each point in a windowed-RMS series to the
/// point one cycle earlier. Only rising edges into "stepped" state are reported —
/// not every sample during a sustained excursion — so one physical event produces
/// one marker.
///
/// Comparisons only start once *both* points are backed by a full one-cycle RMS
/// window (i.e. from `2 * cycle_len` onward): `windowed_rms` uses an expanding
/// window for its first `cycle_len - 1` points, and comparing a full window against
/// that still-filling one produces a spurious "step" as the window transitions —
/// not a real signal event.
pub fn detect_step_changes(rms_series: &[f32], cycle_len: usize, threshold_pct: f32) -> Vec<EventMarker> {
    let mut events = Vec::new();
    if cycle_len == 0 || rms_series.len() <= 2 * cycle_len {
        return events;
    }

    let mut was_stepped = false;
    for i in (2 * cycle_len)..rms_series.len() {
        let baseline = rms_series[i - cycle_len];
        let value = rms_series[i];
        if baseline.abs() < f32::EPSILON {
            was_stepped = false;
            continue;
        }
        let change_pct = (value - baseline) / baseline * 100.0;
        let stepped = change_pct.abs() >= threshold_pct;
        if stepped && !was_stepped {
            events.push(EventMarker {
                sample_index: i,
                direction: if change_pct > 0.0 { StepDirection::Rise } else { StepDirection::Fall },
                baseline,
                value,
                change_pct,
            });
        }
        was_stepped = stepped;
    }
    events
}
