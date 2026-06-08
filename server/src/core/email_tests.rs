//! Unit tests for the shared email chrome helpers.

use super::{branded_html, cta_button, fine_print, heading};

#[test]
fn branded_html_wraps_body_and_keeps_footer() {
    let out = branded_html("<p>hi</p>");
    assert!(out.contains("<p>hi</p>"), "body must be embedded");
    assert!(
        out.contains("Rift — Deep links for humans and agents"),
        "brand footer must be present"
    );
    assert!(
        out.contains("max-width: 480px"),
        "outer shell must be present"
    );
}

#[test]
fn cta_button_carries_href_and_label() {
    let out = cta_button("Accept Invitation", "https://example.com/accept");
    assert!(out.contains(r#"href="https://example.com/accept""#));
    assert!(out.contains("Accept Invitation"));
    assert!(out.contains("#0d9488"), "brand button color");
}

#[test]
fn heading_and_fine_print_render_text() {
    assert!(heading("You've been invited").contains("You've been invited"));
    assert!(fine_print("expires in 24 hours").contains("expires in 24 hours"));
}
