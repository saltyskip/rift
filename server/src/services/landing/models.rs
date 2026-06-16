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

/// How the CTA button is filled. One axis of the composable [`ButtonStyle`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, ToSchema)]
#[cfg_attr(feature = "mcp", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum ButtonFill {
    #[default]
    Solid,
    /// Low-opacity brand tint.
    Tint,
    /// Transparent with a brand border.
    Outline,
    /// Brand gradient (bright → deep).
    Gradient,
}

/// Depth treatment shared by the CTA button and the app tile, so the page's
/// elevation reads as one coherent property rather than per-element opinions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, ToSchema)]
#[cfg_attr(feature = "mcp", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum Elevation {
    /// No shadow.
    Flat,
    /// Soft neutral drop shadow.
    Soft,
    /// Accent-tinted glow.
    #[default]
    Glow,
}

/// Composable CTA button styling — assembled from orthogonal properties rather
/// than chosen from named presets, so brands compose their own look while every
/// axis stays a constrained enum (no arbitrary CSS).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, ToSchema)]
#[cfg_attr(feature = "mcp", derive(schemars::JsonSchema))]
pub struct ButtonStyle {
    #[serde(default)]
    pub fill: ButtonFill,
    #[serde(default)]
    pub elevation: Elevation,
    /// Corner radius; `None` inherits the theme's `corner_style`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub radius: Option<CornerStyle>,
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
    /// Typeface preset — the system-font fallback stack when no `font_family`
    /// is set (or while a web font loads).
    #[serde(default)]
    pub font: FontPreset,
    /// Google Fonts family name (e.g. "Inter", "Space Grotesk"). Loaded from
    /// Google Fonts when set; falls back to the `font` preset's stack.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schema(example = "Space Grotesk")]
    pub font_family: Option<String>,
    /// Corner treatment for cards and the CTA button.
    #[serde(default)]
    pub corner_style: CornerStyle,
    /// Composable CTA button styling (fill / elevation / radius).
    #[serde(default)]
    pub button: ButtonStyle,
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
    /// Hide the CTA buttons on desktop, leaving only the QR — pushes desktop
    /// visitors to scan and install on their phone (acquisition links).
    #[serde(default)]
    pub hide_cta_on_desktop: bool,
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
            font_family: None,
            corner_style: CornerStyle::default(),
            button: ButtonStyle::default(),
            brand_name: None,
            icon_url: None,
            logo_url: None,
            tagline: None,
            cta_label: None,
            hide_powered_by: false,
            hide_cta_on_desktop: false,
            show_agent_panel: true,
        }
    }
}
