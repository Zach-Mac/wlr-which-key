mod anchor;
mod compat;
mod entry;
mod font;
mod namespace;
pub mod theme;

use std::collections::HashSet;
use std::env;
use std::fs::read_to_string;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use serde::Deserialize;
use smart_default::SmartDefault;

pub use self::anchor::ConfigAnchor;
pub use self::entry::Entry;
pub use self::font::Font;
pub use self::namespace::Namespace;
pub use self::theme::EffectiveConfig;
use crate::color::Color;

#[derive(Deserialize, SmartDefault)]
#[serde(deny_unknown_fields, default)]
pub struct Config {
    #[default(Color::from_rgba_hex(0x282828ff))]
    pub background: Color,
    #[default(Color::from_rgba_hex(0xfbf1c7ff))]
    pub color: Color,
    pub key_color: Option<Color>,
    pub desc_color: Option<Color>,
    #[default(Color::from_rgba_hex(0x8ec07cff))]
    pub border: Color,

    pub anchor: ConfigAnchor,
    pub margin_top: i32,
    pub margin_right: i32,
    pub margin_bottom: i32,
    pub margin_left: i32,

    #[default(Font::new("monospace 10"))]
    pub font: Font,
    #[default(" ➜ ".into())]
    pub separator: String,
    #[default(4.0)]
    pub border_width: f64,
    #[default(20.0)]
    pub corner_r: f64,
    pub padding: Option<f64>,
    pub rows_per_column: Option<usize>,
    pub column_padding: Option<f64>,
    pub row_padding: Option<f64>,

    pub inhibit_compositor_keyboard_shortcuts: bool,
    pub auto_kbd_layout: bool,

    pub menu: Vec<Entry>,

    #[default(Namespace::new(c"wlr_which_key".to_owned()))]
    pub namespace: Namespace,
}

impl Config {
    pub fn new(name: &str) -> Result<Self> {
        let mut config_path = config_dir().context("Cound not find config directory")?;
        config_path.push("wlr-which-key");
        let config_dir = config_path.clone();
        config_path.push(name);
        config_path.set_extension("yaml");

        if !config_path.exists() {
            bail!("config file not found: {}", config_path.display());
        }

        let config_str =
            read_to_string(&config_path).context("Failed to read configuration")?;

        let mut config = match serde_yaml::from_str::<Self>(&config_str)
            .context("Failed to deserialize configuration")
        {
            Ok(config) => config,
            Err(err) => match serde_yaml::from_str::<compat::Config>(&config_str) {
                Ok(compat) => {
                    eprintln!(
                        "Warning: using the old config format, which will be removed in a future version."
                    );
                    compat.into()
                }
                Err(_compat_err) => return Err(err),
            },
        };

        let mut visited = HashSet::new();
        if let Ok(canonical) = config_path.canonicalize() {
            visited.insert(canonical);
        }
        config.menu = resolve_entries(config.menu, &config_dir, &mut visited)?;

        Ok(config)
    }

    pub fn padding(&self) -> f64 {
        self.padding.unwrap_or(self.corner_r)
    }

    pub fn column_padding(&self) -> f64 {
        self.column_padding.unwrap_or_else(|| self.padding())
    }

    pub fn row_padding(&self) -> f64 {
        self.row_padding.unwrap_or(0.0)
    }

    pub fn key_color(&self) -> Color {
        self.key_color.unwrap_or(self.color)
    }

    pub fn desc_color(&self) -> Color {
        self.desc_color.unwrap_or(self.color)
    }
}

fn config_dir() -> Option<PathBuf> {
    env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| Some(PathBuf::from(env::var_os("HOME")?).join(".config")))
}

fn resolve_entries(
    entries: Vec<Entry>,
    config_dir: &Path,
    visited: &mut HashSet<PathBuf>,
) -> Result<Vec<Entry>> {
    entries
        .into_iter()
        .map(|entry| match entry {
            Entry::ExternalSubmenu { key, file, desc } => {
                let file_path = if Path::new(&file).is_absolute() {
                    PathBuf::from(&file)
                } else {
                    config_dir.join(&file)
                };
                let file_path = if file_path.extension().is_none() {
                    file_path.with_extension("yaml")
                } else {
                    file_path
                };

                let canonical = file_path.canonicalize().with_context(|| {
                    format!("submenu_file not found: {}", file_path.display())
                })?;

                if !visited.insert(canonical.clone()) {
                    bail!(
                        "circular submenu_file include detected: {}",
                        file_path.display()
                    );
                }

                let content = read_to_string(&file_path).with_context(|| {
                    format!("failed to read submenu file: {}", file_path.display())
                })?;

                let (mut sub_entries, overrides) =
                    match serde_yaml::from_str::<Vec<Entry>>(&content) {
                        Ok(entries) => (entries, None),
                        Err(_) => {
                            let sub_file: theme::SubmenuFile =
                                serde_yaml::from_str(&content).with_context(|| {
                                    format!(
                                        "failed to parse submenu file: {}",
                                        file_path.display()
                                    )
                                })?;
                            let overrides = if sub_file.overrides.has_any() {
                                Some(Box::new(sub_file.overrides))
                            } else {
                                None
                            };
                            (sub_file.menu, overrides)
                        }
                    };

                sub_entries = resolve_entries(sub_entries, config_dir, visited)?;
                visited.remove(&canonical);

                Ok(Entry::Recursive {
                    key,
                    submenu: sub_entries,
                    desc,
                    overrides,
                })
            }
            Entry::Recursive {
                key,
                submenu,
                desc,
                overrides,
            } => Ok(Entry::Recursive {
                key,
                submenu: resolve_entries(submenu, config_dir, visited)?,
                desc,
                overrides,
            }),
            cmd @ Entry::Cmd { .. } => Ok(cmd),
        })
        .collect()
}
