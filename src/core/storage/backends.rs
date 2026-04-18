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

use std::path::{Path, PathBuf};

use anyhow::Result;

use crate::core::backend::StorageBackend;

pub struct Ext4Backend {
    pub mount_point: PathBuf,
    pub mode: String,
}

impl Ext4Backend {
    pub fn new(mount_point: &Path) -> Self {
        Self {
            mount_point: mount_point.to_path_buf(),
            mode: "ext4".to_string(),
        }
    }
}

impl StorageBackend for Ext4Backend {
    fn commit(&mut self, _disable_umount: bool) -> Result<()> {
        Ok(())
    }

    fn mount_point(&self) -> &Path {
        &self.mount_point
    }

    fn mode(&self) -> &str {
        &self.mode
    }
}

pub struct TmpfsBackend {
    pub mount_point: PathBuf,
    pub mode: String,
}

impl TmpfsBackend {
    pub fn new(mount_point: &Path) -> Self {
        Self {
            mount_point: mount_point.to_path_buf(),
            mode: "tmpfs".to_string(),
        }
    }
}

impl StorageBackend for TmpfsBackend {
    fn commit(&mut self, _disable_umount: bool) -> Result<()> {
        Ok(())
    }

    fn mount_point(&self) -> &Path {
        &self.mount_point
    }

    fn mode(&self) -> &str {
        &self.mode
    }
}
