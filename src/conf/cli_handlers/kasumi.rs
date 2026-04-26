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

use anyhow::{Context, Result, bail};
use serde::Serialize;

use super::shared::{
    clear_pathbuf, detect_rule_file_type, load_effective_config, print_config_save_result,
    require_live_kasumi, update_config_for_cli,
};
use crate::{
    conf::{
        cli::Cli,
        schema::{KasumiConfig, KasumiKstatRuleConfig, KasumiMapsRuleConfig},
    },
    core::{
        api::{self, LkmPayload},
        runtime_state::{KasumiRuntimeInfo, RuntimeState},
    },
    mount::kasumi as kasumi_mount,
    sys::kasumi,
};

#[derive(Debug, Clone, Serialize)]
struct KasumiStatusPayload {
    pub status: String,
    pub available: bool,
    pub protocol_version: Option<i32>,
    pub feature_bits: Option<i32>,
    pub feature_names: Vec<String>,
    pub hooks: Vec<String>,
    pub rule_count: usize,
    pub user_hide_rule_count: usize,
    pub mirror_path: std::path::PathBuf,
    pub lkm: LkmPayload,
    pub config: KasumiConfig,
    pub runtime: KasumiStatusRuntime,
}

#[derive(Debug, Clone, Serialize)]
struct KasumiStatusRuntime {
    pub snapshot: KasumiRuntimeInfo,
    pub kasumi_modules: Vec<String>,
    pub active_mounts: Vec<String>,
}

pub fn handle_kasumi_status(cli: &Cli) -> Result<()> {
    let config = load_effective_config(cli)?;
    let runtime_state = match RuntimeState::load() {
        Ok(state) => state,
        Err(err) => {
            crate::scoped_log!(
                debug,
                "cli:kasumi:status",
                "fallback: reason=runtime_state_load_failed, error={:#}",
                err
            );
            RuntimeState::default()
        }
    };
    let kasumi_info = kasumi_mount::collect_runtime_info(&config);

    let output = KasumiStatusPayload {
        status: kasumi_info.status,
        available: kasumi_info.available,
        protocol_version: kasumi_info.protocol_version,
        feature_bits: kasumi_info.feature_bits,
        feature_names: kasumi_info.feature_names,
        hooks: kasumi_info.hooks,
        rule_count: kasumi_info.rule_count,
        user_hide_rule_count: kasumi_info.user_hide_rule_count,
        mirror_path: kasumi_info.mirror_path,
        lkm: api::build_lkm_payload(&config),
        config: config.kasumi.clone(),
        runtime: KasumiStatusRuntime {
            snapshot: runtime_state.kasumi.clone(),
            kasumi_modules: runtime_state.kasumi_modules.clone(),
            active_mounts: runtime_state.active_mounts.clone(),
        },
    };

    println!(
        "{}",
        serde_json::to_string_pretty(&output).context("Failed to serialize Kasumi status")?
    );
    Ok(())
}

pub fn handle_kasumi_list(cli: &Cli) -> Result<()> {
    let config = load_effective_config(cli)?;
    let payload = if kasumi_mount::can_operate(&config) {
        api::parse_kasumi_rule_listing(&kasumi::get_active_rules()?)
    } else {
        Vec::new()
    };
    println!(
        "{}",
        serde_json::to_string_pretty(&payload).context("Failed to serialize Kasumi rules")?
    );
    Ok(())
}

pub fn handle_kasumi_version(cli: &Cli) -> Result<()> {
    let config = load_effective_config(cli)?;
    let state = match RuntimeState::load() {
        Ok(state) => state,
        Err(err) => {
            crate::scoped_log!(
                debug,
                "cli:kasumi:version",
                "fallback: reason=runtime_state_load_failed, error={:#}",
                err
            );
            RuntimeState::default()
        }
    };
    let payload = api::build_kasumi_version_payload(&config, &state);
    println!(
        "{}",
        serde_json::to_string_pretty(&payload).context("Failed to serialize Kasumi version")?
    );
    Ok(())
}

pub fn handle_kasumi_features() -> Result<()> {
    let output = api::build_features_payload();
    println!(
        "{}",
        serde_json::to_string_pretty(&output).context("Failed to serialize Kasumi features")?
    );
    Ok(())
}

pub fn handle_kasumi_hooks() -> Result<()> {
    println!("{}", kasumi_mount::hook_lines()?.join("\n"));
    Ok(())
}

pub fn handle_kasumi_clear() -> Result<()> {
    crate::scoped_log!(info, "cli:kasumi:clear", "start");
    kasumi::clear_rules()?;
    crate::scoped_log!(info, "cli:kasumi:clear", "complete");
    println!("Kasumi rules cleared.");
    Ok(())
}

pub fn handle_kasumi_release_connection() -> Result<()> {
    kasumi::release_connection();
    println!("Released cached Kasumi client connection.");
    Ok(())
}

pub fn handle_kasumi_invalidate_cache() -> Result<()> {
    kasumi::invalidate_status_cache();
    println!("Invalidated cached Kasumi status.");
    Ok(())
}

pub fn handle_kasumi_fix_mounts() -> Result<()> {
    crate::scoped_log!(info, "cli:kasumi:fix_mounts", "start");
    kasumi::fix_mounts()?;
    crate::scoped_log!(info, "cli:kasumi:fix_mounts", "complete");
    println!("Kasumi mount ordering fixed.");
    Ok(())
}

pub fn handle_kasumi_set_enabled(cli: &Cli, enabled: bool) -> Result<()> {
    crate::scoped_log!(info, "cli:kasumi:set_enabled", "start: enabled={}", enabled);
    let (path, _) = update_config_for_cli(cli, |config| {
        config.kasumi.enabled = enabled;
    })?;
    kasumi::invalidate_status_cache();
    crate::scoped_log!(
        info,
        "cli:kasumi:set_enabled",
        "complete: enabled={}, path={}",
        enabled,
        path.display()
    );
    print_config_save_result(
        &path,
        if enabled {
            "Kasumi enabled state"
        } else {
            "Kasumi disabled state"
        },
    );
    Ok(())
}

pub fn handle_kasumi_set_hidexattr(cli: &Cli, enabled: bool) -> Result<()> {
    let (path, _) = update_config_for_cli(cli, |config| {
        config.kasumi.enable_hidexattr = enabled;
    })?;
    print_config_save_result(&path, "Kasumi hidexattr setting");
    Ok(())
}

pub fn handle_kasumi_set_mirror(cli: &Cli, path_value: &Path) -> Result<()> {
    let (path, _) = update_config_for_cli(cli, |config| {
        config.kasumi.mirror_path = path_value.to_path_buf();
    })?;
    print_config_save_result(&path, "Kasumi mirror path");
    Ok(())
}

pub fn handle_kasumi_set_debug(cli: &Cli, enabled: bool) -> Result<()> {
    let (path, _) = update_config_for_cli(cli, |config| {
        config.kasumi.enable_kernel_debug = enabled;
    })?;
    print_config_save_result(&path, "Kasumi kernel debug setting");
    Ok(())
}

pub fn handle_kasumi_set_stealth(cli: &Cli, enabled: bool) -> Result<()> {
    let (path, _) = update_config_for_cli(cli, |config| {
        config.kasumi.enable_stealth = enabled;
    })?;
    print_config_save_result(&path, "Kasumi stealth setting");
    Ok(())
}

pub fn handle_kasumi_set_mount_hide(
    cli: &Cli,
    enabled: bool,
    path_pattern: Option<&Path>,
) -> Result<()> {
    let (save_path, _) = update_config_for_cli(cli, |config| {
        config.kasumi.enable_mount_hide = enabled;
        config.kasumi.mount_hide.enabled = enabled;
        if enabled {
            if let Some(path_pattern) = path_pattern {
                config.kasumi.mount_hide.path_pattern = path_pattern.to_path_buf();
            }
        } else {
            clear_pathbuf(&mut config.kasumi.mount_hide.path_pattern);
        }
    })?;
    print_config_save_result(&save_path, "Kasumi mount_hide setting");
    Ok(())
}

pub fn handle_kasumi_set_maps_spoof(cli: &Cli, enabled: bool) -> Result<()> {
    let (path, _) = update_config_for_cli(cli, |config| {
        config.kasumi.enable_maps_spoof = enabled;
    })?;
    print_config_save_result(&path, "Kasumi maps_spoof setting");
    Ok(())
}

pub fn handle_kasumi_set_statfs_spoof(
    cli: &Cli,
    enabled: bool,
    path_value: Option<&Path>,
    spoof_f_type: Option<u64>,
) -> Result<()> {
    let (save_path, _) = update_config_for_cli(cli, |config| {
        config.kasumi.enable_statfs_spoof = enabled;
        config.kasumi.statfs_spoof.enabled = enabled;
        if enabled {
            if let Some(path) = path_value {
                config.kasumi.statfs_spoof.path = path.to_path_buf();
            }
            if let Some(spoof_f_type) = spoof_f_type {
                config.kasumi.statfs_spoof.spoof_f_type = spoof_f_type;
            }
        } else {
            clear_pathbuf(&mut config.kasumi.statfs_spoof.path);
            config.kasumi.statfs_spoof.spoof_f_type = 0;
        }
    })?;
    print_config_save_result(&save_path, "Kasumi statfs_spoof setting");
    Ok(())
}

pub fn handle_kasumi_set_uname(
    cli: &Cli,
    sysname: Option<&str>,
    nodename: Option<&str>,
    release: Option<&str>,
    version: Option<&str>,
    machine: Option<&str>,
    domainname: Option<&str>,
) -> Result<()> {
    if sysname.is_none()
        && nodename.is_none()
        && release.is_none()
        && version.is_none()
        && machine.is_none()
        && domainname.is_none()
    {
        bail!("No uname fields were provided. Use `kasumi uname clear` to clear spoofing.");
    }

    let (path, _) = update_config_for_cli(cli, |config| {
        if let Some(value) = sysname {
            config.kasumi.uname.sysname = value.to_string();
        }
        if let Some(value) = nodename {
            config.kasumi.uname.nodename = value.to_string();
        }
        if let Some(value) = release {
            config.kasumi.uname.release = value.to_string();
        }
        if let Some(value) = version {
            config.kasumi.uname.version = value.to_string();
        }
        if let Some(value) = machine {
            config.kasumi.uname.machine = value.to_string();
        }
        if let Some(value) = domainname {
            config.kasumi.uname.domainname = value.to_string();
        }
    })?;
    print_config_save_result(&path, "Kasumi uname spoof setting");
    Ok(())
}

pub fn handle_kasumi_clear_uname(cli: &Cli) -> Result<()> {
    let (path, _) = update_config_for_cli(cli, |config| {
        config.kasumi.uname = Default::default();
    })?;
    print_config_save_result(&path, "Kasumi uname spoof setting");
    Ok(())
}

pub fn handle_kasumi_set_cmdline(cli: &Cli, value: &str) -> Result<()> {
    let (path, _) = update_config_for_cli(cli, |config| {
        config.kasumi.cmdline_value = value.to_string();
    })?;
    print_config_save_result(&path, "Kasumi cmdline spoof setting");
    Ok(())
}

pub fn handle_kasumi_clear_cmdline(cli: &Cli) -> Result<()> {
    let (path, _) = update_config_for_cli(cli, |config| {
        config.kasumi.cmdline_value.clear();
    })?;
    print_config_save_result(&path, "Kasumi cmdline spoof setting");
    Ok(())
}

pub fn handle_kasumi_set_hide_uids(cli: &Cli, uids: &[u32]) -> Result<()> {
    let (path, _) = update_config_for_cli(cli, |config| {
        config.kasumi.hide_uids = uids.to_vec();
    })?;
    print_config_save_result(&path, "Kasumi hide_uids setting");
    Ok(())
}

pub fn handle_kasumi_clear_hide_uids(cli: &Cli) -> Result<()> {
    let (path, _) = update_config_for_cli(cli, |config| {
        config.kasumi.hide_uids.clear();
    })?;
    print_config_save_result(&path, "Kasumi hide_uids setting");
    Ok(())
}

pub fn handle_kasumi_add_maps_rule(
    cli: &Cli,
    target_ino: u64,
    target_dev: u64,
    spoofed_ino: u64,
    spoofed_dev: u64,
    path: &Path,
) -> Result<()> {
    let new_rule = KasumiMapsRuleConfig {
        target_ino,
        target_dev,
        spoofed_ino,
        spoofed_dev,
        spoofed_pathname: path.to_path_buf(),
    };

    let (path_out, _) = update_config_for_cli(cli, |config| {
        if let Some(existing) = config
            .kasumi
            .maps_rules
            .iter_mut()
            .find(|rule| rule.target_ino == target_ino && rule.target_dev == target_dev)
        {
            *existing = new_rule.clone();
        } else {
            config.kasumi.maps_rules.push(new_rule.clone());
        }
    })?;
    print_config_save_result(&path_out, "Kasumi maps rule");
    Ok(())
}

pub fn handle_kasumi_clear_maps_rules(cli: &Cli) -> Result<()> {
    let (path, _) = update_config_for_cli(cli, |config| {
        config.kasumi.maps_rules.clear();
    })?;
    print_config_save_result(&path, "Kasumi maps rules");
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub fn handle_kasumi_upsert_kstat_rule(
    cli: &Cli,
    target_ino: u64,
    target_path: &Path,
    spoofed_ino: u64,
    spoofed_dev: u64,
    spoofed_nlink: u32,
    spoofed_size: i64,
    spoofed_atime_sec: i64,
    spoofed_atime_nsec: i64,
    spoofed_mtime_sec: i64,
    spoofed_mtime_nsec: i64,
    spoofed_ctime_sec: i64,
    spoofed_ctime_nsec: i64,
    spoofed_blksize: u64,
    spoofed_blocks: u64,
    is_static: bool,
) -> Result<()> {
    let new_rule = KasumiKstatRuleConfig {
        target_ino,
        target_pathname: target_path.to_path_buf(),
        spoofed_ino,
        spoofed_dev,
        spoofed_nlink,
        spoofed_size,
        spoofed_atime_sec,
        spoofed_atime_nsec,
        spoofed_mtime_sec,
        spoofed_mtime_nsec,
        spoofed_ctime_sec,
        spoofed_ctime_nsec,
        spoofed_blksize,
        spoofed_blocks,
        is_static,
    };

    let (path, _) = update_config_for_cli(cli, |config| {
        if let Some(existing) = config
            .kasumi
            .kstat_rules
            .iter_mut()
            .find(|rule| rule.target_ino == target_ino && rule.target_pathname == target_path)
        {
            *existing = new_rule.clone();
        } else {
            config.kasumi.kstat_rules.push(new_rule.clone());
        }
    })?;
    print_config_save_result(&path, "Kasumi kstat rule");
    Ok(())
}

pub fn handle_kasumi_clear_kstat_rules_config(cli: &Cli) -> Result<()> {
    let (path, _) = update_config_for_cli(cli, |config| {
        config.kasumi.kstat_rules.clear();
    })?;
    println!(
        "Kasumi kstat rules were removed from {}. Existing kernel kstat spoof rules may persist until the LKM is reloaded.",
        path.display()
    );
    Ok(())
}

pub fn handle_kasumi_rule_add(
    cli: &Cli,
    target: &Path,
    source: &Path,
    file_type: Option<i32>,
) -> Result<()> {
    let config = load_effective_config(cli)?;
    require_live_kasumi(&config, "add Kasumi rule")?;
    let file_type = match file_type {
        Some(value) => value,
        None => detect_rule_file_type(source)?,
    };
    kasumi::add_rule(target, source, file_type)?;
    println!(
        "Kasumi ADD rule applied: target={}, source={}, file_type={}",
        target.display(),
        source.display(),
        file_type
    );
    Ok(())
}

pub fn handle_kasumi_rule_merge(cli: &Cli, target: &Path, source: &Path) -> Result<()> {
    let config = load_effective_config(cli)?;
    require_live_kasumi(&config, "add Kasumi merge rule")?;
    kasumi::add_merge_rule(target, source)?;
    println!(
        "Kasumi MERGE rule applied: target={}, source={}",
        target.display(),
        source.display()
    );
    Ok(())
}

pub fn handle_kasumi_rule_hide(cli: &Cli, path: &Path) -> Result<()> {
    let config = load_effective_config(cli)?;
    require_live_kasumi(&config, "add Kasumi hide rule")?;
    kasumi::hide_path(path)?;
    println!("Kasumi HIDE rule applied: {}", path.display());
    Ok(())
}

pub fn handle_kasumi_rule_delete(cli: &Cli, path: &Path) -> Result<()> {
    let config = load_effective_config(cli)?;
    require_live_kasumi(&config, "delete Kasumi rule")?;
    kasumi::delete_rule(path)?;
    println!("Kasumi rule deleted: {}", path.display());
    Ok(())
}

pub fn handle_kasumi_rule_add_dir(cli: &Cli, target_base: &Path, source_dir: &Path) -> Result<()> {
    let config = load_effective_config(cli)?;
    require_live_kasumi(&config, "add Kasumi rules from directory")?;
    kasumi::add_rules_from_directory(target_base, source_dir)?;
    println!(
        "Kasumi directory rules applied: target_base={}, source_dir={}",
        target_base.display(),
        source_dir.display()
    );
    Ok(())
}

pub fn handle_kasumi_rule_remove_dir(
    cli: &Cli,
    target_base: &Path,
    source_dir: &Path,
) -> Result<()> {
    let config = load_effective_config(cli)?;
    require_live_kasumi(&config, "remove Kasumi rules from directory")?;
    kasumi::remove_rules_from_directory(target_base, source_dir)?;
    println!(
        "Kasumi directory rules removed: target_base={}, source_dir={}",
        target_base.display(),
        source_dir.display()
    );
    Ok(())
}
