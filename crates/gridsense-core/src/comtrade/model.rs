use serde::Serialize;

/// IEEE C37.111 revision the .cfg file declares itself as. Affects which optional
/// trailing fields (time_multiplier, Binary32/Float32 dat formats) are expected.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum Revision {
    Y1991,
    Y1999,
    Y2013,
}

/// Data encoding of the paired .dat file. Only Ascii and Binary16 are decoded in v1;
/// Binary32/Float32 are recognized (2013 revision) so the enum doesn't need to change
/// shape when that decoder is added.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum DatFormat {
    Ascii,
    Binary16,
    Binary32,
    Float32,
}

#[derive(Debug, Clone, Serialize)]
pub struct AnalogChannelDef {
    pub index: u32,
    pub id: String,
    pub phase: Option<String>,
    pub circuit_component: Option<String>,
    pub units: String,
    /// Engineering value = raw * a + b.
    pub a: f64,
    pub b: f64,
    pub skew: f64,
    pub min: f64,
    pub max: f64,
    pub primary: f64,
    pub secondary: f64,
    pub ps: char,
}

#[derive(Debug, Clone, Serialize)]
pub struct DigitalChannelDef {
    pub index: u32,
    pub id: String,
    pub phase: Option<String>,
    pub circuit_component: Option<String>,
    pub normal_state: bool,
}

#[derive(Debug, Clone, Copy, Serialize)]
pub struct SampleRateSegment {
    pub samp_hz: f64,
    /// Last sample number (1-based, per spec) covered by this rate.
    pub end_sample: u32,
}

#[derive(Debug, Clone, Serialize)]
pub struct CfgFile {
    pub station_name: String,
    pub device_id: String,
    pub revision: Revision,
    pub analog_channels: Vec<AnalogChannelDef>,
    pub digital_channels: Vec<DigitalChannelDef>,
    pub line_frequency: f64,
    pub sample_rates: Vec<SampleRateSegment>,
    /// Derived from the last sample-rate segment's end_sample; 0 if unknown until the
    /// .dat file is parsed.
    pub total_samples: u32,
    pub timestamp_start_raw: String,
    pub timestamp_trigger_raw: String,
    pub dat_format: DatFormat,
    pub time_multiplier: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct ComtradeRecord {
    pub cfg: CfgFile,
    pub sample_numbers: Vec<u32>,
    /// Relative microseconds from the first sample.
    pub timestamps_us: Vec<f64>,
    /// Outer index: channel (same order as cfg.analog_channels), already scaled by a/b.
    pub analog_samples: Vec<Vec<f32>>,
    /// Outer index: channel (same order as cfg.digital_channels).
    pub digital_samples: Vec<Vec<bool>>,
}
