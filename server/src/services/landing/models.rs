//! Brand-config data types for the link landing page.
//!
//! [`LandingTheme`] is the renderer's sole brand input. Customers *select* a
//! [`Template`] and feed brand inputs (colors, fonts, copy); they never author
//! HTML/CSS. Every knob is an enum or a constrained string so the rendered page
//! can't land in "trash" territory — the palette derivation engine
//! (`api/links/palette.rs`) fills in the hundred sub-decisions tastefully.
//!
//! All types derive both schema systems (the shared REST+MCP pattern from
//! CLAUDE.md) so Phase 2 can expose them over both transports without drift.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Rift's default accent — the teal the landing page shipped with. Used when a
/// theme carries no `theme_color`.
pub const DEFAULT_ACCENT: &str = "#0d9488";

/// Which first-party template renders the landing page. First-party and
/// code-owned: the variant is a *selector*, not customer-authored markup. New
/// variants are added over time without breaking existing links.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, ToSchema)]
#[cfg_attr(feature = "mcp", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum Template {
    #[default]
    Default,
}

/// Light/dark canvas strategy. `Auto` emits both palettes and lets the
/// browser pick via `prefers-color-scheme`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, ToSchema)]
#[cfg_attr(feature = "mcp", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum ColorScheme {
    Dark,
    Light,
    #[default]
    Auto,
}

/// Typeface preset. An allowlist — never an arbitrary font URL (privacy + perf).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, ToSchema)]
#[cfg_attr(feature = "mcp", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum FontPreset {
    #[default]
    System,
    Serif,
    Rounded,
    Mono,
}

/// Corner treatment for cards and the CTA button.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, ToSchema)]
#[cfg_attr(feature = "mcp", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum CornerStyle {
    Sharp,
    #[default]
    Rounded,
    Pill,
}

/// Brand configuration for a tenant's landing pages. Phase 1: held in code with
/// a `Default` impl. Phase 2: persisted on `TenantDoc`, with per-link content
/// overrides layered on top.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[cfg_attr(feature = "mcp", derive(schemars::JsonSchema))]
pub struct LandingTheme {
    /// Which first-party template to render.
    #[serde(default)]
    pub template: Template,
    /// Light/dark canvas strategy.
    #[serde(default)]
    pub color_scheme: ColorScheme,
    /// Brand accent (hex `#RGB`/`#RRGGBB`). `None` ⇒ Rift's default teal. The
    /// palette engine clamps wild values into a tasteful range.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schema(example = "#FF6B35")]
    pub theme_color: Option<String>,
    /// Typeface preset (allowlist).
    #[serde(default)]
    pub font: FontPreset,
    /// Corner treatment for cards and the CTA button.
    #[serde(default)]
    pub corner_style: CornerStyle,
    /// Brand display name shown in the CTA and header. `None` ⇒ "App".
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schema(example = "TableFour")]
    pub brand_name: Option<String>,
    /// Square brand mark URL.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schema(example = "https://cdn.example.com/icon-512.png")]
    pub icon_url: Option<String>,
    /// Wordmark/logo URL (reserved for future template use).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub logo_url: Option<String>,
    /// Short tagline shown under the title.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tagline: Option<String>,
    /// CTA button label override. `None` ⇒ "Open in {brand}".
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schema(example = "Reserve a table")]
    pub cta_label: Option<String>,
    /// Hide the "Powered by Rift" footer (paid tiers).
    #[serde(default)]
    pub hide_powered_by: bool,
    /// Show the machine-readable agent panel (the 40% side).
    #[serde(default = "default_true")]
    pub show_agent_panel: bool,
}

fn default_true() -> bool {
    true
}

impl Default for LandingTheme {
    fn default() -> Self {
        Self {
            template: Template::default(),
            color_scheme: ColorScheme::default(),
            theme_color: None,
            font: FontPreset::default(),
            corner_style: CornerStyle::default(),
            brand_name: None,
            icon_url: None,
            logo_url: None,
            tagline: None,
            cta_label: None,
            hide_powered_by: false,
            show_agent_panel: true,
        }
    }
}
