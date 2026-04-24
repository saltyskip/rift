use super::format_date_ymd;

#[test]
fn formats_unix_epoch() {
    assert_eq!(format_date_ymd(0), "1970-01-01");
}

#[test]
fn formats_known_date() {
    // 2021-01-01 00:00:00 UTC — pinned to a value I can verify by hand.
    assert_eq!(format_date_ymd(1_609_459_200_000), "2021-01-01");
}

#[test]
fn negative_clamps_to_epoch() {
    assert_eq!(format_date_ymd(-1), "1970-01-01");
}
