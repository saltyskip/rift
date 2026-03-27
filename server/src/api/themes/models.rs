use mongodb::bson::{oid::ObjectId, DateTime};
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ThemeStatus {
    Active,
    Archived,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum LayoutTemplate {
    Split,
    Centered,
    Editorial,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ContentAlignment {
    Left,
    Center,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ContentWidth {
    Narrow,
    Regular,
    Wide,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AgentPanelMode {
    Expanded,
    Compact,
    HiddenOnMobile,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum FontPreset {
    SystemSans,
    ModernSans,
    HumanistSans,
    GeometricSans,
    EditorialSerif,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TypeScale {
    Compact,
    Comfortable,
    Spacious,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RadiusPreset {
    Sharp,
    Soft,
    Rounded,
    Pill,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ButtonStyle {
    Solid,
    Outline,
    Soft,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CardStyle {
    Flat,
    Elevated,
    Glass,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ShadowPreset {
    None,
    Soft,
    Medium,
    Dramatic,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum BackgroundStyle {
    Solid,
    Gradient,
    Image,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, Default)]
pub struct GradientConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(example = "#0B0D12")]
    pub from: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(example = "#111827")]
    pub to: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(example = 135)]
    pub angle: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, Default)]
pub struct ThemePalette {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub primary: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub secondary: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub accent: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub background: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub surface: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub surface_muted: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text_muted: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub border: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub success: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub warning: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub danger: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, Default)]
pub struct ThemeTypography {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub heading_font: Option<FontPreset>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body_font: Option<FontPreset>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mono_font: Option<FontPreset>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scale: Option<TypeScale>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, Default)]
pub struct ThemeShape {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub radius: Option<RadiusPreset>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub button_style: Option<ButtonStyle>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub card_style: Option<CardStyle>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub shadow: Option<ShadowPreset>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, Default)]
pub struct ThemeBackground {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub style: Option<BackgroundStyle>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub solid: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gradient: Option<GradientConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(example = 0.32)]
    pub overlay_opacity: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, Default)]
pub struct ThemeMotion {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subtle_reveal: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub button_lift: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ambient_glow: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, Default)]
pub struct ThemeTokens {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub palette: Option<ThemePalette>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub typography: Option<ThemeTypography>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub shape: Option<ThemeShape>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub background: Option<ThemeBackground>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub motion: Option<ThemeMotion>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, Default)]
pub struct ThemeCopy {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub brand_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tagline: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_headline: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_subheadline: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub primary_cta_label: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub secondary_cta_label: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub footer_text: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, Default)]
pub struct ThemeMedia {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logo_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wordmark_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hero_image_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub og_image_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, Default)]
pub struct ThemeLayout {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub template: Option<LayoutTemplate>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub alignment: Option<ContentAlignment>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content_width: Option<ContentWidth>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agent_panel_mode: Option<AgentPanelMode>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, Default)]
pub struct ThemeModules {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub show_logo: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub show_icon: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub show_hero_image: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub show_trust_panel: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub show_agent_panel: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub show_footer: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub show_store_badges: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, Default)]
pub struct ThemeSeo {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_og_title_template: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_og_description_template: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub twitter_card: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LandingTheme {
    #[serde(rename = "_id")]
    pub id: ObjectId,
    pub tenant_id: ObjectId,
    pub name: String,
    pub slug: String,
    pub is_default: bool,
    pub status: ThemeStatus,
    pub tokens: ThemeTokens,
    pub copy: ThemeCopy,
    pub media: ThemeMedia,
    pub layout: ThemeLayout,
    pub modules: ThemeModules,
    pub seo: ThemeSeo,
    pub created_at: DateTime,
    pub updated_at: DateTime,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateThemeRequest {
    #[schema(example = "Nord Roast")]
    pub name: String,
    #[schema(example = "nord-roast")]
    pub slug: String,
    #[serde(default)]
    pub is_default: bool,
    #[serde(default)]
    pub tokens: ThemeTokens,
    #[serde(default)]
    pub copy: ThemeCopy,
    #[serde(default)]
    pub media: ThemeMedia,
    #[serde(default)]
    pub layout: ThemeLayout,
    #[serde(default)]
    pub modules: ThemeModules,
    #[serde(default)]
    pub seo: ThemeSeo,
}

#[derive(Debug, Deserialize, ToSchema, Default)]
pub struct UpdateThemeRequest {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub slug: Option<String>,
    #[serde(default)]
    pub is_default: Option<bool>,
    #[serde(default)]
    pub status: Option<ThemeStatus>,
    #[serde(default)]
    pub tokens: Option<ThemeTokens>,
    #[serde(default)]
    pub copy: Option<ThemeCopy>,
    #[serde(default)]
    pub media: Option<ThemeMedia>,
    #[serde(default)]
    pub layout: Option<ThemeLayout>,
    #[serde(default)]
    pub modules: Option<ThemeModules>,
    #[serde(default)]
    pub seo: Option<ThemeSeo>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ThemeDetail {
    #[schema(example = "665a1b2c3d4e5f6a7b8c9d0e")]
    pub id: String,
    pub name: String,
    pub slug: String,
    pub is_default: bool,
    pub status: ThemeStatus,
    pub tokens: ThemeTokens,
    pub copy: ThemeCopy,
    pub media: ThemeMedia,
    pub layout: ThemeLayout,
    pub modules: ThemeModules,
    pub seo: ThemeSeo,
    #[schema(example = "2025-06-15T10:30:00Z")]
    pub created_at: String,
    #[schema(example = "2025-06-15T10:30:00Z")]
    pub updated_at: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ListThemesResponse {
    pub themes: Vec<ThemeDetail>,
}

#[derive(Debug, Deserialize, IntoParams)]
pub struct ListThemesQuery {
    pub status: Option<String>,
}
