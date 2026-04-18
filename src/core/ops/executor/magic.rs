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

use std::{collections::HashSet, path::Path};

use anyhow::Result;

use crate::{conf::config, core::runtime_state::MountStatistics, mount::magic_mount};

pub(super) fn mount_magic(
    ids: &[String],
    config: &config::Config,
    tempdir: &Path,
) -> Result<(Vec<String>, MountStatistics)> {
    let magic_ws_path = tempdir.join("magic_workspace");

    crate::scoped_log!(
        debug,
        "executor:magic",
        "prepare workspace: path={}",
        magic_ws_path.display()
    );

    if !magic_ws_path.exists() {
        std::fs::create_dir_all(&magic_ws_path)?;
    }

    let module_ids: HashSet<String> = ids.iter().cloned().collect();

    let stats = magic_mount::magic_mount(
        &magic_ws_path,
        tempdir,
        &config.mountsource,
        &config.partitions,
        module_ids,
        !config.disable_umount,
    )?;

    crate::scoped_log!(
        debug,
        "executor:magic",
        "complete: module_count={}",
        ids.len()
    );

    Ok((ids.to_vec(), stats))
}
