use super::phasor::Complex64;

/// The complex operator `a = 1∠120°`, used to build the symmetrical-component
/// transform.
const A: Complex64 = Complex64 { re: -0.5, im: 0.8660254037844387 };
const A2: Complex64 = Complex64 { re: -0.5, im: -0.8660254037844387 };

pub struct SequenceComponents {
    pub zero: Complex64,
    pub positive: Complex64,
    pub negative: Complex64,
}

/// Fortescue transform: decomposes three phase phasors (in A/B/C order) into
/// zero/positive/negative sequence components. A nonzero zero-sequence relative to
/// positive-sequence is the standard indicator of a ground-involved fault.
pub fn sequence_components(va: Complex64, vb: Complex64, vc: Complex64) -> SequenceComponents {
    let third = 1.0 / 3.0;
    SequenceComponents {
        zero: (va + vb + vc).scale(third),
        positive: (va + A * vb + A2 * vc).scale(third),
        negative: (va + A2 * vb + A * vc).scale(third),
    }
}
