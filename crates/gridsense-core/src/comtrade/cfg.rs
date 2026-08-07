use super::error::ComtradeError;
use super::model::{
    AnalogChannelDef, CfgFile, DatFormat, DigitalChannelDef, Revision, SampleRateSegment,
};

/// Sequential reader over the .cfg file's lines, giving each parsing step access to the
/// next raw line (for lines consumed as a single blob, e.g. timestamps) or its
/// comma-split fields (for structured lines).
struct LineReader<'a> {
    lines: std::iter::Peekable<std::str::Lines<'a>>,
    line_no: usize,
}

impl<'a> LineReader<'a> {
    fn new(text: &'a str) -> Self {
        Self {
            lines: text.lines().peekable(),
            line_no: 0,
        }
    }

    fn next_raw(&mut self) -> Option<&'a str> {
        let line = self.lines.next()?;
        self.line_no += 1;
        Some(line.trim())
    }

    fn next_fields(&mut self) -> Option<(usize, Vec<&'a str>)> {
        let line = self.next_raw()?;
        Some((self.line_no, line.split(',').map(str::trim).collect()))
    }

    fn peek_is_present(&mut self) -> bool {
        self.lines.peek().is_some()
    }
}

fn opt_str(s: Option<&&str>) -> Option<String> {
    let s = (*s?).trim();
    if s.is_empty() {
        None
    } else {
        Some(s.to_string())
    }
}

fn req_f64(fields: &[&str], idx: usize, line: usize, field: &'static str) -> Result<f64, ComtradeError> {
    fields
        .get(idx)
        .and_then(|s| s.trim().parse::<f64>().ok())
        .ok_or(ComtradeError::CfgParseNumber { line, field })
}

fn opt_f64(fields: &[&str], idx: usize, default: f64) -> f64 {
    fields
        .get(idx)
        .and_then(|s| s.trim().parse::<f64>().ok())
        .unwrap_or(default)
}

/// Strips a trailing unit-suffix letter (e.g. "4A" -> 4, "1D" -> 1) used on the total
/// channel-count line.
fn parse_count_with_suffix(field: &str) -> Option<usize> {
    let digits: String = field.chars().filter(|c| c.is_ascii_digit()).collect();
    digits.parse().ok()
}

fn parse_analog_channel(fields: &[&str], line: usize) -> Result<AnalogChannelDef, ComtradeError> {
    if fields.len() < 7 {
        return Err(ComtradeError::MalformedAnalogChannel {
            line,
            found: fields.len(),
        });
    }
    Ok(AnalogChannelDef {
        index: fields[0].parse().unwrap_or(0),
        id: fields.get(1).unwrap_or(&"").to_string(),
        phase: opt_str(fields.get(2)),
        circuit_component: opt_str(fields.get(3)),
        units: fields.get(4).unwrap_or(&"").to_string(),
        a: req_f64(fields, 5, line, "a")?,
        b: req_f64(fields, 6, line, "b")?,
        skew: opt_f64(fields, 7, 0.0),
        min: opt_f64(fields, 8, f64::MIN),
        max: opt_f64(fields, 9, f64::MAX),
        primary: opt_f64(fields, 10, 1.0),
        secondary: opt_f64(fields, 11, 1.0),
        ps: fields
            .get(12)
            .and_then(|s| s.trim().chars().next())
            .unwrap_or('P'),
    })
}

fn parse_digital_channel(fields: &[&str], line: usize) -> Result<DigitalChannelDef, ComtradeError> {
    if fields.len() < 2 {
        return Err(ComtradeError::MalformedDigitalChannel {
            line,
            found: fields.len(),
        });
    }
    Ok(DigitalChannelDef {
        index: fields[0].parse().unwrap_or(0),
        id: fields.get(1).unwrap_or(&"").to_string(),
        phase: opt_str(fields.get(2)),
        circuit_component: opt_str(fields.get(3)),
        normal_state: fields.get(4).map(|s| s.trim() == "1").unwrap_or(false),
    })
}

fn parse_dat_format(s: &str) -> DatFormat {
    match s.trim().to_ascii_uppercase().as_str() {
        "BINARY" => DatFormat::Binary16,
        "BINARY32" => DatFormat::Binary32,
        "FLOAT32" => DatFormat::Float32,
        _ => DatFormat::Ascii,
    }
}

/// Parses an IEEE C37.111 .cfg file (revisions 1991/1999/2013, ASCII text).
///
/// Lenient on trailing/optional fields that vary by revision or vendor (skew, min/max,
/// primary/secondary, ps, time_multiplier) — defaults are applied rather than failing
/// the whole file. Strict on fields that affect data correctness (analog `a`/`b` scale
/// factors) since a silently wrong scale factor would violate the "auditable numbers"
/// design goal.
pub fn parse_cfg(text: &str) -> Result<CfgFile, ComtradeError> {
    let mut r = LineReader::new(text);

    // Line 1: station_name, device_id, revision_year (revision_year optional -> 1991)
    let (_, l1) = r
        .next_fields()
        .ok_or(ComtradeError::TruncatedCfg { expected: "station/device/revision line" })?;
    let station_name = l1.first().unwrap_or(&"").to_string();
    let device_id = l1.get(1).unwrap_or(&"").to_string();
    let revision = match l1.get(2).map(|s| s.trim()) {
        Some("1999") => Revision::Y1999,
        Some("2013") => Revision::Y2013,
        _ => Revision::Y1991,
    };

    // Line 2: total_channels, ##A, ##D
    let (line2, l2) = r
        .next_fields()
        .ok_or(ComtradeError::TruncatedCfg { expected: "channel count line" })?;
    let analog_count = l2
        .get(1)
        .and_then(|s| parse_count_with_suffix(s))
        .ok_or(ComtradeError::CfgParseNumber { line: line2, field: "analog channel count" })?;
    let digital_count = l2
        .get(2)
        .and_then(|s| parse_count_with_suffix(s))
        .ok_or(ComtradeError::CfgParseNumber { line: line2, field: "digital channel count" })?;

    let mut analog_channels = Vec::with_capacity(analog_count);
    for _ in 0..analog_count {
        let (line, fields) = r
            .next_fields()
            .ok_or(ComtradeError::TruncatedCfg { expected: "analog channel line" })?;
        analog_channels.push(parse_analog_channel(&fields, line)?);
    }

    let mut digital_channels = Vec::with_capacity(digital_count);
    for _ in 0..digital_count {
        let (line, fields) = r
            .next_fields()
            .ok_or(ComtradeError::TruncatedCfg { expected: "digital channel line" })?;
        digital_channels.push(parse_digital_channel(&fields, line)?);
    }

    // Line frequency — lenient, defaults to 60 Hz if missing/malformed.
    let line_frequency = r
        .next_raw()
        .and_then(|s| s.parse::<f64>().ok())
        .unwrap_or(60.0);

    // nrates, then that many "samp,endsamp" lines. nrates == 0 means "derive timing
    // from the per-sample timestamp field in the .dat file instead" (no rate lines
    // follow) — leave sample_rates empty in that case.
    let (nrates_line, nrates_fields) = r
        .next_fields()
        .ok_or(ComtradeError::TruncatedCfg { expected: "nrates line" })?;
    let nrates: usize = nrates_fields
        .first()
        .and_then(|s| s.parse().ok())
        .ok_or(ComtradeError::CfgParseNumber { line: nrates_line, field: "nrates" })?;

    let mut sample_rates = Vec::with_capacity(nrates);
    for _ in 0..nrates {
        let (line, fields) = r
            .next_fields()
            .ok_or(ComtradeError::TruncatedCfg { expected: "sample rate line" })?;
        let samp_hz = req_f64(&fields, 0, line, "samp")?;
        let end_sample = fields
            .get(1)
            .and_then(|s| s.trim().parse::<u32>().ok())
            .ok_or(ComtradeError::CfgParseNumber { line, field: "endsamp" })?;
        sample_rates.push(SampleRateSegment { samp_hz, end_sample });
    }
    let total_samples = sample_rates.last().map(|s| s.end_sample).unwrap_or(0);

    // Start / trigger timestamps — kept as raw display strings for v1 (see plan: full
    // calendar parsing is low-value since the plot time axis comes from sample rate).
    let timestamp_start_raw = r
        .next_raw()
        .ok_or(ComtradeError::TruncatedCfg { expected: "start timestamp line" })?
        .to_string();
    let timestamp_trigger_raw = r
        .next_raw()
        .ok_or(ComtradeError::TruncatedCfg { expected: "trigger timestamp line" })?
        .to_string();

    // DAT file type.
    let dat_format = r
        .next_raw()
        .map(parse_dat_format)
        .ok_or(ComtradeError::TruncatedCfg { expected: "dat file type line" })?;

    // time_multiplier (1999/2013 only, and even then not always present) — lenient.
    let time_multiplier = if r.peek_is_present() {
        r.next_raw().and_then(|s| s.parse().ok()).unwrap_or(1.0)
    } else {
        1.0
    };
    // Any further 2013-only lines (time_code/local_code, etc.) are intentionally
    // unparsed in v1 — not required for parsing or analysis correctness.

    Ok(CfgFile {
        station_name,
        device_id,
        revision,
        analog_channels,
        digital_channels,
        line_frequency,
        sample_rates,
        total_samples,
        timestamp_start_raw,
        timestamp_trigger_raw,
        dat_format,
        time_multiplier,
    })
}
