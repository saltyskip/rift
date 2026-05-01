//! Request / response and query DTOs for `api/links/routes.rs`.

use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct ResolveQuery {
    #[serde(default)]
    pub redirect: Option<String>,
}

#[derive(Debug, Deserialize, utoipa::IntoParams)]
#[serde(rename_all = "camelCase")]
pub struct QrCodeQuery {
    /// Logo image URL to center in the QR code.
    pub logo: Option<String>,
    /// Output size in pixels. Defaults to 600.
    #[param(example = 600)]
    pub size: Option<u32>,
    /// QR error correction level. One of L, M, Q, H. Defaults to L (or H when a logo is set).
    #[param(example = "H")]
    pub level: Option<String>,
    /// Foreground color hex value applied to dots, eye frames, and eye pupils when their
    /// per-component color is not set. Defaults to #000000.
    #[serde(rename = "fgColor")]
    #[param(example = "#111827")]
    pub fg_color: Option<String>,
    /// Background color hex value. Defaults to #FFFFFF.
    #[serde(rename = "bgColor")]
    #[param(example = "#FFFFFF")]
    pub bg_color: Option<String>,
    /// Ignore the logo URL when true.
    #[serde(default, rename = "hideLogo")]
    pub hide_logo: bool,
    /// Margin around the QR code in modules. Defaults to 2.
    #[param(example = 2)]
    pub margin: Option<u32>,
    /// Deprecated compatibility flag. If false and margin is absent, margin becomes 0.
    #[serde(rename = "includeMargin")]
    pub include_margin: Option<bool>,
    /// Shape of the inner dots. One of `square`, `dots`, `rounded`, `classy`,
    /// `classy-rounded`, `extra-rounded`. Defaults to `rounded`.
    #[serde(rename = "dotType")]
    #[param(example = "rounded")]
    pub dot_type: Option<String>,
    /// Shape of the three large positioning "eye" frames. One of `square`, `dot`,
    /// `extra-rounded`. Defaults to `extra-rounded`.
    #[serde(rename = "cornerSquareType")]
    #[param(example = "extra-rounded")]
    pub corner_square_type: Option<String>,
    /// Shape of the pupil inside each eye frame. One of `dot`, `square`. Defaults to `dot`.
    #[serde(rename = "cornerDotType")]
    #[param(example = "dot")]
    pub corner_dot_type: Option<String>,
    /// Overall canvas shape. One of `square`, `circle`. Defaults to `square`.
    #[param(example = "square")]
    pub shape: Option<String>,
    /// Override color for the inner dots only. Defaults to `fgColor`.
    #[serde(rename = "dotColor")]
    #[param(example = "#0d9488")]
    pub dot_color: Option<String>,
    /// Override color for the eye frames only. Defaults to `fgColor`.
    #[serde(rename = "cornerSquareColor")]
    #[param(example = "#111827")]
    pub corner_square_color: Option<String>,
    /// Override color for the eye pupils only. Defaults to `fgColor`.
    #[serde(rename = "cornerDotColor")]
    #[param(example = "#111827")]
    pub corner_dot_color: Option<String>,
}
