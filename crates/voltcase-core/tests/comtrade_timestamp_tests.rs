use voltcase_core::comtrade::timestamp::parse_comtrade_timestamp;

#[test]
fn parses_known_reference_dates() {
    // 2026-01-01T00:00:00Z: 20454 days since epoch (cross-checked independently via
    // 2000-01-01 = 10957 days + 26 years incl. 7 leap days = 9497 -> 20454).
    assert_eq!(
        parse_comtrade_timestamp("01/01/2026,00:00:00.000000"),
        Some(20_454.0 * 86_400_000_000.0)
    );
    // 2020-01-01T00:00:00Z: 18262 days since epoch.
    assert_eq!(
        parse_comtrade_timestamp("01/01/2020,00:00:00.000000"),
        Some(18_262.0 * 86_400_000_000.0)
    );
}

#[test]
fn combines_date_time_and_fraction_correctly() {
    // 16/04/2026 07:02:14.098071 -> date offset + h/m/s/frac all additive.
    let days = 20_559.0; // 2026-04-16, cross-checked against the 2026-01-01 anchor (31+28+31+15 = 105 days later)
    let expected = days * 86_400_000_000.0
        + 7.0 * 3_600_000_000.0
        + 2.0 * 60_000_000.0
        + 14.0 * 1_000_000.0
        + 98_071.0;
    assert_eq!(parse_comtrade_timestamp("16/04/2026,07:02:14.098071"), Some(expected));
}

#[test]
fn pads_millisecond_fractions_to_microseconds() {
    // Some vendors emit 3-digit ms instead of the spec's 6-digit us.
    let a = parse_comtrade_timestamp("01/01/2026,00:00:00.123").unwrap();
    let b = parse_comtrade_timestamp("01/01/2026,00:00:00.123000").unwrap();
    assert_eq!(a, b);
}

#[test]
fn truncates_over_long_fractions() {
    let a = parse_comtrade_timestamp("01/01/2026,00:00:00.1234567").unwrap();
    let b = parse_comtrade_timestamp("01/01/2026,00:00:00.123456").unwrap();
    assert_eq!(a, b);
}

#[test]
fn handles_missing_fraction() {
    let a = parse_comtrade_timestamp("01/01/2026,00:00:00").unwrap();
    let b = parse_comtrade_timestamp("01/01/2026,00:00:00.000000").unwrap();
    assert_eq!(a, b);
}

#[test]
fn leap_year_feb_29_is_valid_only_on_leap_years() {
    assert!(parse_comtrade_timestamp("29/02/2024,12:00:00.000000").is_some());
    assert!(parse_comtrade_timestamp("29/02/2023,12:00:00.000000").is_none());
}

#[test]
fn rejects_malformed_input() {
    assert!(parse_comtrade_timestamp("").is_none());
    assert!(parse_comtrade_timestamp("not-a-timestamp").is_none());
    assert!(parse_comtrade_timestamp("31/13/2026,00:00:00.000000").is_none()); // month 13
    assert!(parse_comtrade_timestamp("32/01/2026,00:00:00.000000").is_none()); // day 32
    assert!(parse_comtrade_timestamp("01/01/2026 00:00:00.000000").is_none()); // missing comma
    assert!(parse_comtrade_timestamp("01/01/26,00:00:00.000000").is_none()); // 2-digit year
    assert!(parse_comtrade_timestamp("01/01/2026,25:00:00.000000").is_none()); // hour 25
    assert!(parse_comtrade_timestamp("01/01/2026,00:60:00.000000").is_none()); // minute 60
    assert!(parse_comtrade_timestamp("01/01/2026,00:00:60.000000").is_none()); // second 60
}
