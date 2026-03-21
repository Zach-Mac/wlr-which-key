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
    Row {
        columns: Vec<Vec<Self>>,
    },
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawEntry {
    key: Option<Key>,
    desc: Option<String>,
    cmd: Option<String>,
    keep_open: Option<bool>,
    submenu: Option<Vec<Entry>>,
    submenu_file: Option<String>,
    row: Option<Vec<Vec<Entry>>>,
}

impl TryFrom<RawEntry> for Entry {
    type Error = anyhow::Error;

    fn try_from(value: RawEntry) -> Result<Self, Self::Error> {
        // Row entries have no key/desc and are mutually exclusive with everything else
        if let Some(columns) = value.row {
            if value.key.is_some()
                || value.desc.is_some()
                || value.cmd.is_some()
                || value.keep_open.is_some()
                || value.submenu.is_some()
                || value.submenu_file.is_some()
            {
                bail!("'row' cannot be combined with other fields");
            }
            return Ok(Self::Row { columns });
        }

        let key = value
            .key
            .ok_or_else(|| anyhow::anyhow!("'key' is required"))?;
        let desc = value
            .desc
            .ok_or_else(|| anyhow::anyhow!("'desc' is required"))?;

        match (value.cmd, value.submenu, value.submenu_file) {
            (Some(cmd), None, None) => Ok(Self::Cmd {
                key,
                cmd,
                desc,
                keep_open: value.keep_open.unwrap_or(false),
            }),
            (None, Some(submenu), None) => {
                if value.keep_open.is_some() {
                    bail!("cannot have both 'submenu' and 'keep_open'");
                }
                Ok(Self::Recursive {
                    key,
                    submenu,
                    desc,
                    overrides: None,
                })
            }
            (None, None, Some(file)) => {
                if value.keep_open.is_some() {
                    bail!("cannot have both 'submenu_file' and 'keep_open'");
                }
                Ok(Self::ExternalSubmenu {
                    key,
                    file,
                    desc,
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
