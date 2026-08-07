use std::ops::{Add, Mul, Sub};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Complex64 {
    pub re: f64,
    pub im: f64,
}

impl Complex64 {
    pub const ZERO: Complex64 = Complex64 { re: 0.0, im: 0.0 };

    pub fn new(re: f64, im: f64) -> Self {
        Self { re, im }
    }

    pub fn magnitude(&self) -> f64 {
        self.re.hypot(self.im)
    }

    pub fn angle_rad(&self) -> f64 {
        self.im.atan2(self.re)
    }

    pub fn scale(&self, k: f64) -> Complex64 {
        Complex64::new(self.re * k, self.im * k)
    }
}

impl Add for Complex64 {
    type Output = Complex64;
    fn add(self, rhs: Complex64) -> Complex64 {
        Complex64::new(self.re + rhs.re, self.im + rhs.im)
    }
}

impl Sub for Complex64 {
    type Output = Complex64;
    fn sub(self, rhs: Complex64) -> Complex64 {
        Complex64::new(self.re - rhs.re, self.im - rhs.im)
    }
}

impl Mul for Complex64 {
    type Output = Complex64;
    fn mul(self, rhs: Complex64) -> Complex64 {
        Complex64::new(
            self.re * rhs.re - self.im * rhs.im,
            self.re * rhs.im + self.im * rhs.re,
        )
    }
}

/// Extracts the fundamental-frequency phasor of one cycle starting at `cycle_start`,
/// via a single-frequency (Goertzel-equivalent) DFT rather than a full FFT — cheap,
/// and all the analysis engine needs is the power-frequency component.
///
/// Returns a peak-amplitude phasor (not RMS) in the same engineering units as
/// `samples`, since COMTRADE analog channels are instantaneous waveform samples.
/// Divide `magnitude()` by sqrt(2) to get an RMS-equivalent value.
pub fn extract_phasor(samples: &[f32], fs: f64, f0: f64, cycle_start: usize) -> Option<Complex64> {
    if fs <= 0.0 || f0 <= 0.0 {
        return None;
    }
    let n = (fs / f0).round() as usize;
    if n == 0 || cycle_start + n > samples.len() {
        return None;
    }

    let mut re = 0.0f64;
    let mut im = 0.0f64;
    for k in 0..n {
        let angle = 2.0 * std::f64::consts::PI * f0 * (k as f64) / fs;
        let x = samples[cycle_start + k] as f64;
        re += x * angle.cos();
        im -= x * angle.sin();
    }
    let scale = 2.0 / n as f64;
    Some(Complex64::new(re * scale, im * scale))
}
