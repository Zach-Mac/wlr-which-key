use serde::Deserialize;

use super::entry::Entry;
use super::{Config, Font};
use crate::color::Color;

#[derive(Deserialize, Default, Clone)]
#[serde(default)]
pub struct ThemeOverrides {
    pub background: Option<Color>,
    pub color: Option<Color>,
    pub key_color: Option<Color>,
    pub desc_color: Option<Color>,
    pub border: Option<Color>,
    pub font: Option<Font>,
    pub separator: Option<String>,
    pub border_width: Option<f64>,
    pub corner_r: Option<f64>,
    pub padding: Option<f64>,
    pub column_padding: Option<f64>,
    pub row_padding: Option<f64>,
    pub rows_per_column: Option<usize>,
}

#[derive(Deserialize)]
pub struct SubmenuFile {
    #[serde(flatten)]
    pub overrides: ThemeOverrides,
    pub menu: Vec<Entry>,
}

pub struct EffectiveConfig<'a> {
    base: &'a Config,
    overrides: &'a ThemeOverrides,
}

impl<'a> EffectiveConfig<'a> {
    pub fn new(base: &'a Config, overrides: &'a ThemeOverrides) -> Self {
        Self { base, overrides }
    }

    pub fn background(&self) -> Color {
        self.overrides.background.unwrap_or(self.base.background)
    }

    pub fn color(&self) -> Color {
        self.overrides.color.unwrap_or(self.base.color)
    }

    pub fn key_color(&self) -> Color {
        self.overrides
            .key_color
            .or(self.overrides.color)
            .unwrap_or(self.base.key_color())
    }

    pub fn desc_color(&self) -> Color {
        self.overrides
            .desc_color
            .or(self.overrides.color)
            .unwrap_or(self.base.desc_color())
    }

    pub fn border(&self) -> Color {
        self.overrides.border.unwrap_or(self.base.border)
    }

    pub fn font(&self) -> &Font {
        self.overrides.font.as_ref().unwrap_or(&self.base.font)
    }

    pub fn separator(&self) -> &str {
        self.overrides
            .separator
            .as_deref()
            .unwrap_or(&self.base.separator)
    }

    pub fn border_width(&self) -> f64 {
        self.overrides.border_width.unwrap_or(self.base.border_width)
    }

    pub fn corner_r(&self) -> f64 {
        self.overrides.corner_r.unwrap_or(self.base.corner_r)
    }

    pub fn padding(&self) -> f64 {
        self.overrides.padding.unwrap_or(self.base.padding())
    }

    pub fn column_padding(&self) -> f64 {
        self.overrides
            .column_padding
            .unwrap_or_else(|| self.overrides.padding.unwrap_or(self.base.column_padding()))
    }

    pub fn row_padding(&self) -> f64 {
        self.overrides.row_padding.unwrap_or(self.base.row_padding())
    }

    pub fn rows_per_column(&self) -> Option<usize> {
        self.overrides
            .rows_per_column
            .or(self.base.rows_per_column)
    }
}

impl ThemeOverrides {
    pub fn has_any(&self) -> bool {
        self.background.is_some()
            || self.color.is_some()
            || self.key_color.is_some()
            || self.desc_color.is_some()
            || self.border.is_some()
            || self.font.is_some()
            || self.separator.is_some()
            || self.border_width.is_some()
            || self.corner_r.is_some()
            || self.padding.is_some()
            || self.column_padding.is_some()
            || self.row_padding.is_some()
            || self.rows_per_column.is_some()
    }
}
