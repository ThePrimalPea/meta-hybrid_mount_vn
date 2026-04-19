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

use anyhow::{Context, Result};

use super::shared::{load_effective_config, save_config_for_cli};
use crate::{
    conf::cli::Cli,
    core::api,
    sys::{hymofs, lkm},
};

pub fn handle_lkm_status(cli: &Cli) -> Result<()> {
    let config = load_effective_config(cli)?;
    let payload = api::build_lkm_payload(&config);
    println!(
        "{}",
        serde_json::to_string_pretty(&payload).context("Failed to serialize LKM status")?
    );
    Ok(())
}

pub fn handle_lkm_load(cli: &Cli) -> Result<()> {
    let config = load_effective_config(cli)?;
    lkm::load(&config.hymofs)?;
    hymofs::invalidate_status_cache();
    println!("HymoFS LKM loaded.");
    Ok(())
}

pub fn handle_lkm_unload(cli: &Cli) -> Result<()> {
    let config = load_effective_config(cli)?;
    lkm::unload(&config.hymofs)?;
    hymofs::invalidate_status_cache();
    println!("HymoFS LKM unloaded.");
    Ok(())
}

pub fn handle_lkm_set_autoload(cli: &Cli, enabled: bool) -> Result<()> {
    let mut config = load_effective_config(cli)?;
    config.hymofs.lkm_autoload = enabled;
    let path = save_config_for_cli(cli, &config)?;
    println!(
        "HymoFS LKM autoload {} in {}.",
        if enabled { "enabled" } else { "disabled" },
        path.display()
    );
    Ok(())
}

pub fn handle_lkm_set_kmi(cli: &Cli, kmi: &str) -> Result<()> {
    let mut config = load_effective_config(cli)?;
    config.hymofs.lkm_kmi_override = kmi.to_string();
    let path = save_config_for_cli(cli, &config)?;
    println!(
        "HymoFS LKM KMI override set to {} in {}.",
        kmi,
        path.display()
    );
    Ok(())
}

pub fn handle_lkm_clear_kmi(cli: &Cli) -> Result<()> {
    let mut config = load_effective_config(cli)?;
    config.hymofs.lkm_kmi_override.clear();
    let path = save_config_for_cli(cli, &config)?;
    println!("HymoFS LKM KMI override cleared in {}.", path.display());
    Ok(())
}
