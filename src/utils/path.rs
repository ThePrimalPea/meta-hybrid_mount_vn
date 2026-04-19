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

use std::{
    fs,
    path::{Component, Path, PathBuf},
};

pub fn normalize_path(path: &Path) -> PathBuf {
    let mut normalized = PathBuf::new();
    let mut saw_root = false;

    for component in path.components() {
        match component {
            Component::RootDir => {
                normalized.push(Path::new("/"));
                saw_root = true;
            }
            Component::CurDir => {}
            Component::ParentDir => {
                let _ = normalized.pop();
                if saw_root && normalized.as_os_str().is_empty() {
                    normalized.push(Path::new("/"));
                }
            }
            Component::Normal(value) => normalized.push(value),
            Component::Prefix(prefix) => normalized.push(prefix.as_os_str()),
        }
    }

    if saw_root && normalized.as_os_str().is_empty() {
        PathBuf::from("/")
    } else {
        normalized
    }
}

pub fn resolve_link_path(path: &Path) -> PathBuf {
    match fs::read_link(path) {
        Ok(target) if target.is_absolute() => normalize_path(&target),
        Ok(target) => normalize_path(&path.parent().unwrap_or(Path::new("/")).join(target)),
        Err(_) => normalize_path(path),
    }
}

pub fn resolve_path_with_root(system_root: &Path, path: &Path) -> PathBuf {
    let virtual_path = if path.is_absolute() {
        path.to_path_buf()
    } else {
        Path::new("/").join(path)
    };

    let translated_path = if system_root == Path::new("/") {
        virtual_path.clone()
    } else {
        let relative = virtual_path.strip_prefix("/").unwrap_or(&virtual_path);
        system_root.join(relative)
    };

    let Some(parent) = translated_path.parent() else {
        return virtual_path;
    };

    let Some(filename) = translated_path.file_name() else {
        return virtual_path;
    };

    let mut current = parent.to_path_buf();
    let mut suffix = Vec::new();

    while current != system_root && !current.exists() {
        if let Some(name) = current.file_name() {
            suffix.push(name.to_os_string());
        }
        if !current.pop() {
            break;
        }
    }

    let mut resolved = if current.exists() {
        current
    } else {
        parent.to_path_buf()
    };

    for item in suffix.iter().rev() {
        resolved.push(item);
    }
    resolved.push(filename);

    if system_root == Path::new("/") {
        return resolved;
    }

    if let Ok(relative) = resolved.strip_prefix(system_root) {
        return Path::new("/").join(relative);
    }

    virtual_path
}
