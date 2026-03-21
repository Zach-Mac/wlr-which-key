use std::f64::consts::{FRAC_PI_2, PI};
use std::str::FromStr;

use anyhow::{Error, Result, bail};
use pangocairo::{cairo, pango};
use wayrs_utils::keyboard::xkb;

use crate::DEBUG_LAYOUT;
use crate::color::Color;
use crate::config::theme::{EffectiveConfig, ThemeOverrides};
use crate::config::{self, Config, RowsPerColumn};
use crate::key::{Key, ModifierState, SingleKey};
use crate::text::{self, ComputedText};

pub struct Menu {
    pages: Vec<MenuPage>,
    cur_page: usize,
    separator: ComputedText,
    touch_mode: bool,
    nav_back_text: ComputedText,
    nav_close_text: ComputedText,
}

struct MenuPage {
    sections: Vec<PageSection>,
    parent: Option<usize>,
    overrides: ThemeOverrides,
    separator: Option<ComputedText>,
}

struct PageSection {
    item_height: f64,
    columns: Vec<MenuColumn>,
}

struct MenuColumn {
    key_col_width: f64,
    val_col_width: f64,
    items: Vec<MenuItem>,
}

struct MenuItem {
    action: Action,
    key_comp: ComputedText,
    val_comp: ComputedText,
    key: Key,
}

#[derive(Clone)]
pub enum Action {
    Quit,
    Exec { cmd: String, keep_open: bool },
    Submenu(usize),
}

impl Menu {
    pub fn new(config: &Config, touch_mode: bool) -> Result<Self> {
        let context = pango::Context::new();
        let fontmap = pangocairo::FontMap::new();
        context.set_font_map(Some(&fontmap));

        let mut this = Self {
            pages: Vec::new(),
            cur_page: 0,
            separator: ComputedText::new(&config.separator, &context, &config.font.0),
            touch_mode,
            nav_back_text: ComputedText::new("\u{2190} Back", &context, &config.font.0),
            nav_close_text: ComputedText::new("\u{2715} Close", &context, &config.font.0),
        };

        this.push_page(&context, &config.menu, config, None, ThemeOverrides::default())?;

        Ok(this)
    }

    fn push_page(
        &mut self,
        context: &pango::Context,
        entries: &[config::Entry],
        config: &Config,
        parent: Option<usize>,
        overrides: ThemeOverrides,
    ) -> Result<usize> {
        if entries.is_empty() {
            bail!("Empty menu pages are not allowed");
        }

        let page_separator = if overrides.font.is_some() || overrides.separator.is_some() {
            let effective = EffectiveConfig::new(config, &overrides);
            Some(ComputedText::new(
                effective.separator(),
                context,
                &effective.font().0,
            ))
        } else {
            None
        };

        let cur_page = self.pages.len();

        let sep_height = page_separator
            .as_ref()
            .unwrap_or(&self.separator)
            .height;

        self.pages.push(MenuPage {
            sections: Vec::new(),
            parent,
            overrides,
            separator: page_separator,
        });

        let effective = EffectiveConfig::new(config, &self.pages[cur_page].overrides);
        let font = effective.font().0.clone();
        let rows_per_column = effective.rows_per_column().cloned();
        drop(effective);

        let mut pending_items: Vec<MenuItem> = Vec::new();

        for entry in entries {
            match entry {
                config::Entry::Row { columns } => {
                    if !pending_items.is_empty() {
                        let section = Self::build_auto_section(
                            std::mem::take(&mut pending_items),
                            &rows_per_column,
                            sep_height,
                        );
                        self.pages[cur_page].sections.push(section);
                    }
                    let mut row_columns: Vec<Vec<MenuItem>> = Vec::new();
                    for col_entries in columns {
                        let mut col_items = Vec::new();
                        for e in col_entries {
                            col_items.push(self.build_menu_item(
                                e, context, &font, config, cur_page,
                            )?);
                        }
                        row_columns.push(col_items);
                    }
                    let section = Self::build_row_section(row_columns, sep_height);
                    self.pages[cur_page].sections.push(section);
                }
                _ => {
                    let item =
                        self.build_menu_item(entry, context, &font, config, cur_page)?;
                    pending_items.push(item);
                }
            }
        }

        if !pending_items.is_empty() {
            let section =
                Self::build_auto_section(pending_items, &rows_per_column, sep_height);
            self.pages[cur_page].sections.push(section);
        }

        Ok(cur_page)
    }

    fn build_menu_item(
        &mut self,
        entry: &config::Entry,
        context: &pango::Context,
        font: &pango::FontDescription,
        config: &Config,
        cur_page: usize,
    ) -> Result<MenuItem> {
        match entry {
            config::Entry::Cmd {
                key,
                cmd,
                desc,
                keep_open,
            } => Ok(MenuItem {
                action: Action::Exec {
                    cmd: cmd.into(),
                    keep_open: *keep_open,
                },
                key_comp: ComputedText::new(key.to_string(), context, font),
                val_comp: ComputedText::new(desc, context, font),
                key: key.clone(),
            }),
            config::Entry::Recursive {
                key,
                submenu: entries,
                desc,
                overrides,
            } => {
                let child_overrides =
                    overrides.as_deref().cloned().unwrap_or_default();
                let new_page =
                    self.push_page(context, entries, config, Some(cur_page), child_overrides)?;
                Ok(MenuItem {
                    action: Action::Submenu(new_page),
                    key_comp: ComputedText::new(key.to_string(), context, font),
                    val_comp: ComputedText::new(format!("+{desc}"), context, font),
                    key: key.clone(),
                })
            }
            config::Entry::ExternalSubmenu { .. } => {
                unreachable!("ExternalSubmenu should be resolved before menu creation")
            }
            config::Entry::Row { .. } => {
                unreachable!("Row entries should be handled at section level")
            }
        }
    }

    fn build_auto_section(
        items: Vec<MenuItem>,
        rows_per_column: &Option<RowsPerColumn>,
        min_height: f64,
    ) -> PageSection {
        let mut section = PageSection {
            item_height: min_height,
            columns: Vec::new(),
        };

        for (entry_i, item) in items.into_iter().enumerate() {
            let height = f64::max(item.key_comp.height, item.val_comp.height);
            if height > section.item_height {
                section.item_height = height;
            }

            let col_i = rows_per_column
                .as_ref()
                .map_or(0, |rpc| rpc.column_for_entry(entry_i));

            if col_i >= section.columns.len() {
                section.columns.push(MenuColumn {
                    key_col_width: item.key_comp.width,
                    val_col_width: item.val_comp.width,
                    items: vec![item],
                });
            } else {
                let col = &mut section.columns[col_i];
                col.key_col_width = col.key_col_width.max(item.key_comp.width);
                col.val_col_width = col.val_col_width.max(item.val_comp.width);
                col.items.push(item);
            }
        }

        section
    }

    fn build_row_section(column_groups: Vec<Vec<MenuItem>>, min_height: f64) -> PageSection {
        let mut section = PageSection {
            item_height: min_height,
            columns: Vec::new(),
        };

        for items in column_groups {
            let mut col = MenuColumn {
                key_col_width: 0.0,
                val_col_width: 0.0,
                items: Vec::new(),
            };
            for item in items {
                let height = f64::max(item.key_comp.height, item.val_comp.height);
                if height > section.item_height {
                    section.item_height = height;
                }
                col.key_col_width = col.key_col_width.max(item.key_comp.width);
                col.val_col_width = col.val_col_width.max(item.val_comp.width);
                col.items.push(item);
            }
            section.columns.push(col);
        }

        section
    }

    pub fn current_overrides(&self) -> &ThemeOverrides {
        &self.pages[self.cur_page].overrides
    }

    fn touch_content_width(&self, page: &MenuPage, effective: &EffectiveConfig) -> f64 {
        let sections_width = page
            .sections
            .iter()
            .map(|section| self.touch_section_width(section, effective))
            .max_by(f64::total_cmp)
            .unwrap_or(0.0);

        if self.touch_mode {
            let nav_text = self.nav_text(page);
            let nav_min_width = nav_text.width + effective.button_padding() * 2.0;
            sections_width.max(nav_min_width)
        } else {
            sections_width
        }
    }

    fn nav_text(&self, page: &MenuPage) -> &ComputedText {
        if page.parent.is_some() {
            &self.nav_back_text
        } else {
            &self.nav_close_text
        }
    }

    fn touch_section_width(&self, section: &PageSection, effective: &EffectiveConfig) -> f64 {
        let n_cols = section.columns.len();
        if n_cols == 0 {
            return 0.0;
        }
        let button_w = self.touch_button_width(section, effective);
        button_w * n_cols as f64 + effective.column_padding() * (n_cols - 1) as f64
    }

    fn touch_button_width(&self, section: &PageSection, effective: &EffectiveConfig) -> f64 {
        if let Some(w) = effective.button_width() {
            return w;
        }
        let button_pad = effective.button_padding();
        let max_text_width = section
            .columns
            .iter()
            .flat_map(|col| col.items.iter())
            .map(|item| item.val_comp.width)
            .max_by(f64::total_cmp)
            .unwrap_or(0.0);
        max_text_width + button_pad * 2.0
    }

    fn touch_button_height(&self, effective: &EffectiveConfig, page: &MenuPage) -> f64 {
        if let Some(h) = effective.button_height() {
            return h;
        }
        let max_item_height = page
            .sections
            .iter()
            .map(|s| s.item_height)
            .max_by(f64::total_cmp)
            .unwrap_or(0.0);
        max_item_height + effective.button_padding_v() * 2.0
    }

    pub fn width(&self, config: &Config) -> f64 {
        let page = &self.pages[self.cur_page];
        let effective = EffectiveConfig::new(config, &page.overrides);
        let inset = (effective.padding() + effective.border_width()) * 2.0;

        if self.touch_mode {
            self.touch_content_width(page, &effective) + inset
        } else {
            let sep = page.separator.as_ref().unwrap_or(&self.separator);
            let content_width = page
                .sections
                .iter()
                .map(|section| {
                    section
                        .columns
                        .iter()
                        .map(|col| col.key_col_width + col.val_col_width + sep.width)
                        .sum::<f64>()
                        + (section.columns.len().saturating_sub(1)) as f64
                            * effective.column_padding()
                })
                .max_by(f64::total_cmp)
                .unwrap_or(0.0);
            content_width + inset
        }
    }

    pub fn height(&self, config: &Config) -> f64 {
        let page = &self.pages[self.cur_page];
        let effective = EffectiveConfig::new(config, &page.overrides);
        let inset = (effective.padding() + effective.border_width()) * 2.0;

        if self.touch_mode {
            let button_h = self.touch_button_height(&effective, page);
            let gap = effective.button_gap().max(effective.row_padding());

            let mut total = 0.0;
            for (i, section) in page.sections.iter().enumerate() {
                if i > 0 {
                    total += gap;
                }
                let max_rows = section
                    .columns
                    .iter()
                    .map(|col| col.items.len())
                    .max()
                    .unwrap_or(0);
                total +=
                    button_h * max_rows as f64 + gap * max_rows.saturating_sub(1) as f64;
            }
            // Nav button (back/close)
            total += gap + button_h;
            total + inset
        } else {
            let row_padding = effective.row_padding();

            let mut total = 0.0;
            for (i, section) in page.sections.iter().enumerate() {
                if i > 0 {
                    total += row_padding;
                }
                let section_height = section
                    .columns
                    .iter()
                    .map(|col| {
                        section.item_height * col.items.len() as f64
                            + row_padding * (col.items.len().saturating_sub(1)) as f64
                    })
                    .max_by(f64::total_cmp)
                    .unwrap_or(0.0);
                total += section_height;
            }
            total + inset
        }
    }

    pub fn render(&self, config: &config::Config, cairo_ctx: &cairo::Context) -> Result<()> {
        let page = &self.pages[self.cur_page];
        let effective = EffectiveConfig::new(config, &page.overrides);

        if self.touch_mode {
            self.render_touch(&effective, cairo_ctx, page)
        } else {
            let sep = page.separator.as_ref().unwrap_or(&self.separator);
            let inset = effective.padding() + effective.border_width();
            let row_padding = effective.row_padding();

            let mut section_y = inset;
            for (si, section) in page.sections.iter().enumerate() {
                if si > 0 {
                    section_y += row_padding;
                }
                let mut dx = inset;
                for col in &section.columns {
                    self.render_column(
                        &effective, cairo_ctx, dx, section_y, section, col, sep,
                    )?;
                    dx += col.key_col_width
                        + col.val_col_width
                        + sep.width
                        + effective.column_padding();
                }
                let section_height = section
                    .columns
                    .iter()
                    .map(|col| {
                        section.item_height * col.items.len() as f64
                            + row_padding * (col.items.len().saturating_sub(1)) as f64
                    })
                    .max_by(f64::total_cmp)
                    .unwrap_or(0.0);
                section_y += section_height;
            }
            Ok(())
        }
    }

    fn render_touch(
        &self,
        effective: &EffectiveConfig,
        cairo_ctx: &cairo::Context,
        page: &MenuPage,
    ) -> Result<()> {
        let inset = effective.padding() + effective.border_width();
        let button_h = self.touch_button_height(effective, page);
        let gap = effective.button_gap().max(effective.row_padding());
        let button_r = effective.button_corner_r();
        let button_color = effective.button_color();
        let button_text_color = effective.button_text_color();
        let button_border_color = effective.button_border_color();
        let button_border_width = effective.button_border_width();
        let col_gap = effective.column_padding();

        let total_content_width = self.touch_content_width(page, effective);

        let mut section_y = inset;
        for (si, section) in page.sections.iter().enumerate() {
            if si > 0 {
                section_y += gap;
            }
            let n_cols = section.columns.len();
            if n_cols == 0 {
                continue;
            }
            let button_w = if n_cols == 1 {
                total_content_width
            } else {
                (total_content_width - col_gap * (n_cols - 1) as f64) / n_cols as f64
            };

            for (col_i, col) in section.columns.iter().enumerate() {
                let col_x = inset + col_i as f64 * (button_w + col_gap);
                for (row_i, item) in col.items.iter().enumerate() {
                    let btn_y = section_y + row_i as f64 * (button_h + gap);

                    // Button fill
                    button_color.apply(cairo_ctx);
                    rounded_rect(cairo_ctx, col_x, btn_y, button_w, button_h, button_r);
                    cairo_ctx.fill().unwrap();

                    // Button border
                    if button_border_width > 0.0 {
                        if let Some(bc) = button_border_color {
                            bc.apply(cairo_ctx);
                            rounded_rect(
                                cairo_ctx, col_x, btn_y, button_w, button_h, button_r,
                            );
                            cairo_ctx.set_line_width(button_border_width);
                            cairo_ctx.stroke().unwrap();
                        }
                    }

                    // Label (centered)
                    item.val_comp.render(
                        cairo_ctx,
                        text::RenderOptions {
                            x: col_x + (button_w - item.val_comp.width) * 0.5,
                            y: btn_y,
                            fg_color: button_text_color,
                            height: button_h,
                        },
                    )?;
                }
            }

            let max_rows = section
                .columns
                .iter()
                .map(|col| col.items.len())
                .max()
                .unwrap_or(0);
            section_y +=
                button_h * max_rows as f64 + gap * max_rows.saturating_sub(1) as f64;
        }

        // Nav button (back/close)
        section_y += gap;
        let nav_text = self.nav_text(page);

        button_color.apply(cairo_ctx);
        rounded_rect(cairo_ctx, inset, section_y, total_content_width, button_h, button_r);
        cairo_ctx.fill().unwrap();

        if button_border_width > 0.0 {
            if let Some(bc) = button_border_color {
                bc.apply(cairo_ctx);
                rounded_rect(
                    cairo_ctx, inset, section_y, total_content_width, button_h, button_r,
                );
                cairo_ctx.set_line_width(button_border_width);
                cairo_ctx.stroke().unwrap();
            }
        }

        nav_text.render(
            cairo_ctx,
            text::RenderOptions {
                x: inset + (total_content_width - nav_text.width) * 0.5,
                y: section_y,
                fg_color: button_text_color,
                height: button_h,
            },
        )?;

        Ok(())
    }

    fn render_column(
        &self,
        effective: &EffectiveConfig,
        cairo_ctx: &cairo::Context,
        dx: f64,
        dy: f64,
        section: &PageSection,
        column: &MenuColumn,
        sep: &ComputedText,
    ) -> Result<()> {
        let row_stride = section.item_height + effective.row_padding();
        for (i, comp) in column.items.iter().enumerate() {
            let item_y = dy + row_stride * (i as f64);
            comp.key_comp.render(
                cairo_ctx,
                text::RenderOptions {
                    x: dx + column.key_col_width - comp.key_comp.width,
                    y: item_y,
                    fg_color: effective.key_color(),
                    height: section.item_height,
                },
            )?;
            sep.render(
                cairo_ctx,
                text::RenderOptions {
                    x: dx + column.key_col_width,
                    y: item_y,
                    fg_color: effective.color(),
                    height: section.item_height,
                },
            )?;
            comp.val_comp.render(
                cairo_ctx,
                text::RenderOptions {
                    x: dx + column.key_col_width + sep.width,
                    y: item_y,
                    fg_color: effective.desc_color(),
                    height: section.item_height,
                },
            )?;
        }

        if *DEBUG_LAYOUT {
            Color::from_rgba(0, 0, 255, 255).apply(cairo_ctx);
            cairo_ctx.rectangle(
                dx,
                dy,
                column.key_col_width + column.val_col_width + sep.width,
                column.items.len() as f64 * section.item_height
                    + (column.items.len().saturating_sub(1)) as f64 * effective.row_padding(),
            );
            cairo_ctx.set_line_width(1.0);
            cairo_ctx.stroke().unwrap();
        }

        Ok(())
    }

    pub fn get_action_at(&self, x: f64, y: f64, config: &Config) -> Option<Action> {
        let page = &self.pages[self.cur_page];
        let effective = EffectiveConfig::new(config, &page.overrides);
        let inset = effective.padding() + effective.border_width();

        if self.touch_mode {
            let button_h = self.touch_button_height(&effective, page);
            let gap = effective.button_gap().max(effective.row_padding());
            let col_gap = effective.column_padding();
            let total_content_width = self.touch_content_width(page, &effective);

            let mut section_y = inset;
            for (si, section) in page.sections.iter().enumerate() {
                if si > 0 {
                    section_y += gap;
                }
                let n_cols = section.columns.len();
                if n_cols == 0 {
                    continue;
                }
                let button_w = if n_cols == 1 {
                    total_content_width
                } else {
                    (total_content_width - col_gap * (n_cols - 1) as f64) / n_cols as f64
                };

                for (col_i, col) in section.columns.iter().enumerate() {
                    let col_x = inset + col_i as f64 * (button_w + col_gap);
                    for (row_i, item) in col.items.iter().enumerate() {
                        let btn_y = section_y + row_i as f64 * (button_h + gap);
                        if x >= col_x
                            && x <= col_x + button_w
                            && y >= btn_y
                            && y <= btn_y + button_h
                        {
                            return Some(item.action.clone());
                        }
                    }
                }

                let max_rows = section
                    .columns
                    .iter()
                    .map(|col| col.items.len())
                    .max()
                    .unwrap_or(0);
                section_y +=
                    button_h * max_rows as f64 + gap * max_rows.saturating_sub(1) as f64;
            }

            // Nav button hit-test
            section_y += gap;
            if x >= inset
                && x <= inset + total_content_width
                && y >= section_y
                && y <= section_y + button_h
            {
                return Some(if let Some(parent) = page.parent {
                    Action::Submenu(parent)
                } else {
                    Action::Quit
                });
            }
        } else {
            let sep = page.separator.as_ref().unwrap_or(&self.separator);
            let row_padding = effective.row_padding();

            let mut section_y = inset;
            for (si, section) in page.sections.iter().enumerate() {
                if si > 0 {
                    section_y += row_padding;
                }
                let row_stride = section.item_height + row_padding;
                let mut dx = inset;

                for col in &section.columns {
                    let col_w = col.key_col_width + col.val_col_width + sep.width;
                    if x >= dx && x <= dx + col_w {
                        for (i, item) in col.items.iter().enumerate() {
                            let item_y = section_y + row_stride * (i as f64);
                            if y >= item_y && y <= item_y + section.item_height {
                                return Some(item.action.clone());
                            }
                        }
                    }
                    dx += col_w + effective.column_padding();
                }

                let section_height = section
                    .columns
                    .iter()
                    .map(|col| {
                        section.item_height * col.items.len() as f64
                            + row_padding * (col.items.len().saturating_sub(1)) as f64
                    })
                    .max_by(f64::total_cmp)
                    .unwrap_or(0.0);
                section_y += section_height;
            }
        }

        None
    }

    pub fn get_action(&self, modifiers: ModifierState, sym: xkb::Keysym) -> Option<Action> {
        let page = &self.pages[self.cur_page];

        let action = page.sections.iter().find_map(|section| {
            section.columns.iter().find_map(|col| {
                col.items
                    .iter()
                    .find_map(|i| i.key.matches(sym, modifiers).then(|| i.action.clone()))
            })
        });
        if action.is_some() {
            return action;
        }

        match sym {
            xkb::Keysym::Escape => {
                return Some(Action::Quit);
            }
            xkb::Keysym::bracketleft | xkb::Keysym::g if modifiers.mod_ctrl => {
                return Some(Action::Quit);
            }
            xkb::Keysym::BackSpace => {
                if let Some(parent) = page.parent {
                    return Some(Action::Submenu(parent));
                }
            }
            _ => (),
        }

        None
    }

    pub fn set_page(&mut self, page: usize) {
        self.cur_page = page;
    }

    pub fn navigate_to_key_sequence(&mut self, key_sequence: &str) -> Result<Option<Action>> {
        let mut last_action = None;
        for key_str in key_sequence.split_whitespace() {
            if let Some((last_key_str, _action)) = &last_action {
                bail!("Key '{last_key_str}' leads to a command, but more keys follow in sequence");
            }
            let key = SingleKey::from_str(key_str).map_err(Error::msg)?;
            match self.get_action(key.modifiers, key.keysym) {
                Some(Action::Submenu(submenu_page)) => self.set_page(submenu_page),
                Some(action) => last_action = Some((key_str, action)),
                None => bail!("Key '{}' not found in current menu", key_str),
            }
        }
        Ok(last_action.map(|x| x.1))
    }
}

fn rounded_rect(ctx: &cairo::Context, x: f64, y: f64, w: f64, h: f64, r: f64) {
    ctx.new_sub_path();
    ctx.arc(x + w - r, y + r, r, -FRAC_PI_2, 0.0);
    ctx.arc(x + w - r, y + h - r, r, 0.0, FRAC_PI_2);
    ctx.arc(x + r, y + h - r, r, FRAC_PI_2, PI);
    ctx.arc(x + r, y + r, r, PI, 3.0 * FRAC_PI_2);
    ctx.close_path();
}
