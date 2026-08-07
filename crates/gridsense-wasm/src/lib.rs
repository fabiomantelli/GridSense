use gridsense_core::comtrade::{self, ComtradeRecord};
use gridsense_core::analysis;
use serde::Serialize;
use wasm_bindgen::prelude::*;

fn js_error(msg: impl std::fmt::Display) -> JsValue {
    JsValue::from(js_sys::Error::new(&msg.to_string()))
}

/// serde-wasm-bindgen's default serializer turns Rust `None` into JS `undefined`, not
/// `null` — easy to miss since every hand-written `Option<T>` field here reads `null`
/// on the TS/Svelte side (`x !== null` guards, `T | null` types), and `undefined`
/// silently slips past those checks straight into a crash (e.g. `.toFixed()` on
/// `undefined`). Route every struct we hand to JS through this serializer instead of
/// `serde_wasm_bindgen::to_value` so `None` consistently becomes `null`.
fn to_js<T: Serialize + ?Sized>(value: &T) -> Result<JsValue, JsValue> {
    let serializer = serde_wasm_bindgen::Serializer::new().serialize_missing_as_null(true);
    value.serialize(&serializer).map_err(js_error)
}

#[wasm_bindgen]
pub struct ComtradeHandle {
    record: ComtradeRecord,
}

/// Parses a COMTRADE record from the raw bytes of its .cfg and .dat files. The .cfg
/// bytes are decoded as UTF-8 text (ASCII 1991/1999 files are a strict subset); the
/// .dat bytes are handed through unchanged — the parser decides ASCII vs. binary based
/// on what the already-parsed .cfg declares.
#[wasm_bindgen]
pub fn parse_comtrade(cfg_bytes: &[u8], dat_bytes: &[u8]) -> Result<ComtradeHandle, JsValue> {
    console_error_panic_hook::set_once();
    let cfg_text = std::str::from_utf8(cfg_bytes).map_err(|_| js_error("cfg file is not valid UTF-8"))?;
    let record = comtrade::load(cfg_text, dat_bytes).map_err(js_error)?;
    Ok(ComtradeHandle { record })
}

#[wasm_bindgen]
impl ComtradeHandle {
    /// Station/device/revision + full channel definitions (id, phase, units, scale
    /// factors, sample-rate segments) as a plain JS object.
    #[wasm_bindgen]
    pub fn metadata(&self) -> Result<JsValue, JsValue> {
        to_js(&self.record.cfg)
    }

    pub fn sample_count(&self) -> usize {
        self.record.sample_numbers.len()
    }

    /// Scaled (engineering-unit) samples for one analog channel, as a single bulk copy
    /// into a typed array — not a zero-copy view, since the WASM heap can move/grow
    /// across the lifetime of this handle.
    pub fn analog_channel_f32(&self, index: usize) -> Result<js_sys::Float32Array, JsValue> {
        let samples = self
            .record
            .analog_samples
            .get(index)
            .ok_or_else(|| js_error("analog channel index out of range"))?;
        Ok(js_sys::Float32Array::from(samples.as_slice()))
    }

    /// Digital channel samples as 0/1 bytes (one per sample).
    pub fn digital_channel_bools(&self, index: usize) -> Result<js_sys::Uint8Array, JsValue> {
        let samples = self
            .record
            .digital_samples
            .get(index)
            .ok_or_else(|| js_error("digital channel index out of range"))?;
        let bytes: Vec<u8> = samples.iter().map(|&b| b as u8).collect();
        Ok(js_sys::Uint8Array::from(bytes.as_slice()))
    }

    /// Relative microseconds from the first sample, shared across all channels.
    pub fn timestamps_f64(&self) -> js_sys::Float64Array {
        js_sys::Float64Array::from(self.record.timestamps_us.as_slice())
    }

    /// Runs the deterministic analysis engine (RMS, phasors, symmetrical components,
    /// event detection/classification) and returns the resulting `AnalysisFacts` as a
    /// plain JS object. No raw sample arrays are included — only computed facts.
    pub fn run_analysis(&self) -> Result<JsValue, JsValue> {
        let facts = analysis::analyze(&self.record);
        to_js(&facts)
    }
}
