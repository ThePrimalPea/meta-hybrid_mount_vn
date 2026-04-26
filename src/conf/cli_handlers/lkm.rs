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

use anyhow::{Context, Result};

use super::shared::{load_effective_config, update_config_for_cli};
use crate::{
    conf::cli::Cli,
    core::api,
    sys::{kasumi, lkm},
};

pub fn handle_lkm_status(cli: &Cli) -> Result<()> {
    let config = load_effective_config(cli)?;
    crate::scoped_log!(debug, "cli:lkm:status", "start");
    let payload = api::build_lkm_payload(&config);
    println!(
        "{}",
        serde_json::to_string_pretty(&payload).context("Failed to serialize LKM status")?
    );
    crate::scoped_log!(debug, "cli:lkm:status", "complete");
    Ok(())
}

pub fn handle_lkm_load(cli: &Cli) -> Result<()> {
    let config = load_effective_config(cli)?;
    crate::scoped_log!(info, "cli:lkm:load", "start");
    lkm::load(&config.kasumi)?;
    kasumi::invalidate_status_cache();
    crate::scoped_log!(info, "cli:lkm:load", "complete");
    println!("Kasumi LKM loaded.");
    Ok(())
}

pub fn handle_lkm_unload(cli: &Cli) -> Result<()> {
    let config = load_effective_config(cli)?;
    crate::scoped_log!(info, "cli:lkm:unload", "start");
    lkm::unload(&config.kasumi)?;
    kasumi::invalidate_status_cache();
    crate::scoped_log!(info, "cli:lkm:unload", "complete");
    println!("Kasumi LKM unloaded.");
    Ok(())
}

pub fn handle_lkm_set_autoload(cli: &Cli, enabled: bool) -> Result<()> {
    crate::scoped_log!(info, "cli:lkm:set_autoload", "start: enabled={}", enabled);
    let (path, _) = update_config_for_cli(cli, |config| {
        config.kasumi.lkm_autoload = enabled;
    })?;
    crate::scoped_log!(
        info,
        "cli:lkm:set_autoload",
        "complete: enabled={}, path={}",
        enabled,
        path.display()
    );
    println!(
        "Kasumi LKM autoload {} in {}.",
        if enabled { "enabled" } else { "disabled" },
        path.display()
    );
    Ok(())
}

pub fn handle_lkm_set_kmi(cli: &Cli, kmi: &str) -> Result<()> {
    crate::scoped_log!(info, "cli:lkm:set_kmi", "start: kmi={}", kmi);
    let (path, _) = update_config_for_cli(cli, |config| {
        config.kasumi.lkm_kmi_override = kmi.to_string();
    })?;
    crate::scoped_log!(
        info,
        "cli:lkm:set_kmi",
        "complete: kmi={}, path={}",
        kmi,
        path.display()
    );
    println!(
        "Kasumi LKM KMI override set to {} in {}.",
        kmi,
        path.display()
    );
    Ok(())
}

pub fn handle_lkm_clear_kmi(cli: &Cli) -> Result<()> {
    crate::scoped_log!(info, "cli:lkm:clear_kmi", "start");
    let (path, _) = update_config_for_cli(cli, |config| {
        config.kasumi.lkm_kmi_override.clear();
    })?;
    crate::scoped_log!(
        info,
        "cli:lkm:clear_kmi",
        "complete: path={}",
        path.display()
    );
    println!("Kasumi LKM KMI override cleared in {}.", path.display());
    Ok(())
}
