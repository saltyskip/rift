use super::*;
use crate::services::landing::models::ColorScheme;

#[test]
fn parses_short_and_long_hex() {
    assert_eq!(parse_hex("#fff"), Some((255, 255, 255)));
    assert_eq!(parse_hex("#000000"), Some((0, 0, 0)));
    assert_eq!(parse_hex("  #0d9488 "), Some((13, 148, 136)));
    assert_eq!(parse_hex("#abc"), Some((170, 187, 204)));
}

#[test]
fn rejects_malformed_hex() {
    assert_eq!(parse_hex("0d9488"), None); // missing '#'
    assert_eq!(parse_hex("#12"), None); // wrong length
    assert_eq!(parse_hex("#gggggg"), None); // non-hex digits
    assert_eq!(parse_hex("rebeccapurple"), None); // named color
}

#[test]
fn garbage_color_falls_back_to_default_accent() {
    // A garbage value must never break rendering — it resolves to Rift teal.
    let garbage = derive_palette("not-a-color", ColorScheme::Dark);
    let teal = derive_palette("#0d9488", ColorScheme::Dark);
    assert_eq!(garbage.root.accent, teal.root.accent);
}

#[test]
fn accent_is_clamped_into_tasteful_band() {
    // Pure neon green: saturation 1.0, lightness 0.5 → saturation must be
    // pulled below the 0.90 ceiling, lightness stays within [0.40, 0.65].
    let p = derive_palette("#00ff00", ColorScheme::Dark);
    let rgb = parse_hex(&p.root.accent).expect("accent is valid hex");
    let (_, s, l) = rgb_to_hsl(rgb);
    assert!(
        s <= 0.90 + 1e-3,
        "saturation {s} should be clamped to <= 0.90"
    );
    assert!(
        (0.40..=0.65).contains(&l),
        "lightness {l} should be within [0.40, 0.65]"
    );
}

#[test]
fn very_dark_accent_is_lifted() {
    // Near-black accent must be lifted to the lightness floor so it works as a button.
    let p = derive_palette("#020202", ColorScheme::Dark);
    let rgb = parse_hex(&p.root.accent).unwrap();
    let (_, _, l) = rgb_to_hsl(rgb);
    assert!(
        l >= 0.40 - 1e-3,
        "lightness {l} should be lifted to >= 0.40"
    );
}

#[test]
fn accent_text_contrasts_with_accent() {
    // Bright yellow accent → dark on-accent text.
    let yellow = derive_palette("#ffe600", ColorScheme::Dark);
    assert_eq!(yellow.root.accent_text, "#0a0a0a");
    // Deep blue accent → light on-accent text.
    let blue = derive_palette("#1e3a8a", ColorScheme::Dark);
    assert_eq!(blue.root.accent_text, "#fafafa");
}

#[test]
fn scheme_controls_which_palettes_are_emitted() {
    assert!(derive_palette("#0d9488", ColorScheme::Dark)
        .prefers_light
        .is_none());
    assert!(derive_palette("#0d9488", ColorScheme::Light)
        .prefers_light
        .is_none());
    assert!(derive_palette("#0d9488", ColorScheme::Auto)
        .prefers_light
        .is_some());
}

#[test]
fn dark_and_light_backgrounds_differ() {
    let auto = derive_palette("#0d9488", ColorScheme::Auto);
    let light = auto.prefers_light.unwrap();
    assert_ne!(auto.root.bg, light.bg);
    // Dark canvas is near-black; light canvas is near-white.
    assert!(relative_luminance(parse_hex(&auto.root.bg).unwrap()) < 0.1);
    assert!(relative_luminance(parse_hex(&light.bg).unwrap()) > 0.8);
}

#[test]
fn background_is_tinted_not_flat_black() {
    // The brand tint must actually shift the canvas off pure #0a0a0a.
    let p = derive_palette("#ff0000", ColorScheme::Dark);
    assert_ne!(p.root.bg, "#0a0a0a");
}
