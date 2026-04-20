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

use super::shared::{load_effective_config, require_live_hymofs};
use crate::{
    conf::cli::Cli,
    core::{api, runtime_state::RuntimeState},
    mount::hymofs as hymofs_mount,
};

pub fn handle_api_system(cli: &Cli) -> Result<()> {
    let config = load_effective_config(cli)?;
    let state = RuntimeState::load().unwrap_or_default();
    let payload = api::build_system_payload(&config, &state);
    println!(
        "{}",
        serde_json::to_string_pretty(&payload).context("Failed to serialize API system payload")?
    );
    Ok(())
}

pub fn handle_api_storage() -> Result<()> {
    let state = RuntimeState::load().unwrap_or_default();
    let payload = api::build_storage_payload(&state);
    println!(
        "{}",
        serde_json::to_string_pretty(&payload)
            .context("Failed to serialize API storage payload")?
    );
    Ok(())
}

pub fn handle_api_mount_stats() -> Result<()> {
    let state = RuntimeState::load().unwrap_or_default();
    let payload = api::build_mount_stats_payload(&state);
    println!(
        "{}",
        serde_json::to_string_pretty(&payload).context("Failed to serialize API mount stats")?
    );
    Ok(())
}

pub fn handle_api_mount_topology(cli: &Cli) -> Result<()> {
    let config = load_effective_config(cli)?;
    let state = RuntimeState::load().unwrap_or_default();
    let payload = api::build_mount_topology_payload(&config, &state);
    println!(
        "{}",
        serde_json::to_string_pretty(&payload)
            .context("Failed to serialize API mount topology payload")?
    );
    Ok(())
}

pub fn handle_api_partitions(cli: &Cli) -> Result<()> {
    let config = load_effective_config(cli)?;
    let payload = api::build_partitions_payload(&config);
    println!(
        "{}",
        serde_json::to_string_pretty(&payload).context("Failed to serialize API partitions")?
    );
    Ok(())
}

pub fn handle_api_lkm(cli: &Cli) -> Result<()> {
    let config = load_effective_config(cli)?;
    let payload = api::build_lkm_payload(&config);
    println!(
        "{}",
        serde_json::to_string_pretty(&payload).context("Failed to serialize API lkm payload")?
    );
    Ok(())
}

pub fn handle_api_features() -> Result<()> {
    let payload = api::build_features_payload();
    println!(
        "{}",
        serde_json::to_string_pretty(&payload).context("Failed to serialize API features")?
    );
    Ok(())
}

pub fn handle_api_hooks(cli: &Cli) -> Result<()> {
    let config = load_effective_config(cli)?;
    require_live_hymofs(&config, "read HymoFS hooks")?;
    println!("{}", hymofs_mount::hook_lines()?.join("\n"));
    Ok(())
}
