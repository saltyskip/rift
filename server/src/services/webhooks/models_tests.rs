use super::*;
use crate::core::public_id::AffiliateId;

#[test]
fn empty_filters_match_every_event() {
    let f = WebhookFilters::default();
    assert!(f.is_empty());
    // An unfiltered webhook receives events regardless of context.
    assert!(f.matches(&EventMatchContext::default()));
    assert!(f.matches(&EventMatchContext {
        affiliate_id: Some(AffiliateId::new()),
    }));
}

#[test]
fn affiliate_filter_matches_only_same_affiliate() {
    let a = AffiliateId::new();
    let b = AffiliateId::new();
    let f = WebhookFilters {
        affiliate_id: Some(a),
    };
    assert!(!f.is_empty());

    // Same affiliate credited → deliver.
    assert!(f.matches(&EventMatchContext {
        affiliate_id: Some(a),
    }));
    // A different affiliate → drop (no cross-affiliate leakage).
    assert!(!f.matches(&EventMatchContext {
        affiliate_id: Some(b),
    }));
    // No affiliate context (organic last-touch, or a non-conversion event
    // that carries no affiliate) → an affiliate-filtered webhook drops it.
    assert!(!f.matches(&EventMatchContext { affiliate_id: None }));
    assert!(!f.matches(&EventMatchContext::default()));
}

#[test]
fn empty_filters_omit_from_json_and_default_on_absence() {
    // Empty filters serialize to `{}` (no keys) so storage / responses
    // stay clean for the common unfiltered case.
    let empty = WebhookFilters::default();
    assert_eq!(serde_json::to_value(&empty).unwrap(), serde_json::json!({}));

    // An absent `filters` object deserializes to empty via serde default.
    let parsed: WebhookFilters = serde_json::from_value(serde_json::json!({})).unwrap();
    assert!(parsed.is_empty());
}

#[test]
fn affiliate_filter_round_trips_through_json() {
    let a = AffiliateId::new();
    let f = WebhookFilters {
        affiliate_id: Some(a),
    };

    let v = serde_json::to_value(&f).unwrap();
    // Serialized as the public `aff_…` wire form.
    assert!(v["affiliate_id"].as_str().unwrap().starts_with("aff_"));

    let back: WebhookFilters = serde_json::from_value(v).unwrap();
    assert_eq!(back, f);
}
