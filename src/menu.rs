use std::f64::consts::{FRAC_PI_2, PI};
use std::str::FromStr;

use anyhow::{Error, Result, bail};
use pangocairo::{cairo, pango};
use wayrs_utils::keyboard::xkb;

use crate::DEBUG_LAYOUT;
use crate::color::Color;
use crate::config::theme::{EffectiveConfig, ThemeOverrides};
use crate::config::{self, Config};
use crate::key::{Key, ModifierState, SingleKey};
use crate::text::{self, ComputedText};

const TOUCH_BUTTON_H_PAD: f64 = 24.0;
const TOUCH_BUTTON_V_PAD: f64 = 16.0;
const TOUCH_BUTTON_GAP: f64 = 8.0;
const TOUCH_BUTTON_R: f64 = 8.0;

pub struct Menu {
    pages: Vec<MenuPage>,
    cur_page: usize,
    separator: ComputedText,
    touch_mode: bool,
}

struct MenuPage {
    item_height: f64,
    columns: Vec<MenuColumn>,
    parent: Option<usize>,
    overrides: ThemeOverrides,
    separator: Option<ComputedText>,
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
            item_height: sep_height,
            columns: Vec::new(),
            parent,
            overrides,
            separator: page_separator,
        });

        let effective = EffectiveConfig::new(config, &self.pages[cur_page].overrides);
        let font = effective.font().0.clone();
        let rows_per_column = effective.rows_per_column();
        drop(effective);

        for (entry_i, entry) in entries.iter().enumerate() {
            let item = match entry {
                config::Entry::Cmd {
                    key,
                    cmd,
                    desc,
                    keep_open,
                } => MenuItem {
                    action: Action::Exec {
                        cmd: cmd.into(),
                        keep_open: *keep_open,
                    },
                    key_comp: ComputedText::new(key.to_string(), context, &font),
                    val_comp: ComputedText::new(desc, context, &font),
                    key: key.clone(),
                },
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
                    MenuItem {
                        action: Action::Submenu(new_page),
                        key_comp: ComputedText::new(key.to_string(), context, &font),
                        val_comp: ComputedText::new(format!("+{desc}"), context, &font),
                        key: key.clone(),
                    }
                }
                config::Entry::ExternalSubmenu { .. } => {
                    unreachable!("ExternalSubmenu should be resolved before menu creation")
                }
            };

            let height = f64::max(item.key_comp.height, item.val_comp.height);
            if height > self.pages[cur_page].item_height {
                self.pages[cur_page].item_height = height;
            }

            let col_i = rows_per_column.map_or(0, |rpc| entry_i / rpc);

            if col_i == self.pages[cur_page].columns.len() {
                self.pages[cur_page].columns.push(MenuColumn {
                    key_col_width: item.key_comp.width,
                    val_col_width: item.val_comp.width,
                    items: vec![item],
                });
            } else {
                let col = &mut self.pages[cur_page].columns[col_i];
                col.key_col_width = col.key_col_width.max(item.key_comp.width);
                col.val_col_width = col.val_col_width.max(item.val_comp.width);
                col.items.push(item);
            }
        }

        Ok(cur_page)
    }

    pub fn current_overrides(&self) -> &ThemeOverrides {
        &self.pages[self.cur_page].overrides
    }

    pub fn width(&self, config: &Config) -> f64 {
        let page = &self.pages[self.cur_page];
        let effective = EffectiveConfig::new(config, &page.overrides);
        if self.touch_mode {
            self.touch_button_width() + (effective.padding() + effective.border_width()) * 2.0
        } else {
            let sep = page.separator.as_ref().unwrap_or(&self.separator);
            page.columns
                .iter()
                .map(|col| col.key_col_width + col.val_col_width + sep.width)
                .sum::<f64>()
                + (page.columns.len() - 1) as f64 * effective.column_padding()
                + (effective.padding() + effective.border_width()) * 2.0
        }
    }

    pub fn height(&self, config: &Config) -> f64 {
        let page = &self.pages[self.cur_page];
        let effective = EffectiveConfig::new(config, &page.overrides);
        if self.touch_mode {
            let n_items: usize = page
                .columns
                .iter()
                .map(|col| col.items.len())
                .sum();
            let button_h = page.item_height + TOUCH_BUTTON_V_PAD * 2.0;
            let gap = effective.row_padding().max(TOUCH_BUTTON_GAP);
            button_h * n_items as f64
                + gap * n_items.saturating_sub(1) as f64
                + (effective.padding() + effective.border_width()) * 2.0
        } else {
            let row_padding = effective.row_padding();
            page.columns
                .iter()
                .map(|col| {
                    page.item_height * col.items.len() as f64
                        + row_padding * (col.items.len().saturating_sub(1)) as f64
                })
                .max_by(f64::total_cmp)
                .unwrap()
                + (effective.padding() + effective.border_width()) * 2.0
        }
    }

    pub fn render(&self, config: &config::Config, cairo_ctx: &cairo::Context) -> Result<()> {
        let page = &self.pages[self.cur_page];
        let effective = EffectiveConfig::new(config, &page.overrides);

        if self.touch_mode {
            self.render_touch(&effective, cairo_ctx, page)
        } else {
            let sep = page.separator.as_ref().unwrap_or(&self.separator);
            let mut dx = effective.padding() + effective.border_width();
            let dy = effective.padding() + effective.border_width();
            for col in &page.columns {
                self.render_column(&effective, cairo_ctx, dx, dy, page, col, sep)?;
                dx += col.key_col_width
                    + col.val_col_width
                    + sep.width
                    + effective.column_padding();
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
        let button_h = page.item_height + TOUCH_BUTTON_V_PAD * 2.0;
        let gap = effective.row_padding().max(TOUCH_BUTTON_GAP);
        let button_w = self.touch_button_width();

        let mut y = inset;
        for col in &page.columns {
            for item in &col.items {
                // Button background
                effective.border().apply(cairo_ctx);
                rounded_rect(cairo_ctx, inset, y, button_w, button_h, TOUCH_BUTTON_R);
                cairo_ctx.fill().unwrap();

                // Button label (centered)
                item.val_comp.render(
                    cairo_ctx,
                    text::RenderOptions {
                        x: inset + (button_w - item.val_comp.width) * 0.5,
                        y,
                        fg_color: effective.desc_color(),
                        height: button_h,
                    },
                )?;

                y += button_h + gap;
            }
        }
        Ok(())
    }

    fn touch_button_width(&self) -> f64 {
        let page = &self.pages[self.cur_page];
        let max_val_width = page
            .columns
            .iter()
            .flat_map(|col| col.items.iter())
            .map(|item| item.val_comp.width)
            .max_by(f64::total_cmp)
            .unwrap_or(0.0);
        max_val_width + TOUCH_BUTTON_H_PAD * 2.0
    }

    fn render_column(
        &self,
        effective: &EffectiveConfig,
        cairo_ctx: &cairo::Context,
        dx: f64,
        dy: f64,
        page: &MenuPage,
        column: &MenuColumn,
        sep: &ComputedText,
    ) -> Result<()> {
        let row_stride = page.item_height + effective.row_padding();
        for (i, comp) in column.items.iter().enumerate() {
            let item_y = dy + row_stride * (i as f64);
            comp.key_comp.render(
                cairo_ctx,
                text::RenderOptions {
                    x: dx + column.key_col_width - comp.key_comp.width,
                    y: item_y,
                    fg_color: effective.key_color(),
                    height: page.item_height,
                },
            )?;
            sep.render(
                cairo_ctx,
                text::RenderOptions {
                    x: dx + column.key_col_width,
                    y: item_y,
                    fg_color: effective.color(),
                    height: page.item_height,
                },
            )?;
            comp.val_comp.render(
                cairo_ctx,
                text::RenderOptions {
                    x: dx + column.key_col_width + sep.width,
                    y: item_y,
                    fg_color: effective.desc_color(),
                    height: page.item_height,
                },
            )?;
        }

        if *DEBUG_LAYOUT {
            Color::from_rgba(0, 0, 255, 255).apply(cairo_ctx);
            cairo_ctx.rectangle(
                dx,
                dy,
                column.key_col_width + column.val_col_width + sep.width,
                column.items.len() as f64 * page.item_height
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
            let button_h = page.item_height + TOUCH_BUTTON_V_PAD * 2.0;
            let gap = effective.row_padding().max(TOUCH_BUTTON_GAP);
            let button_w = self.touch_button_width();

            let mut btn_y = inset;
            for col in &page.columns {
                for item in &col.items {
                    if x >= inset
                        && x <= inset + button_w
                        && y >= btn_y
                        && y <= btn_y + button_h
                    {
                        return Some(item.action.clone());
                    }
                    btn_y += button_h + gap;
                }
            }
        } else {
            // Normal mode hit-testing
            let sep = page.separator.as_ref().unwrap_or(&self.separator);
            let row_stride = page.item_height + effective.row_padding();
            let mut dx = inset;

            for col in &page.columns {
                let col_w = col.key_col_width + col.val_col_width + sep.width;
                if x >= dx && x <= dx + col_w {
                    for (i, item) in col.items.iter().enumerate() {
                        let item_y = inset + row_stride * (i as f64);
                        if y >= item_y && y <= item_y + page.item_height {
                            return Some(item.action.clone());
                        }
                    }
                }
                dx += col_w + effective.column_padding();
            }
        }

        None
    }

    pub fn get_action(&self, modifiers: ModifierState, sym: xkb::Keysym) -> Option<Action> {
        let page = &self.pages[self.cur_page];

        let action = page.columns.iter().find_map(|col| {
            col.items
                .iter()
                .find_map(|i| i.key.matches(sym, modifiers).then(|| i.action.clone()))
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
