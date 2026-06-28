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

    /// Format as `#RRGGBB` (uppercase, alpha dropped — the editor only edits RGB).
    pub fn to_hex(&self) -> String {
        format!("#{:02X}{:02X}{:02X}", self.r, self.g, self.b)
    }

    /// Parse `#RRGGBB` / `RRGGBB` (case-insensitive). Alpha is set to 255.
    /// Returns `None` for any non-6-hex-digit input.
    pub fn from_hex(s: &str) -> Option<Rgba> {
        let h = s.strip_prefix('#').unwrap_or(s);
        if h.len() != 6 || !h.chars().all(|c| c.is_ascii_hexdigit()) {
            return None;
        }
        let r = u8::from_str_radix(&h[0..2], 16).ok()?;
        let g = u8::from_str_radix(&h[2..4], 16).ok()?;
        let b = u8::from_str_radix(&h[4..6], 16).ok()?;
        Some(Rgba::new(r, g, b, 255))
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

/// Named-file persistence for themes. One `<name>.json` per theme in a
/// directory; the built-in `Default` is always present and never written or
/// deleted on disk.
pub mod store {
    use super::Theme;
    use anyhow::{bail, Result};
    use std::path::Path;

    /// Sanitize a theme name into a safe file stem (alphanumerics, space, -, _).
    fn file_stem(name: &str) -> String {
        name.chars()
            .map(|c| if c.is_alphanumeric() || c == ' ' || c == '-' || c == '_' { c } else { '_' })
            .collect()
    }

    /// List all themes: the built-in default first, then every valid
    /// `<dir>/*.json`. Corrupt files are skipped (logged via eprintln).
    pub fn list_themes(dir: &Path) -> Vec<Theme> {
        let mut out = vec![Theme::default()];
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|e| e.to_str()) != Some("json") {
                    continue;
                }
                match std::fs::read_to_string(&path).ok().and_then(|s| Theme::from_json(&s).ok()) {
                    Some(t) if !t.is_builtin_default() => out.push(t),
                    Some(_) => {}
                    None => eprintln!("skip corrupt theme {}", path.display()),
                }
            }
        }
        out
    }

    /// Load one theme by name. Returns the built-in default for "Default".
    pub fn load_theme(dir: &Path, name: &str) -> Result<Theme> {
        if name == "Default" {
            return Ok(Theme::default());
        }
        let path = dir.join(format!("{}.json", file_stem(name)));
        let text = std::fs::read_to_string(&path)?;
        Theme::from_json(&text)
    }

    /// Save a theme to `<dir>/<name>.json`. Refuses to overwrite the built-in
    /// default's reserved name.
    pub fn save_theme(dir: &Path, theme: &Theme) -> Result<()> {
        if theme.is_builtin_default() {
            bail!("the built-in Default theme cannot be saved over");
        }
        std::fs::create_dir_all(dir)?;
        let path = dir.join(format!("{}.json", file_stem(&theme.name)));
        std::fs::write(path, theme.to_json()?)?;
        Ok(())
    }

    /// Delete a theme file. Refuses to delete the built-in default.
    pub fn delete_theme(dir: &Path, name: &str) -> Result<()> {
        if name == "Default" {
            bail!("the built-in Default theme cannot be deleted");
        }
        let path = dir.join(format!("{}.json", file_stem(name)));
        if path.exists() {
            std::fs::remove_file(path)?;
        }
        Ok(())
    }
}
