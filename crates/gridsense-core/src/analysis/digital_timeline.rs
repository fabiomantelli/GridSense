#[derive(Debug, Clone, Copy)]
pub struct DigitalStateChange {
    pub sample_index: usize,
    pub state: bool,
}

#[derive(Debug, Clone)]
pub struct DigitalChannelTimeline {
    pub channel_index: usize,
    pub channel_id: String,
    /// Run-length-encoded: one entry per state transition, starting with the initial
    /// state at sample 0. Empty only if `samples` was empty.
    pub changes: Vec<DigitalStateChange>,
}

pub fn build_timeline(channel_index: usize, channel_id: &str, samples: &[bool]) -> DigitalChannelTimeline {
    let mut changes = Vec::new();
    if let Some(&first) = samples.first() {
        let mut state = first;
        changes.push(DigitalStateChange { sample_index: 0, state });
        for (i, &s) in samples.iter().enumerate().skip(1) {
            if s != state {
                changes.push(DigitalStateChange { sample_index: i, state: s });
                state = s;
            }
        }
    }
    DigitalChannelTimeline {
        channel_index,
        channel_id: channel_id.to_string(),
        changes,
    }
}
