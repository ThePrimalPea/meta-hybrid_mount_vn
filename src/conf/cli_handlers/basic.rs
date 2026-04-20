// Copyright (C) 2026 YuzakiKokuban <heibanbaize@gmail.com>
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result, bail};
use serde::Deserialize;

use super::shared::{decode_hex_json, load_effective_config};
use crate::{
    conf::{
        cli::Cli,
        config::{self, Config},
        schema::HymoFsConfig,
    },
    core::{inventory::listing as modules, runtime_state::RuntimeState},
    defs,
    domain::DefaultMode,
    utils,
};

#[derive(Debug, Deserialize)]
struct SaveConfigPatch {
    moduledir: Option<PathBuf>,
    mountsource: Option<String>,
    partitions: Option<Vec<String>>,
    overlay_mode: Option<config::OverlayMode>,
    disable_umount: Option<bool>,
    enable_overlay_fallback: Option<bool>,
    default_mode: Option<DefaultMode>,
    hymofs: Option<HymoFsConfig>,
    rules: Option<HashMap<String, config::ModuleRules>>,
}

impl SaveConfigPatch {
    fn apply_to(self, config: &mut Config) {
        if let Some(moduledir) = self.moduledir {
            config.moduledir = moduledir;
        }

        if let Some(mountsource) = self.mountsource {
            config.mountsource = mountsource;
        }

        if let Some(partitions) = self.partitions {
            config.partitions = partitions;
        }

        if let Some(overlay_mode) = self.overlay_mode {
            config.overlay_mode = overlay_mode;
        }

        if let Some(disable_umount) = self.disable_umount {
            config.disable_umount = disable_umount;
        }

        if let Some(enable_overlay_fallback) = self.enable_overlay_fallback {
            config.enable_overlay_fallback = enable_overlay_fallback;
        }

        if let Some(default_mode) = self.default_mode {
            config.default_mode = default_mode;
        }

        if let Some(hymofs) = self.hymofs {
            config.hymofs = hymofs;
        }

        if let Some(rules) = self.rules {
            config.rules = rules;
        }
    }
}

fn save_config_patch(config_path: &Path, patch: SaveConfigPatch) -> Result<()> {
    let mut config = Config::load_optional_from_file(config_path)
        .with_context(|| format!("Failed to load config file {}", config_path.display()))?;

    patch.apply_to(&mut config);

    config
        .save_to_file(config_path)
        .with_context(|| format!("Failed to save config file to {}", config_path.display()))
}

pub fn handle_gen_config(output: &Path, force: bool) -> Result<()> {
    if output.exists() && !force {
        bail!(
            "Config already exists at {}. Use --force to overwrite.",
            output.display()
        );
    }

    Config::default()
        .save_to_file(output)
        .with_context(|| format!("Failed to save generated config to {}", output.display()))
}

pub fn handle_show_config(cli: &Cli) -> Result<()> {
    let config = load_effective_config(cli)?;
    let json = serde_json::to_string(&config).context("Failed to serialize config to JSON")?;
    println!("{}", json);
    Ok(())
}

pub fn handle_save_config(payload: &str) -> Result<()> {
    let patch: SaveConfigPatch = decode_hex_json(payload, "config")?;

    save_config_patch(Path::new(defs::CONFIG_FILE), patch).context("Failed to save config file")?;

    println!("Configuration saved successfully.");
    Ok(())
}

pub fn handle_save_module_rules(module_id: &str, payload: &str) -> Result<()> {
    utils::validate_module_id(module_id)?;
    let new_rules: config::ModuleRules = decode_hex_json(payload, "module rules")?;
    let mut config = Config::load_default().unwrap_or_default();

    config.rules.insert(module_id.to_string(), new_rules);
    config
        .save_to_file(defs::CONFIG_FILE)
        .context("Failed to update config file with new rules")?;

    println!("Module rules saved for {} into config.toml", module_id);
    Ok(())
}

pub fn handle_modules(cli: &Cli) -> Result<()> {
    let config = load_effective_config(cli)?;
    modules::print_list(&config).context("Failed to list modules")
}

pub fn handle_state() -> Result<()> {
    let state = RuntimeState::load().context("Failed to load runtime state")?;
    println!(
        "{}",
        serde_json::to_string_pretty(&state).context("Failed to serialize runtime state")?
    );
    Ok(())
}

pub fn handle_logs(lines: usize) -> Result<()> {
    if !Path::new(defs::DAEMON_LOG_FILE).exists() {
        println!("No daemon log has been written yet.");
        return Ok(());
    }

    let content = std::fs::read_to_string(defs::DAEMON_LOG_FILE)
        .with_context(|| format!("Failed to read daemon log file {}", defs::DAEMON_LOG_FILE))?;
    let mut selected: Vec<&str> = content.lines().rev().take(lines).collect();
    selected.reverse();

    for line in selected {
        println!("{line}");
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use tempfile::tempdir;

    use super::*;
    use crate::{
        conf::schema::{HymoFsConfig, OverlayMode},
        domain::{ModuleRules, MountMode},
    };

    #[test]
    fn save_config_patch_preserves_unsent_fields() {
        let tempdir = tempdir().expect("tempdir");
        let config_path = tempdir.path().join("config.toml");

        let config = Config {
            moduledir: PathBuf::from("/data/adb/modules"),
            mountsource: "KSU".to_string(),
            partitions: vec!["system".to_string()],
            overlay_mode: OverlayMode::Ext4,
            default_mode: DefaultMode::Magic,
            hymofs: HymoFsConfig {
                enabled: true,
                ..Default::default()
            },
            rules: HashMap::from([(
                "demo".to_string(),
                ModuleRules {
                    default_mode: MountMode::Magic,
                    paths: HashMap::new(),
                },
            )]),
            ..Default::default()
        };
        config.save_to_file(&config_path).expect("seed config");

        let patch = SaveConfigPatch {
            moduledir: Some(PathBuf::from("/data/adb/custom_modules")),
            mountsource: None,
            partitions: Some(Vec::new()),
            overlay_mode: Some(OverlayMode::Tmpfs),
            disable_umount: Some(true),
            enable_overlay_fallback: Some(true),
            default_mode: None,
            hymofs: None,
            rules: None,
        };

        save_config_patch(&config_path, patch).expect("save patch");

        let saved = Config::load_optional_from_file(&config_path).expect("load saved config");
        assert_eq!(saved.moduledir, PathBuf::from("/data/adb/custom_modules"));
        assert_eq!(saved.partitions, Vec::<String>::new());
        assert_eq!(saved.overlay_mode, OverlayMode::Tmpfs);
        assert!(saved.disable_umount);
        assert!(saved.enable_overlay_fallback);
        assert_eq!(saved.default_mode, DefaultMode::Magic);
        assert!(saved.hymofs.enabled);
        assert_eq!(
            saved
                .rules
                .get("demo")
                .map(|rules| rules.default_mode.clone()),
            Some(MountMode::Magic)
        );
    }
}
