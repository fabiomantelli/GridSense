use voltcase_core::comtrade::{self, DatFormat, Revision};

fn fixture(name: &str) -> (String, Vec<u8>) {
    let base = format!("{}/tests/fixtures/ascii/{name}", env!("CARGO_MANIFEST_DIR"));
    let cfg = std::fs::read_to_string(format!("{base}.cfg")).expect("read cfg fixture");
    let dat = std::fs::read(format!("{base}.dat")).expect("read dat fixture");
    (cfg, dat)
}

#[test]
fn parses_1999_revision_with_analog_and_digital_channels() {
    let (cfg_text, dat_bytes) = fixture("synthetic_v1999");
    let record = comtrade::load(&cfg_text, &dat_bytes).expect("parse synthetic v1999 fixture");

    assert_eq!(record.cfg.station_name, "TESTSTATION");
    assert_eq!(record.cfg.device_id, "DEV1");
    assert_eq!(record.cfg.revision, Revision::Y1999);
    assert_eq!(record.cfg.dat_format, DatFormat::Ascii);
    assert_eq!(record.cfg.line_frequency, 60.0);
    assert_eq!(record.cfg.total_samples, 4);

    assert_eq!(record.cfg.analog_channels.len(), 2);
    assert_eq!(record.cfg.digital_channels.len(), 1);

    let va = &record.cfg.analog_channels[0];
    assert_eq!(va.id, "VA");
    assert_eq!(va.phase.as_deref(), Some("A"));
    assert_eq!(va.units, "V");
    assert_eq!(va.a, 1.0);
    assert_eq!(va.b, 0.0);

    let ia = &record.cfg.analog_channels[1];
    assert_eq!(ia.id, "IA");
    assert_eq!(ia.a, 0.5);
    assert_eq!(ia.b, 2.0);
    assert_eq!(ia.ps, 'S');

    assert_eq!(record.cfg.digital_channels[0].id, "52a");
    assert!(record.cfg.digital_channels[0].normal_state);

    // Sample numbers and count.
    assert_eq!(record.sample_numbers, vec![1, 2, 3, 4]);

    // Engineering-scaled analog values: VA = raw*1.0+0.0, IA = raw*0.5+2.0.
    assert_eq!(record.analog_samples[0], vec![100.0, 200.0, 300.0, 400.0]);
    assert_eq!(record.analog_samples[1], vec![7.0, 12.0, 17.0, 22.0]);

    // Digital channel: 1,1,0,0 -> true,true,false,false.
    assert_eq!(
        record.digital_samples[0],
        vec![true, true, false, false]
    );

    // Per-sample raw timestamps are all 0 in the fixture, so timing is derived from
    // the single 1000 Hz sample-rate segment: dt = 1000 us/sample.
    assert_eq!(record.timestamps_us, vec![0.0, 1000.0, 2000.0, 3000.0]);
}

#[test]
fn parses_1991_revision_defaulting_missing_optional_fields() {
    let (cfg_text, dat_bytes) = fixture("synthetic_v1991");
    let record = comtrade::load(&cfg_text, &dat_bytes).expect("parse synthetic v1991 fixture");

    assert_eq!(record.cfg.station_name, "OLDSTATION");
    // No revision-year field on line 1 -> defaults to 1991 per spec.
    assert_eq!(record.cfg.revision, Revision::Y1991);
    // No time_multiplier line present -> defaults to 1.0.
    assert_eq!(record.cfg.time_multiplier, 1.0);
    assert_eq!(record.cfg.line_frequency, 50.0);

    assert_eq!(record.cfg.analog_channels.len(), 1);
    assert_eq!(record.cfg.digital_channels.len(), 0);
    assert!(record.digital_samples.is_empty());

    let vx = &record.cfg.analog_channels[0];
    assert_eq!(vx.a, 2.0);
    assert_eq!(vx.b, 0.0);

    // Engineering-scaled: raw*2.0+0.0.
    assert_eq!(record.analog_samples[0], vec![20.0, 40.0]);

    // 500 Hz segment -> dt = 2000 us/sample.
    assert_eq!(record.timestamps_us, vec![0.0, 2000.0]);
}

/// Builds one binary16 record: sample#(u32 LE), timestamp(u32 LE), analog(i16 LE)*N,
/// then a single u16 LE digital bitfield word (fixture only has 1 digital channel).
fn binary_record(sn: u32, ts: u32, analog_raw: &[i16], digital_bit0: bool) -> Vec<u8> {
    let mut buf = Vec::new();
    buf.extend_from_slice(&sn.to_le_bytes());
    buf.extend_from_slice(&ts.to_le_bytes());
    for &raw in analog_raw {
        buf.extend_from_slice(&raw.to_le_bytes());
    }
    let word: u16 = digital_bit0 as u16;
    buf.extend_from_slice(&word.to_le_bytes());
    buf
}

#[test]
fn parses_binary16_dat_matching_the_equivalent_ascii_fixture() {
    let base = format!(
        "{}/tests/fixtures/binary/synthetic_binary",
        env!("CARGO_MANIFEST_DIR")
    );
    let cfg_text = std::fs::read_to_string(format!("{base}.cfg")).expect("read cfg fixture");

    // Same underlying VA/IA/52a values as the synthetic_v1999 ASCII fixture, so the
    // scaled/derived results below should match that test exactly.
    let mut dat_bytes = Vec::new();
    dat_bytes.extend(binary_record(1, 0, &[100, 10], true));
    dat_bytes.extend(binary_record(2, 0, &[200, 20], true));
    dat_bytes.extend(binary_record(3, 0, &[300, 30], false));
    dat_bytes.extend(binary_record(4, 0, &[400, 40], false));

    let record = comtrade::load(&cfg_text, &dat_bytes).expect("parse synthetic binary16 fixture");

    assert_eq!(record.cfg.dat_format, DatFormat::Binary16);
    assert_eq!(record.sample_numbers, vec![1, 2, 3, 4]);
    assert_eq!(record.analog_samples[0], vec![100.0, 200.0, 300.0, 400.0]);
    assert_eq!(record.analog_samples[1], vec![7.0, 12.0, 17.0, 22.0]);
    assert_eq!(record.digital_samples[0], vec![true, true, false, false]);
    assert_eq!(record.timestamps_us, vec![0.0, 1000.0, 2000.0, 3000.0]);
}

#[test]
fn truncated_binary_dat_is_reported_not_panicked() {
    let base = format!(
        "{}/tests/fixtures/binary/synthetic_binary",
        env!("CARGO_MANIFEST_DIR")
    );
    let cfg_text = std::fs::read_to_string(format!("{base}.cfg")).expect("read cfg fixture");

    // One full record (14 bytes) plus 3 stray bytes that don't complete another record.
    let mut dat_bytes = binary_record(1, 0, &[100, 10], true);
    dat_bytes.extend_from_slice(&[0, 0, 0]);

    let err = comtrade::load(&cfg_text, &dat_bytes).unwrap_err();
    assert!(matches!(err, comtrade::ComtradeError::TruncatedBinaryDat { .. }));
}

#[test]
fn nrates_zero_still_consumes_one_rate_line() {
    // Regression test: real-world relay exports (confirmed against an actual
    // PRSTF6-style substation file) set nrates=0 but still emit exactly one
    // "samp,endsamp" line (samp=0, endsamp=total_samples). Previously we treated
    // nrates==0 as "zero lines follow," which desynced every line after it and
    // caused an actually-BINARY file to be misread as ASCII (and then fail UTF-8
    // decoding on the real binary .dat bytes).
    let (cfg_text, dat_bytes) = fixture("synthetic_v1999_nrates0");
    let record = comtrade::load(&cfg_text, &dat_bytes).expect("parse nrates=0 fixture");

    assert_eq!(record.cfg.dat_format, DatFormat::Ascii);
    assert_eq!(record.cfg.time_multiplier, 1.0);
    assert_eq!(record.cfg.sample_rates.len(), 1);
    assert_eq!(record.cfg.sample_rates[0].samp_hz, 0.0);
    assert_eq!(record.cfg.total_samples, 4);

    // samp_hz == 0 means "no fixed rate" — timing must come from the per-sample
    // timestamp field in the .dat file instead (here: 0, 333, 667, 1000 us).
    assert_eq!(record.timestamps_us, vec![0.0, 333.0, 667.0, 1000.0]);
    assert_eq!(record.analog_samples[0], vec![100.0, 200.0, 300.0, 400.0]);
}

#[test]
fn parses_multi_channel_fault_event_fixture() {
    // 6 analog + 1 digital, 600 samples at 3 kHz — a synthetic phase-B fault used for
    // manual browser verification of the chart/timeline UI and, later, as an M4
    // event-detection fixture. Here we just check it loads cleanly with the right
    // shape; step-change/classification assertions belong to the M4 analysis tests.
    let (cfg_text, dat_bytes) = fixture("fault_event_v1999");
    let record = comtrade::load(&cfg_text, &dat_bytes).expect("parse fault_event fixture");

    assert_eq!(record.cfg.analog_channels.len(), 6);
    assert_eq!(record.cfg.digital_channels.len(), 1);
    assert_eq!(record.sample_numbers.len(), 600);
    assert_eq!(record.analog_samples[0].len(), 600);
    assert_eq!(record.digital_samples[0].len(), 600);

    // Breaker (52a) starts closed and opens at array index 354 (t = 118 ms), staying open.
    assert!(record.digital_samples[0][0]);
    assert!(record.digital_samples[0][353]);
    assert!(!record.digital_samples[0][354]);
    assert!(!record.digital_samples[0][599]);

    // End-to-end wiring check (not just the parser in isolation, see
    // comtrade_timestamp_tests.rs): cfg.rs actually threads timestamp_start_raw
    // ("01/01/2026,00:00:00.000000") through to start_epoch_us.
    assert_eq!(record.cfg.start_epoch_us, Some(1_767_225_600_000_000.0));
}

#[test]
fn malformed_dat_line_reports_the_offending_line_number() {
    let (cfg_text, _) = fixture("synthetic_v1999");
    // Missing the digital field on line 2.
    let bad_dat = "1,0,100,10,1\n2,0,200,20\n";

    let err = comtrade::load(&cfg_text, bad_dat.as_bytes()).unwrap_err();
    match err {
        comtrade::ComtradeError::MalformedDatLine { line, found, .. } => {
            assert_eq!(line, 2);
            assert_eq!(found, 4);
        }
        other => panic!("expected MalformedDatLine, got {other:?}"),
    }
}
