// Copyright (C) 2026 YuzakiKokuban <heibanbaize@gmail.com>
//
// This program is free software; you can redistribute it and/or
// modify it under the terms of the GNU General Public License
// as published by the Free Software Foundation; either version 2
// of the License, or (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program; if not, write to the Free Software
// Foundation, Inc., 51 Franklin Street, Fifth Floor, Boston, MA  02110-1301, USA.

use std::{
    fs,
    io::ErrorKind,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};
use serde::Deserialize;

use crate::{
    conf::schema::{Config, HymoFsConfig},
    defs,
};

#[derive(Debug, Deserialize, Default)]
struct LegacyWrappedHymofsConfig {
    #[serde(default)]
    hymofs: HymoFsConfig,
}

fn hymofs_sidecar_path_for(main_path: &Path) -> PathBuf {
    main_path
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join("hymofs.toml")
}

fn ensure_parent_dir(path: &Path) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).context("failed to create config directory")?;
    }
    Ok(())
}

fn load_hymofs_config_file(path: &Path) -> Result<HymoFsConfig> {
    let content = fs::read_to_string(path)
        .with_context(|| format!("failed to read HymoFS config file {}", path.display()))?;

    if content.trim().is_empty() {
        return Ok(HymoFsConfig::default());
    }

    let value: toml::Value = toml::from_str(&content)
        .with_context(|| format!("failed to parse HymoFS config file {}", path.display()))?;

    if value.get("hymofs").is_some() {
        toml::from_str::<LegacyWrappedHymofsConfig>(&content)
            .map(|wrapped| wrapped.hymofs)
            .with_context(|| format!("failed to parse wrapped HymoFS config {}", path.display()))
    } else {
        toml::from_str::<HymoFsConfig>(&content)
            .with_context(|| format!("failed to parse HymoFS config file {}", path.display()))
    }
}

fn load_merged_config(main_path: &Path, allow_missing_main: bool) -> Result<Config> {
    let mut config = if main_path.exists() {
        let content = fs::read_to_string(main_path)
            .with_context(|| format!("failed to read config file {}", main_path.display()))?;
        toml::from_str::<Config>(&content)
            .with_context(|| format!("failed to parse config file {}", main_path.display()))?
    } else if allow_missing_main {
        Config::default()
    } else {
        let _ = fs::read_to_string(main_path)
            .with_context(|| format!("failed to read config file {}", main_path.display()))?;
        unreachable!("read_to_string should have returned an error for missing config file");
    };

    let hymofs_path = hymofs_sidecar_path_for(main_path);
    if hymofs_path.exists() {
        config.hymofs = HymoFsConfig::from_file(&hymofs_path)?;
    }

    Ok(config)
}

fn remove_legacy_sidecar_if_present(main_path: &Path) -> Result<()> {
    let hymofs_path = hymofs_sidecar_path_for(main_path);
    match fs::remove_file(&hymofs_path) {
        Ok(()) => Ok(()),
        Err(err) if err.kind() == ErrorKind::NotFound => Ok(()),
        Err(err) => Err(err).with_context(|| {
            format!(
                "failed to remove legacy HymoFS config {}",
                hymofs_path.display()
            )
        }),
    }
}

impl Config {
    pub fn load_optional_from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        load_merged_config(path.as_ref(), true)
    }

    pub fn load_default() -> Result<Self> {
        load_merged_config(Path::new(defs::CONFIG_FILE), true)
    }

    pub fn save_to_file<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let main_path = path.as_ref();
        let content = toml::to_string_pretty(self).context("failed to serialize config")?;

        ensure_parent_dir(main_path)?;
        fs::write(main_path, content)
            .with_context(|| format!("failed to write config file {}", main_path.display()))?;
        remove_legacy_sidecar_if_present(main_path)?;
        Ok(())
    }
}

impl HymoFsConfig {
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        load_hymofs_config_file(path.as_ref())
    }
}
