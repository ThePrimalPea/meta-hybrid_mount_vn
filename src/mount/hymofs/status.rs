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
