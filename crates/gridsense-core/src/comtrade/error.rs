use thiserror::Error;

use super::model::DatFormat;

#[derive(Debug, Error)]
pub enum ComtradeError {
    #[error("cfg file ended unexpectedly; expected a line for '{expected}'")]
    TruncatedCfg { expected: &'static str },

    #[error("cfg line {line}: could not parse '{field}' as a number")]
    CfgParseNumber { line: usize, field: &'static str },

    #[error("cfg line {line}: analog channel record has too few fields ({found}, need at least 7)")]
    MalformedAnalogChannel { line: usize, found: usize },

    #[error("cfg line {line}: digital channel record has too few fields ({found}, need at least 2)")]
    MalformedDigitalChannel { line: usize, found: usize },

    #[error(
        "dat line {line}: expected {expected} fields (sample#, timestamp, {analog} analog, {digital} digital), found {found}"
    )]
    MalformedDatLine {
        line: usize,
        expected: usize,
        analog: usize,
        digital: usize,
        found: usize,
    },

    #[error("dat line {line}: could not parse '{field}' as a number")]
    DatParseNumber { line: usize, field: String },

    #[error("binary dat file too short: expected at least {expected} bytes for {samples} samples, found {found}")]
    TruncatedBinaryDat {
        expected: usize,
        found: usize,
        samples: usize,
    },

    #[error("ascii dat file is not valid utf-8")]
    InvalidUtf8,

    #[error("dat format {0:?} is not supported yet")]
    UnsupportedDatFormat(DatFormat),
}
