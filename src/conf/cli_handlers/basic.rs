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

use std::path::Path;
#[cfg(target_os = "android")]
use std::process::Command;

use anyhow::{Context, Result, bail};

use super::shared::{decode_hex_json, load_config_session, load_effective_config};
use crate::{
    conf::{
        cli::Cli,
        config::{self, Config},
        store::ConfigPatch,
    },
    core::{inventory::listing as modules, runtime_state::RuntimeState},
    utils,
};

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

pub fn handle_save_config(cli: &Cli, payload: &str) -> Result<()> {
    let patch: ConfigPatch = decode_hex_json(payload, "config")?;
    let mut session = load_config_session(cli)?;
    session.apply_patch(patch);
    let path = session.save().context("Failed to save config file")?;

    println!("Configuration saved successfully to {}.", path.display());
    Ok(())
}

pub fn handle_save_module_rules(cli: &Cli, module_id: &str, payload: &str) -> Result<()> {
    utils::validate_module_id(module_id)?;
    let new_rules: config::ModuleRules = decode_hex_json(payload, "module rules")?;
    let mut session = load_config_session(cli)?;
    session.save_module_rules(module_id, new_rules);
    let path = session
        .save()
        .context("Failed to update config file with new rules")?;

    println!(
        "Module rules saved for {} into {}",
        module_id,
        path.display()
    );
    Ok(())
}

pub fn handle_save_all_module_rules(cli: &Cli, payload: &str) -> Result<()> {
    use std::collections::HashMap;

    let all_rules: HashMap<String, config::ModuleRules> =
        decode_hex_json(payload, "all module rules")?;
    let mut session = load_config_session(cli)?;

    for (module_id, rules) in &all_rules {
        utils::validate_module_id(module_id)?;
        session.save_module_rules(module_id, rules.clone());
    }

    let path = session
        .save()
        .context("Failed to update config file with batch rules")?;

    println!(
        "Batch saved {} module rules into {}",
        all_rules.len(),
        path.display()
    );
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
    #[cfg(target_os = "android")]
    {
        let output = Command::new("logcat")
            .args(["-d", "-v", "brief", "-s", "Hybrid_Logger"])
            .output()
            .context("Failed to execute logcat for Hybrid Mount logs")?;

        if !output.status.success() {
            bail!(
                "logcat exited with status {} while fetching Hybrid Mount logs",
                output.status
            );
        }

        let selected: Vec<String> = String::from_utf8(output.stdout)
            .context("logcat returned non-UTF-8 output")?
            .lines()
            .rev()
            .take(lines)
            .map(str::to_owned)
            .collect();

        if selected.is_empty() {
            println!("No Hybrid Mount logcat entries were found.");
            return Ok(());
        }

        for line in selected.into_iter().rev() {
            println!("{line}");
        }

        return Ok(());
    }

    #[cfg(not(target_os = "android"))]
    {
        let _ = lines;
        println!("Hybrid Mount logs are emitted to Android logcat with tag Hybrid_Logger.");
        println!(
            "Run `adb shell logcat -d -v brief -s Hybrid_Logger` on the device to inspect them."
        );
        Ok(())
    }
}
