use super::error::ComtradeError;
use super::model::CfgFile;

pub struct SampleData {
    pub sample_numbers: Vec<u32>,
    pub timestamps_us: Vec<f64>,
    pub analog_samples: Vec<Vec<f32>>,
    pub digital_samples: Vec<Vec<bool>>,
}

/// Parses the ASCII variant of a .dat file: one line per sample,
/// `sample#,raw_timestamp,analog_1..analog_N,digital_1..digital_M`.
pub fn parse_dat_ascii(text: &str, cfg: &CfgFile) -> Result<SampleData, ComtradeError> {
    let n_analog = cfg.analog_channels.len();
    let n_digital = cfg.digital_channels.len();
    let expected = 2 + n_analog + n_digital;

    let mut sample_numbers = Vec::new();
    let mut raw_timestamps: Vec<i64> = Vec::new();
    let mut analog_samples: Vec<Vec<f32>> = vec![Vec::new(); n_analog];
    let mut digital_samples: Vec<Vec<bool>> = vec![Vec::new(); n_digital];

    for (idx, raw_line) in text.lines().enumerate() {
        let line = raw_line.trim();
        if line.is_empty() {
            continue;
        }
        let line_no = idx + 1;
        let fields: Vec<&str> = line.split(',').map(str::trim).collect();
        if fields.len() < expected {
            return Err(ComtradeError::MalformedDatLine {
                line: line_no,
                expected,
                analog: n_analog,
                digital: n_digital,
                found: fields.len(),
            });
        }

        let sn: u32 = fields[0].parse().map_err(|_| ComtradeError::DatParseNumber {
            line: line_no,
            field: "sample_number".to_string(),
        })?;
        let ts: i64 = fields[1].parse().map_err(|_| ComtradeError::DatParseNumber {
            line: line_no,
            field: "timestamp".to_string(),
        })?;
        sample_numbers.push(sn);
        raw_timestamps.push(ts);

        for i in 0..n_analog {
            let raw: f64 = fields[2 + i]
                .parse()
                .map_err(|_| ComtradeError::DatParseNumber {
                    line: line_no,
                    field: format!("analog[{i}]"),
                })?;
            let ch = &cfg.analog_channels[i];
            analog_samples[i].push((raw * ch.a + ch.b) as f32);
        }
        for i in 0..n_digital {
            let raw = fields[2 + n_analog + i];
            digital_samples[i].push(raw == "1");
        }
    }

    let timestamps_us = compute_timestamps(&sample_numbers, &raw_timestamps, cfg);

    Ok(SampleData {
        sample_numbers,
        timestamps_us,
        analog_samples,
        digital_samples,
    })
}

/// Parses the standard 16-bit binary variant of a .dat file ("BINARY" in the .cfg
/// file type line): fixed-size records of
/// `sample#(u32 LE) | timestamp(u32 LE) | analog(i16 LE)*N | digital(u16 LE bitfield)*ceil(M/16)`.
pub fn parse_dat_binary16(bytes: &[u8], cfg: &CfgFile) -> Result<SampleData, ComtradeError> {
    let n_analog = cfg.analog_channels.len();
    let n_digital = cfg.digital_channels.len();
    let digital_words = n_digital.div_ceil(16);
    let record_size = 8 + 2 * n_analog + 2 * digital_words;

    if record_size == 0 || !bytes.len().is_multiple_of(record_size) {
        return Err(ComtradeError::TruncatedBinaryDat {
            expected: record_size,
            found: bytes.len(),
            samples: bytes.len() / record_size.max(1),
        });
    }
    let n_samples = bytes.len() / record_size;

    let mut sample_numbers = Vec::with_capacity(n_samples);
    let mut raw_timestamps: Vec<i64> = Vec::with_capacity(n_samples);
    let mut analog_samples: Vec<Vec<f32>> = vec![Vec::with_capacity(n_samples); n_analog];
    let mut digital_samples: Vec<Vec<bool>> = vec![Vec::with_capacity(n_samples); n_digital];

    for rec in 0..n_samples {
        let base = rec * record_size;

        let sn = u32::from_le_bytes(bytes[base..base + 4].try_into().unwrap());
        let ts = u32::from_le_bytes(bytes[base + 4..base + 8].try_into().unwrap());
        sample_numbers.push(sn);
        raw_timestamps.push(ts as i64);

        let analog_base = base + 8;
        for i in 0..n_analog {
            let off = analog_base + 2 * i;
            let raw = i16::from_le_bytes(bytes[off..off + 2].try_into().unwrap());
            let ch = &cfg.analog_channels[i];
            analog_samples[i].push((raw as f64 * ch.a + ch.b) as f32);
        }

        let digital_base = analog_base + 2 * n_analog;
        for word_idx in 0..digital_words {
            let off = digital_base + 2 * word_idx;
            let word = u16::from_le_bytes(bytes[off..off + 2].try_into().unwrap());
            for bit in 0..16 {
                let ch_idx = word_idx * 16 + bit;
                if ch_idx >= n_digital {
                    break;
                }
                digital_samples[ch_idx].push((word >> bit) & 1 == 1);
            }
        }
    }

    let timestamps_us = compute_timestamps(&sample_numbers, &raw_timestamps, cfg);

    Ok(SampleData {
        sample_numbers,
        timestamps_us,
        analog_samples,
        digital_samples,
    })
}

/// Derives relative microsecond timestamps for each sample.
///
/// Per spec, the per-sample raw timestamp field may be all-zero, in which case timing
/// must instead be derived from the cfg's sample-rate segments. When real (non-zero)
/// per-sample timestamps are present, those are authoritative (scaled by
/// `time_multiplier`) since they can capture rate jitter the segment table can't.
fn compute_timestamps(sample_numbers: &[u32], raw_timestamps: &[i64], cfg: &CfgFile) -> Vec<f64> {
    let all_zero = raw_timestamps.iter().all(|&t| t == 0);

    if !all_zero {
        return raw_timestamps
            .iter()
            .map(|&t| t as f64 * cfg.time_multiplier)
            .collect();
    }

    if cfg.sample_rates.is_empty() {
        // No rate table and no usable per-sample timestamps: fall back to a bare
        // sample-index axis so plotting still works, clearly not time-calibrated.
        return sample_numbers.iter().map(|&sn| sn as f64).collect();
    }

    let mut result = Vec::with_capacity(sample_numbers.len());
    let mut seg_idx = 0usize;
    let mut seg_start_sample = sample_numbers.first().copied().unwrap_or(1);
    let mut seg_start_time_us = 0.0f64;

    for &sn in sample_numbers {
        while seg_idx + 1 < cfg.sample_rates.len() && sn > cfg.sample_rates[seg_idx].end_sample {
            let seg = &cfg.sample_rates[seg_idx];
            let dt_us = if seg.samp_hz > 0.0 { 1_000_000.0 / seg.samp_hz } else { 0.0 };
            let samples_in_seg = seg.end_sample.saturating_sub(seg_start_sample) + 1;
            seg_start_time_us += dt_us * samples_in_seg as f64;
            seg_start_sample = seg.end_sample + 1;
            seg_idx += 1;
        }
        let seg = &cfg.sample_rates[seg_idx];
        let dt_us = if seg.samp_hz > 0.0 { 1_000_000.0 / seg.samp_hz } else { 0.0 };
        let offset_in_seg = sn.saturating_sub(seg_start_sample);
        result.push(seg_start_time_us + dt_us * offset_in_seg as f64);
    }

    result
}
