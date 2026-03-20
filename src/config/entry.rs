use anyhow::bail;
use serde::Deserialize;

use super::theme::ThemeOverrides;
use crate::key::Key;

#[derive(Deserialize)]
#[serde(try_from = "RawEntry")]
pub enum Entry {
    Cmd {
        key: Key,
        cmd: String,
        desc: String,
        keep_open: bool,
    },
    Recursive {
        key: Key,
        submenu: Vec<Self>,
        desc: String,
        overrides: Option<Box<ThemeOverrides>>,
    },
    ExternalSubmenu {
        key: Key,
        file: String,
        desc: String,
    },
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawEntry {
    key: Key,
    desc: String,
    cmd: Option<String>,
    keep_open: Option<bool>,
    submenu: Option<Vec<Entry>>,
    submenu_file: Option<String>,
}

impl TryFrom<RawEntry> for Entry {
    type Error = anyhow::Error;

    fn try_from(value: RawEntry) -> Result<Self, Self::Error> {
        match (value.cmd, value.submenu, value.submenu_file) {
            (Some(cmd), None, None) => Ok(Self::Cmd {
                key: value.key,
                cmd,
                desc: value.desc,
                keep_open: value.keep_open.unwrap_or(false),
            }),
            (None, Some(submenu), None) => {
                if value.keep_open.is_some() {
                    bail!("cannot have both 'submenu' and 'keep_open'");
                }
                Ok(Self::Recursive {
                    key: value.key,
                    submenu,
                    desc: value.desc,
                    overrides: None,
                })
            }
            (None, None, Some(file)) => {
                if value.keep_open.is_some() {
                    bail!("cannot have both 'submenu_file' and 'keep_open'");
                }
                Ok(Self::ExternalSubmenu {
                    key: value.key,
                    file,
                    desc: value.desc,
                })
            }
            (None, None, None) => {
                bail!("one of 'cmd', 'submenu', or 'submenu_file' is required")
            }
            _ => {
                bail!("only one of 'cmd', 'submenu', or 'submenu_file' can be specified")
            }
        }
    }
}
