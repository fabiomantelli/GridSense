use gridsense_core::analysis::event_correlation::{classify_events, FaultKind, Phase, DEFAULT_STEP_THRESHOLD_PCT};
use gridsense_core::analysis::event_detection::{detect_step_changes, StepDirection};
use gridsense_core::analysis::phasor::{extract_phasor, Complex64};
use gridsense_core::analysis::rms::{overall_rms, windowed_rms};
use gridsense_core::analysis::symmetrical_components::sequence_components;
use gridsense_core::analysis::{self};
use gridsense_core::comtrade;

const SQRT2: f64 = std::f64::consts::SQRT_2;

fn sine_wave(amplitude: f64, freq: f64, fs: f64, phase_rad: f64, n_samples: usize) -> Vec<f32> {
    (0..n_samples)
        .map(|n| {
            let t = n as f64 / fs;
            (amplitude * (2.0 * std::f64::consts::PI * freq * t - phase_rad).cos()) as f32
        })
        .collect()
}

#[test]
fn windowed_rms_of_pure_sine_converges_to_closed_form() {
    // 100 samples/cycle at 60 Hz -> no rounding in cycle_len, clean closed-form check.
    let fs: f64 = 6000.0;
    let f0: f64 = 60.0;
    let amplitude = 100.0;
    let cycle_len = (fs / f0).round() as usize; // 100
    let samples = sine_wave(amplitude, f0, fs, 0.0, cycle_len * 3);

    let rms = windowed_rms(&samples, cycle_len);
    let expected = amplitude / SQRT2;

    // After the first full cycle has filled the window, RMS should sit within 0.1% of
    // the closed-form A/sqrt(2) for every subsequent sample.
    for &v in &rms[cycle_len..] {
        assert!(
            ((v as f64) - expected).abs() < expected * 0.001,
            "expected ~{expected}, got {v}"
        );
    }

    let overall = overall_rms(&samples);
    assert!(((overall as f64) - expected).abs() < expected * 0.001);
}

#[test]
fn extract_phasor_recovers_known_amplitude_and_phase() {
    let fs: f64 = 6000.0;
    let f0: f64 = 60.0;
    let amplitude = 100.0;
    let cycle_len = (fs / f0).round() as usize;
    let phase = 30.0f64.to_radians();

    let samples = sine_wave(amplitude, f0, fs, phase, cycle_len);
    let phasor = extract_phasor(&samples, fs, f0, 0).expect("phasor");

    assert!((phasor.magnitude() - amplitude).abs() < amplitude * 0.001);
    // x[n] = A cos(wt - phase) -> our DFT-at-fundamental convention yields angle = -phase.
    assert!((phasor.angle_rad() - (-phase)).abs() < 0.01);
}

fn polar(magnitude: f64, angle_deg: f64) -> Complex64 {
    let a = angle_deg.to_radians();
    Complex64::new(magnitude * a.cos(), magnitude * a.sin())
}

#[test]
fn sequence_components_of_balanced_positive_sequence() {
    let va = polar(100.0, 0.0);
    let vb = polar(100.0, -120.0);
    let vc = polar(100.0, 120.0);

    let seq = sequence_components(va, vb, vc);
    assert!(seq.zero.magnitude() < 1e-6);
    assert!(seq.negative.magnitude() < 1e-6);
    assert!((seq.positive.magnitude() - 100.0).abs() < 1e-6);
}

#[test]
fn sequence_components_of_single_energized_phase() {
    // Classic textbook case: only Va energized, Vb = Vc = 0 -> V0 = V1 = V2 = Va/3.
    let va = polar(90.0, 0.0);
    let vb = Complex64::ZERO;
    let vc = Complex64::ZERO;

    let seq = sequence_components(va, vb, vc);
    let expected = 30.0;
    assert!((seq.zero.magnitude() - expected).abs() < 1e-6);
    assert!((seq.positive.magnitude() - expected).abs() < 1e-6);
    assert!((seq.negative.magnitude() - expected).abs() < 1e-6);
}

#[test]
fn detect_step_changes_flags_only_the_onset() {
    let cycle_len = 50;
    let mut rms_series = vec![100.0f32; 500];
    for v in rms_series.iter_mut().skip(300) {
        *v = 150.0;
    }

    let events = detect_step_changes(&rms_series, cycle_len, 20.0);

    assert_eq!(events.len(), 1, "expected exactly one onset event, got {events:?}");
    let e = &events[0];
    assert_eq!(e.sample_index, 300);
    assert_eq!(e.direction, StepDirection::Rise);
    assert!((e.baseline - 100.0).abs() < 1e-6);
    assert!((e.value - 150.0).abs() < 1e-6);
    assert!((e.change_pct - 50.0).abs() < 1e-3);
}

#[test]
fn detect_step_changes_ignores_deviations_below_threshold() {
    let cycle_len = 50;
    let mut rms_series = vec![100.0f32; 200];
    for v in rms_series.iter_mut().skip(100) {
        *v = 105.0; // 5% change, below a 20% threshold
    }
    let events = detect_step_changes(&rms_series, cycle_len, 20.0);
    assert!(events.is_empty());
}

fn load_fault_fixture() -> comtrade::ComtradeRecord {
    let base = format!("{}/tests/fixtures/ascii/fault_event_v1999", env!("CARGO_MANIFEST_DIR"));
    let cfg = std::fs::read_to_string(format!("{base}.cfg")).unwrap();
    let dat = std::fs::read(format!("{base}.dat")).unwrap();
    comtrade::load(&cfg, &dat).expect("parse fault_event fixture")
}

#[test]
fn classify_events_identifies_phase_b_ground_fault_and_breaker_trip() {
    let record = load_fault_fixture();

    let events = classify_events(
        &record.cfg,
        &record.analog_samples,
        &record.digital_samples,
        &record.timestamps_us,
        DEFAULT_STEP_THRESHOLD_PCT,
    );

    assert_eq!(events.len(), 1, "expected exactly one correlated event, got {events:?}");
    let e = &events[0];

    match &e.kind {
        FaultKind::PhaseToGround(Phase::B) => {}
        other => panic!("expected PhaseToGround(B), got {other:?}"),
    }

    // True fault inception is at sample 240 (t = 80 ms); detection necessarily lags by
    // less than one cycle (50 samples) since it needs the RMS window to cross threshold.
    assert!(e.onset_sample >= 240 && e.onset_sample < 240 + 50, "onset_sample = {}", e.onset_sample);

    // Generator used a 4.7x peak-amplitude step on IB; RMS scales the same way for a
    // like-shaped waveform, so the settled (one full cycle post-onset) ratio should
    // land close to 4.7x.
    let current_multiple = e.current_multiple.expect("current_multiple");
    assert!((4.0..=5.5).contains(&current_multiple), "current_multiple = {current_multiple}");

    assert_eq!(e.breaker_channel_id.as_deref(), Some("52a"));
    let time_to_trip_ms = e.time_to_trip_us.expect("time_to_trip_us") / 1000.0;
    // True trip is 38 ms after true fault inception; measured from the (slightly
    // later) detected onset it must be positive and no more than 38 ms.
    assert!(time_to_trip_ms > 0.0 && time_to_trip_ms <= 38.0, "time_to_trip_ms = {time_to_trip_ms}");
}

#[test]
fn classify_events_leaves_unmatched_optional_fields_none() {
    // Voltage-only sag: no matching current rise, no breaker deviation. Regression
    // guard for the gridsense-wasm serialization boundary — these `None`s must
    // reach JS as `null`, not `undefined` (see gridsense-wasm/src/lib.rs's `to_js`),
    // or the UI's `!= null` guards silently let a `.toFixed()` call crash.
    let base = format!("{}/tests/fixtures/ascii/voltage_only_v1999", env!("CARGO_MANIFEST_DIR"));
    let cfg = std::fs::read_to_string(format!("{base}.cfg")).unwrap();
    let dat = std::fs::read(format!("{base}.dat")).unwrap();
    let record = comtrade::load(&cfg, &dat).unwrap();

    let events = classify_events(
        &record.cfg,
        &record.analog_samples,
        &record.digital_samples,
        &record.timestamps_us,
        DEFAULT_STEP_THRESHOLD_PCT,
    );

    assert_eq!(events.len(), 1, "expected exactly one event, got {events:?}");
    let e = &events[0];
    assert!(matches!(e.kind, FaultKind::Unclassified), "expected Unclassified, got {:?}", e.kind);
    assert!(e.current_multiple.is_none());
    assert!(e.breaker_channel_id.is_none());
    assert!(e.time_to_trip_us.is_none());
}

#[test]
fn analyze_produces_facts_with_no_events_on_a_clean_record() {
    let base = format!("{}/tests/fixtures/ascii/synthetic_v1999", env!("CARGO_MANIFEST_DIR"));
    let cfg = std::fs::read_to_string(format!("{base}.cfg")).unwrap();
    let dat = std::fs::read(format!("{base}.dat")).unwrap();
    let record = comtrade::load(&cfg, &dat).unwrap();

    let facts = analysis::analyze(&record);
    assert_eq!(facts.record_summary.station_name, "TESTSTATION");
    assert_eq!(facts.channel_summaries.len(), 2);
    assert!(facts.events.is_empty());
}
