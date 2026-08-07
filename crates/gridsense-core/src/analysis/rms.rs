/// One-cycle sliding-window RMS, same length as `samples`. The first `cycle_len - 1`
/// points use an expanding (shorter) window rather than `NaN`/being dropped, so the
/// output stays index-aligned with the input for charting and downstream event
/// detection — those early values are simply less precise, which is fine since real
/// events never occur in the first cycle of a trigger-centered record.
pub fn windowed_rms(samples: &[f32], cycle_len: usize) -> Vec<f32> {
    if samples.is_empty() || cycle_len == 0 {
        return vec![0.0; samples.len()];
    }

    let mut out = Vec::with_capacity(samples.len());
    let mut sum_sq = 0.0f64;
    for (i, &x) in samples.iter().enumerate() {
        sum_sq += (x as f64) * (x as f64);
        if i >= cycle_len {
            let dropped = samples[i - cycle_len] as f64;
            sum_sq -= dropped * dropped;
        }
        let window_len = (i + 1).min(cycle_len) as f64;
        out.push((sum_sq / window_len).sqrt() as f32);
    }
    out
}

/// RMS of the single one-cycle window ending at `index` (inclusive), clamped to
/// whatever precedes it if fewer than `cycle_len` samples are available. Useful for
/// reading a settled RMS value at one specific point without materializing the whole
/// series via `windowed_rms`.
pub fn rms_at(samples: &[f32], cycle_len: usize, index: usize) -> f32 {
    if samples.is_empty() || cycle_len == 0 {
        return 0.0;
    }
    let end = index.min(samples.len() - 1);
    let start = end.saturating_sub(cycle_len - 1);
    let window = &samples[start..=end];
    let sum_sq: f64 = window.iter().map(|&x| (x as f64) * (x as f64)).sum();
    (sum_sq / window.len() as f64).sqrt() as f32
}

/// RMS over the entire slice (not windowed).
pub fn overall_rms(samples: &[f32]) -> f32 {
    if samples.is_empty() {
        return 0.0;
    }
    let sum_sq: f64 = samples.iter().map(|&x| (x as f64) * (x as f64)).sum();
    (sum_sq / samples.len() as f64).sqrt() as f32
}
