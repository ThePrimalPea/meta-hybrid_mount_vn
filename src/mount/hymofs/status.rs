// Copyright 2026 Hybrid Mount Developers
// SPDX-License-Identifier: GPL-3.0-or-later

use anyhow::{Result, bail};

use crate::{
    conf::config,
    core::{api, runtime_state::HymoFsRuntimeInfo, user_hide_rules},
    sys::{
        hymofs::{self, HymoFsStatus},
        lkm,
    },
};

pub fn can_operate(config: &config::Config) -> bool {
    hymofs::can_operate(config.hymofs.ignore_protocol_mismatch)
}

pub fn require_live(config: &config::Config, description: &str) -> Result<()> {
    if can_operate(config) {
        return Ok(());
    }

    bail!(
        "HymoFS is not available for {} (status={})",
        description,
        hymofs::status_name(hymofs::check_status())
    );
}

pub fn hook_lines() -> Result<Vec<String>> {
    Ok(hymofs::get_hooks()?
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(ToString::to_string)
        .collect())
}

pub fn collect_runtime_info(config: &config::Config) -> HymoFsRuntimeInfo {
    if !config.hymofs.enabled {
        return HymoFsRuntimeInfo {
            status: "disabled".to_string(),
            available: false,
            lkm_loaded: lkm::is_loaded(),
            lkm_autoload: config.hymofs.lkm_autoload,
            lkm_kmi_override: config.hymofs.lkm_kmi_override.clone(),
            lkm_current_kmi: lkm::current_kmi(),
            lkm_dir: config.hymofs.lkm_dir.clone(),
            protocol_version: None,
            feature_bits: None,
            feature_names: Vec::new(),
            hooks: Vec::new(),
            rule_count: 0,
            user_hide_rule_count: user_hide_rules::user_hide_rule_count(),
            mirror_path: config.hymofs.mirror_path.clone(),
        };
    }

    let status = hymofs::check_status();
    let protocol_version = hymofs::get_protocol_version().ok();
    let feature_bits = hymofs::get_features().ok();
    let feature_names = feature_bits.map(hymofs::feature_names).unwrap_or_default();
    let hooks = hook_lines().unwrap_or_default();
    let rule_count = hymofs::get_active_rules()
        .map(|value| api::parse_hymofs_rule_listing(&value).len())
        .unwrap_or(0);

    HymoFsRuntimeInfo {
        status: hymofs::status_name(status).to_string(),
        available: status == HymoFsStatus::Available,
        lkm_loaded: lkm::is_loaded(),
        lkm_autoload: config.hymofs.lkm_autoload,
        lkm_kmi_override: config.hymofs.lkm_kmi_override.clone(),
        lkm_current_kmi: lkm::current_kmi(),
        lkm_dir: config.hymofs.lkm_dir.clone(),
        protocol_version,
        feature_bits,
        feature_names,
        hooks,
        rule_count,
        user_hide_rule_count: user_hide_rules::user_hide_rule_count(),
        mirror_path: config.hymofs.mirror_path.clone(),
    }
}
