//! Theme data model: a saved set of styling applied to projected slides.
//! Serialized one-theme-per-JSON-file; see `theme_store` for persistence.

use serde::{Deserialize, Serialize};

/// RGBA colour as 4 bytes; serialized as a "#rrggbbaa" string is overkill —
/// keep it as a struct for round-trip simplicity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Rgba {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Rgba {
    pub const fn new(r: u8, g: u8, b: u8, a: u8) -> Self {
        Rgba { r, g, b, a }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HAlign {
    Left,
    Center,
    Right,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum VAlign {
    Top,
    Middle,
    Bottom,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Shadow {
    pub enabled: bool,
    pub color: Rgba,
    pub blur: f32,
    pub dx: f32,
    pub dy: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Outline {
    pub enabled: bool,
    pub color: Rgba,
    pub width: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TextStyle {
    pub font_family: String,
    /// `None` = auto-fit; `Some(pt)` = fixed point size.
    pub font_size_pt: Option<f32>,
    pub font_weight: u16,
    pub color: Rgba,
    pub h_align: HAlign,
    pub v_align: VAlign,
    pub line_spacing: f32,
    pub shadow: Shadow,
    pub outline: Outline,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Background {
    Solid { color: Rgba },
    Gradient { from: Rgba, to: Rgba, angle: f32 },
    Image { path: String },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ImageFit {
    Cover,
    Contain,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BackgroundStyle {
    pub kind: Background,
    pub image_fit: ImageFit,
    pub overlay_color: Rgba,
    pub overlay_opacity: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LayoutStyle {
    pub margin: f32,
    /// Fraction of slide width (0..1) the text may occupy.
    pub max_text_width: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FooterContent {
    HymnNumberTitle,
    SlideCounter,
    None,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FooterStyle {
    pub show: bool,
    pub content: FooterContent,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Theme {
    pub name: String,
    pub text: TextStyle,
    pub background: BackgroundStyle,
    pub layout: LayoutStyle,
    pub footer: FooterStyle,
}

impl Default for Theme {
    fn default() -> Self {
        Theme {
            name: "Default".into(),
            text: TextStyle {
                font_family: "sans-serif".into(),
                font_size_pt: Some(44.0),
                font_weight: 700,
                color: Rgba::new(255, 255, 255, 255),
                h_align: HAlign::Center,
                v_align: VAlign::Middle,
                line_spacing: 1.3,
                shadow: Shadow {
                    enabled: false,
                    color: Rgba::new(0, 0, 0, 180),
                    blur: 4.0,
                    dx: 2.0,
                    dy: 2.0,
                },
                outline: Outline {
                    enabled: false,
                    color: Rgba::new(0, 0, 0, 255),
                    width: 1.0,
                },
            },
            background: BackgroundStyle {
                kind: Background::Solid {
                    color: Rgba::new(11, 31, 58, 255),
                },
                image_fit: ImageFit::Cover,
                overlay_color: Rgba::new(0, 0, 0, 255),
                overlay_opacity: 0.0,
            },
            layout: LayoutStyle {
                margin: 48.0,
                max_text_width: 0.8,
            },
            footer: FooterStyle {
                show: false,
                content: FooterContent::None,
            },
        }
    }
}

impl Theme {
    pub fn to_json(&self) -> anyhow::Result<String> {
        Ok(serde_json::to_string_pretty(self)?)
    }

    pub fn from_json(text: &str) -> anyhow::Result<Theme> {
        Ok(serde_json::from_str(text)?)
    }

    /// The built-in default theme cannot be deleted or overwritten on disk.
    pub fn is_builtin_default(&self) -> bool {
        self.name == "Default"
    }
}
