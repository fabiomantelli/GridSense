use serde::Serialize;

use super::rms::overall_rms;
use crate::comtrade::ComtradeRecord;

#[derive(Debug, Clone, Serialize)]
pub struct RecordSummary {
    pub station_name: String,
    pub device_id: String,
    pub sample_count: usize,
    pub duration_us: f64,
    pub line_frequency: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct ChannelSummary {
    pub index: usize,
    pub id: String,
    pub units: String,
    pub min: f32,
    pub max: f32,
    pub mean: f32,
    pub rms: f32,
}

pub fn record_summary(record: &ComtradeRecord) -> RecordSummary {
    let duration_us = match (record.timestamps_us.first(), record.timestamps_us.last()) {
        (Some(&first), Some(&last)) => last - first,
        _ => 0.0,
    };
    RecordSummary {
        station_name: record.cfg.station_name.clone(),
        device_id: record.cfg.device_id.clone(),
        sample_count: record.sample_numbers.len(),
        duration_us,
        line_frequency: record.cfg.line_frequency,
    }
}

pub fn analog_channel_summaries(record: &ComtradeRecord) -> Vec<ChannelSummary> {
    record
        .cfg
        .analog_channels
        .iter()
        .enumerate()
        .map(|(i, def)| {
            let samples = &record.analog_samples[i];
            let (min, max, sum) = samples.iter().fold(
                (f32::INFINITY, f32::NEG_INFINITY, 0.0f64),
                |(min, max, sum), &x| (min.min(x), max.max(x), sum + x as f64),
            );
            let mean = if samples.is_empty() { 0.0 } else { (sum / samples.len() as f64) as f32 };
            ChannelSummary {
                index: i,
                id: def.id.clone(),
                units: def.units.clone(),
                min: if samples.is_empty() { 0.0 } else { min },
                max: if samples.is_empty() { 0.0 } else { max },
                mean,
                rms: overall_rms(samples),
            }
        })
        .collect()
}
