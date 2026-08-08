pub mod cfg;
pub mod dat;
pub mod error;
pub mod model;
pub mod timestamp;

pub use error::ComtradeError;
pub use model::{
    AnalogChannelDef, CfgFile, ComtradeRecord, DatFormat, DigitalChannelDef, Revision,
    SampleRateSegment,
};

/// Parses a COMTRADE record from its .cfg text and .dat bytes, dispatching to the
/// right .dat decoder based on what the .cfg file declares.
pub fn load(cfg_text: &str, dat_bytes: &[u8]) -> Result<ComtradeRecord, ComtradeError> {
    let cfg = cfg::parse_cfg(cfg_text)?;

    let sample_data = match cfg.dat_format {
        DatFormat::Ascii => {
            let text = std::str::from_utf8(dat_bytes).map_err(|_| ComtradeError::InvalidUtf8)?;
            dat::parse_dat_ascii(text, &cfg)?
        }
        DatFormat::Binary16 => dat::parse_dat_binary16(dat_bytes, &cfg)?,
        other => return Err(ComtradeError::UnsupportedDatFormat(other)),
    };

    Ok(ComtradeRecord {
        cfg,
        sample_numbers: sample_data.sample_numbers,
        timestamps_us: sample_data.timestamps_us,
        analog_samples: sample_data.analog_samples,
        digital_samples: sample_data.digital_samples,
    })
}
