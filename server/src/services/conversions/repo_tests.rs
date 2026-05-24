//! Regression tests for [`count_field_as_u64`].
//!
//! The previous implementation called `Document::get_i64("count")` to
//! read the result of a `$group: { count: { $sum: 1 } }` stage. That
//! returns `Err` when the BSON value is `Int32` — which Mongo emits
//! for every count that fits in 32 bits, i.e. every realistic value.
//! Combined with `.unwrap_or(0)` and a `count > 0` guard, every
//! conversion in every funnel result was silently dropped.
//!
//! Caught in production when post-cutover smoke tests reported
//! `conversions: {}` on the `/v1/analytics/stats` response even with
//! a known-good conversion in the database and a correctly working
//! `$lookup` upstream of the `$group`.

use super::count_field_as_u64;
use mongodb::bson::{doc, Bson};

#[test]
fn reads_int32_count_correctly() {
    // The exact shape Mongo emits for `$sum: 1` aggregations with
    // small totals — this is what was silently dropped by the old
    // `get_i64`-then-unwrap_or(0) pattern.
    let raw = doc! { "_id": "signup", "count": Bson::Int32(1) };
    assert_eq!(count_field_as_u64(&raw, "count"), 1);
}

#[test]
fn reads_int64_count_correctly() {
    let raw = doc! { "_id": "signup", "count": Bson::Int64(42) };
    assert_eq!(count_field_as_u64(&raw, "count"), 42);
}

#[test]
fn reads_double_count_correctly() {
    // `$sum` of mixed-type aggregations or `$count` with promotion
    // can produce Double — accept it rather than silently dropping.
    let raw = doc! { "_id": "signup", "count": Bson::Double(7.0) };
    assert_eq!(count_field_as_u64(&raw, "count"), 7);
}

#[test]
fn clamps_negative_int_to_zero() {
    // Defensive: a future caller doing `$sum` over signed values
    // shouldn't be able to produce a panic via `as u64`.
    let raw = doc! { "_id": "signup", "count": Bson::Int32(-5) };
    assert_eq!(count_field_as_u64(&raw, "count"), 0);
    let raw = doc! { "_id": "signup", "count": Bson::Int64(-5) };
    assert_eq!(count_field_as_u64(&raw, "count"), 0);
    let raw = doc! { "_id": "signup", "count": Bson::Double(-5.0) };
    assert_eq!(count_field_as_u64(&raw, "count"), 0);
}

#[test]
fn missing_field_yields_zero() {
    let raw = doc! { "_id": "signup" };
    assert_eq!(count_field_as_u64(&raw, "count"), 0);
}

#[test]
fn wrong_type_yields_zero_rather_than_panicking() {
    // A future schema mistake (count emitted as a string from a
    // typo'd pipeline stage) should not crash the funnel endpoint —
    // just zero out that group.
    let raw = doc! { "_id": "signup", "count": "five" };
    assert_eq!(count_field_as_u64(&raw, "count"), 0);
}
