use serde::Deserialize;

use super::entry::Entry;
use super::{ButtonOverflow, Config, Font, RowsPerColumn};
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
    pub rows_per_column: Option<RowsPerColumn>,
    pub button_color: Option<Color>,
    pub button_text_color: Option<Color>,
    pub button_border_color: Option<Color>,
    pub button_border_width: Option<f64>,
    pub button_corner_r: Option<f64>,
    pub button_padding: Option<f64>,
    pub button_padding_v: Option<f64>,
    pub button_width: Option<f64>,
    pub button_height: Option<f64>,
    pub button_row_gap: Option<f64>,
    pub button_column_gap: Option<f64>,
    pub button_overflow: Option<ButtonOverflow>,
    pub touch_grid_ratio: Option<f64>,
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

    pub fn rows_per_column(&self) -> Option<&RowsPerColumn> {
        self.overrides
            .rows_per_column
            .as_ref()
            .or(self.base.rows_per_column.as_ref())
    }

    pub fn button_color(&self) -> Color {
        self.overrides
            .button_color
            .unwrap_or(self.base.button_color())
    }

    pub fn button_text_color(&self) -> Color {
        self.overrides
            .button_text_color
            .unwrap_or(self.base.button_text_color())
    }

    pub fn button_border_color(&self) -> Option<Color> {
        self.overrides
            .button_border_color
            .or(self.base.button_border_color)
    }

    pub fn button_border_width(&self) -> f64 {
        self.overrides
            .button_border_width
            .unwrap_or(self.base.button_border_width)
    }

    pub fn button_corner_r(&self) -> f64 {
        self.overrides
            .button_corner_r
            .unwrap_or(self.base.button_corner_r)
    }

    pub fn button_padding(&self) -> f64 {
        self.overrides
            .button_padding
            .unwrap_or(self.base.button_padding)
    }

    pub fn button_padding_v(&self) -> f64 {
        self.overrides
            .button_padding_v
            .unwrap_or(self.base.button_padding_v)
    }

    pub fn button_width(&self) -> Option<f64> {
        self.overrides.button_width.or(self.base.button_width)
    }

    pub fn button_height(&self) -> Option<f64> {
        self.overrides.button_height.or(self.base.button_height)
    }

    pub fn button_row_gap(&self) -> f64 {
        self.overrides
            .button_row_gap
            .unwrap_or(self.base.button_row_gap)
    }

    pub fn button_column_gap(&self) -> f64 {
        self.overrides
            .button_column_gap
            .unwrap_or(self.base.button_column_gap())
    }

    pub fn button_overflow(&self) -> &ButtonOverflow {
        self.overrides
            .button_overflow
            .as_ref()
            .unwrap_or(&self.base.button_overflow)
    }

    pub fn touch_grid_ratio(&self) -> f64 {
        self.overrides
            .touch_grid_ratio
            .unwrap_or(self.base.touch_grid_ratio)
    }
}

impl ThemeOverrides {
    /// Merge self over parent: self's values take precedence, parent fills in gaps.
    pub fn merge_over(&self, parent: &ThemeOverrides) -> ThemeOverrides {
        ThemeOverrides {
            background: self.background.or(parent.background),
            color: self.color.or(parent.color),
            key_color: self.key_color.or(parent.key_color),
            desc_color: self.desc_color.or(parent.desc_color),
            border: self.border.or(parent.border),
            font: self.font.clone().or(parent.font.clone()),
            separator: self.separator.clone().or(parent.separator.clone()),
            border_width: self.border_width.or(parent.border_width),
            corner_r: self.corner_r.or(parent.corner_r),
            padding: self.padding.or(parent.padding),
            column_padding: self.column_padding.or(parent.column_padding),
            row_padding: self.row_padding.or(parent.row_padding),
            rows_per_column: self.rows_per_column.clone().or(parent.rows_per_column.clone()),
            button_color: self.button_color.or(parent.button_color),
            button_text_color: self.button_text_color.or(parent.button_text_color),
            button_border_color: self.button_border_color.or(parent.button_border_color),
            button_border_width: self.button_border_width.or(parent.button_border_width),
            button_corner_r: self.button_corner_r.or(parent.button_corner_r),
            button_padding: self.button_padding.or(parent.button_padding),
            button_padding_v: self.button_padding_v.or(parent.button_padding_v),
            button_width: self.button_width.or(parent.button_width),
            button_height: self.button_height.or(parent.button_height),
            button_row_gap: self.button_row_gap.or(parent.button_row_gap),
            button_column_gap: self.button_column_gap.or(parent.button_column_gap),
            button_overflow: self.button_overflow.clone().or(parent.button_overflow.clone()),
            touch_grid_ratio: self.touch_grid_ratio.or(parent.touch_grid_ratio),
        }
    }

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
            || self.button_color.is_some()
            || self.button_text_color.is_some()
            || self.button_border_color.is_some()
            || self.button_border_width.is_some()
            || self.button_corner_r.is_some()
            || self.button_padding.is_some()
            || self.button_padding_v.is_some()
            || self.button_width.is_some()
            || self.button_height.is_some()
            || self.button_row_gap.is_some()
            || self.button_column_gap.is_some()
            || self.button_overflow.is_some()
            || self.touch_grid_ratio.is_some()
    }
}
