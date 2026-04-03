// Copyright 2026 Hybrid Mount Developers
// SPDX-License-Identifier: GPL-3.0-or-later

use std::path::Path;

#[cfg(any(target_os = "linux", target_os = "android"))]
use ksu::NukeExt4Sysfs;

pub fn nuke_path(path: &Path) {
    #[cfg(any(target_os = "linux", target_os = "android"))]
    {
        let mut nuke = NukeExt4Sysfs::new();
        nuke.add(path);
        if let Err(e) = nuke.execute() {
            crate::scoped_log!(
                warn,
                "nuke",
                "execute failed: path={}, error={:#}",
                path.display(),
                e
            );
        } else {
            crate::scoped_log!(debug, "nuke", "execute success: path={}", path.display());
        }
    }

    #[cfg(not(any(target_os = "linux", target_os = "android")))]
    let _ = path;
}
