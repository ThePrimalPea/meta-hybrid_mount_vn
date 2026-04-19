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

use std::path::Path;

use anyhow::{Context, Result};

use super::shared::{load_effective_config, require_live_hymofs};
use crate::{conf::cli::Cli, core::user_hide_rules, mount::hymofs as hymofs_mount, sys::hymofs};

pub fn handle_hide_list() -> Result<()> {
    let rules = user_hide_rules::load_user_hide_rules()?;
    println!(
        "{}",
        serde_json::to_string_pretty(&rules).context("Failed to serialize user hide rules")?
    );
    Ok(())
}

pub fn handle_hide_add(cli: &Cli, path: &Path) -> Result<()> {
    let added = user_hide_rules::add_user_hide_rule(path)?;
    if added {
        let config = load_effective_config(cli)?;
        if hymofs_mount::can_operate(&config)
            && let Err(err) = hymofs::hide_path(path)
        {
            crate::scoped_log!(
                warn,
                "cli:hide",
                "live apply failed: path={}, error={:#}",
                path.display(),
                err
            );
        }
    }
    if added {
        println!("User hide rule added: {}", path.display());
    } else {
        println!("User hide rule already exists: {}", path.display());
    }
    Ok(())
}

pub fn handle_hide_remove(path: &Path) -> Result<()> {
    let removed = user_hide_rules::remove_user_hide_rule(path)?;
    if removed {
        println!(
            "User hide rule removed from persistent list: {}. Existing kernel hide state may persist until HymoFS rules are rebuilt.",
            path.display()
        );
    } else {
        println!("User hide rule was not present: {}", path.display());
    }
    Ok(())
}

pub fn handle_hide_apply(cli: &Cli) -> Result<()> {
    let config = load_effective_config(cli)?;
    require_live_hymofs(&config, "apply user hide rules")?;
    let (applied, failed) = user_hide_rules::apply_user_hide_rules()?;
    println!("User hide rules applied: {applied} succeeded, {failed} failed.");
    Ok(())
}
