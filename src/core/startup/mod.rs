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

mod recovery;

use anyhow::{Context, Result};

use crate::{
    conf::{cli::Cli, config::Config, loader},
    defs, sys, utils,
};

fn load_final_config(cli: &Cli) -> Result<Config> {
    let mut config = loader::load_config(cli)?;
    config.merge_with_cli(
        cli.moduledir.clone(),
        cli.mountsource.clone(),
        cli.partitions.clone(),
    );
    Ok(config)
}

pub fn run(cli: &Cli) -> Result<()> {
    sys::fs::ensure_dir_exists(defs::RUN_DIR)
        .with_context(|| format!("Failed to create run directory: {}", defs::RUN_DIR))?;

    let config = load_final_config(cli)?;

    utils::init_logging().context("Failed to initialize logging")?;
    crate::scoped_log!(info, "startup", "init: daemon=hybrid-mount");

    if let Ok(version) = std::fs::read_to_string("/proc/sys/kernel/osrelease") {
        crate::scoped_log!(debug, "startup", "kernel: version={}", version.trim());
    }

    utils::check_ksu();

    if config.hymofs.enabled {
        match sys::lkm::autoload_if_needed(&config.hymofs) {
            Ok(true) => {
                crate::scoped_log!(
                    info,
                    "startup",
                    "hymofs lkm autoload: loaded=true, dir={}",
                    config.hymofs.lkm_dir.display()
                );
            }
            Ok(false) => {
                crate::scoped_log!(
                    debug,
                    "startup",
                    "hymofs lkm autoload: loaded=false, reason=not_needed"
                );
            }
            Err(err) => {
                crate::scoped_log!(
                    warn,
                    "startup",
                    "hymofs lkm autoload failed: error={:#}",
                    err
                );
            }
        }
    } else {
        crate::scoped_log!(debug, "startup", "hymofs disabled: skip_lkm_autoload=true");
    }

    if config.disable_umount {
        crate::scoped_log!(warn, "startup", "config: disable_umount=true");
    }

    recovery::run(config)
}
