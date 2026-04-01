// Copyright 2026 Hybrid Mount Developers
// SPDX-License-Identifier: GPL-3.0-or-later

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
