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
    collections::HashSet,
    fs,
    os::unix::fs::{FileTypeExt, MetadataExt},
    path::{Component, Path, PathBuf},
};

use anyhow::{Context, Result, bail};
use walkdir::WalkDir;

use super::common::build_managed_partitions;
use crate::{
    conf::config,
    core::{
        inventory::Module,
        ops::plan::{HymofsAddRule, HymofsMergeRule, MountPlan},
    },
    defs,
    domain::MountMode,
};

#[derive(Debug, Default)]
pub(super) struct CompiledRules {
    pub(super) add_rules: Vec<HymofsAddRule>,
    pub(super) merge_rules: Vec<HymofsMergeRule>,
    pub(super) hide_rules: Vec<String>,
}

fn resolve_path_for_hymofs_with_root(system_root: &Path, path: &Path) -> PathBuf {
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

fn normalize_path(path: &Path) -> PathBuf {
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

fn normalize_partition_root(path: &Path) -> PathBuf {
    match fs::read_link(path) {
        Ok(target) if target.is_absolute() => normalize_path(&target),
        Ok(target) => normalize_path(&path.parent().unwrap_or(Path::new("/")).join(target)),
        Err(_) => normalize_path(path),
    }
}

fn mirror_module_root(config: &config::Config, module: &Module) -> Result<PathBuf> {
    let module_root = config.hymofs.mirror_path.join(&module.id);
    if module_root.exists() {
        Ok(module_root)
    } else {
        bail!(
            "missing HymoFS mirror content for module {} at {}",
            module.id,
            module_root.display()
        )
    }
}

fn build_dtype(path: &Path) -> Result<(i32, bool)> {
    let metadata = fs::symlink_metadata(path).with_context(|| {
        format!(
            "failed to read metadata for hymofs source {}",
            path.display()
        )
    })?;
    let file_type = metadata.file_type();

    if file_type.is_char_device() && metadata.rdev() == 0 {
        return Ok((libc::DT_UNKNOWN as i32, true));
    }

    let d_type = if file_type.is_file() {
        libc::DT_REG as i32
    } else if file_type.is_symlink() {
        libc::DT_LNK as i32
    } else if file_type.is_dir() {
        libc::DT_DIR as i32
    } else if file_type.is_block_device() {
        libc::DT_BLK as i32
    } else if file_type.is_char_device() {
        libc::DT_CHR as i32
    } else if file_type.is_fifo() {
        libc::DT_FIFO as i32
    } else if file_type.is_socket() {
        libc::DT_SOCK as i32
    } else {
        libc::DT_UNKNOWN as i32
    };

    Ok((d_type, false))
}

pub(super) fn log_compiled_rule_summary(compiled: &CompiledRules, user_hide_paths: &[PathBuf]) {
    crate::scoped_log!(
        debug,
        "mount:hymofs",
        "compiled rules: add_rules={}, merge_rules={}, hide_rules={}, user_hide_rules={}",
        compiled.add_rules.len(),
        compiled.merge_rules.len(),
        compiled.hide_rules.len(),
        user_hide_paths.len()
    );
}

fn relative_mode(module: &Module, relative: &Path) -> MountMode {
    let relative_str = relative.to_string_lossy();
    module.rules.get_mode(relative_str.as_ref())
}

pub(super) fn compile_rules(
    modules: &[Module],
    plan: &MountPlan,
    config: &config::Config,
) -> Result<CompiledRules> {
    let system_root = Path::new("/");
    let managed_partitions = build_managed_partitions(config);
    let active_ids: HashSet<&str> = plan.hymofs_module_ids.iter().map(String::as_str).collect();
    let mut compiled = CompiledRules::default();
    let mut managed_partition_list: Vec<String> = managed_partitions.into_iter().collect();
    managed_partition_list.sort();

    for module in modules.iter().rev() {
        if !active_ids.contains(module.id.as_str()) {
            continue;
        }

        let module_root = mirror_module_root(config, module)?;
        let mut scanned_partition_roots: HashSet<PathBuf> = HashSet::new();

        for partition_name in &managed_partition_list {
            let partition_root = module_root.join(partition_name);
            if !partition_root.is_dir() {
                continue;
            }
            let normalized_partition_root = normalize_partition_root(&partition_root);
            if !scanned_partition_roots.insert(normalized_partition_root) {
                crate::scoped_log!(
                    debug,
                    "mount:hymofs",
                    "partition root dedupe: module={}, partition={}, root={}",
                    module.id,
                    partition_name,
                    partition_root.display()
                );
                continue;
            }

            let mut iterator = WalkDir::new(&partition_root)
                .follow_links(false)
                .into_iter();

            while let Some(entry_result) = iterator.next() {
                let entry = match entry_result {
                    Ok(entry) => entry,
                    Err(err) => {
                        crate::scoped_log!(
                            warn,
                            "mount:hymofs",
                            "walk failed: module={}, partition={}, error={}",
                            module.id,
                            partition_name,
                            err
                        );
                        continue;
                    }
                };

                if entry.depth() == 0 {
                    continue;
                }

                let path = entry.path();
                let relative = match path.strip_prefix(&module_root) {
                    Ok(relative) => relative,
                    Err(err) => {
                        crate::scoped_log!(
                            warn,
                            "mount:hymofs",
                            "relative path failed: module={}, path={}, error={}",
                            module.id,
                            path.display(),
                            err
                        );
                        continue;
                    }
                };

                if !matches!(relative_mode(module, relative), MountMode::Hymofs) {
                    continue;
                }

                if path
                    .file_name()
                    .is_some_and(|name| name == defs::REPLACE_DIR_FILE_NAME)
                {
                    continue;
                }

                let resolved_virtual_path =
                    resolve_path_for_hymofs_with_root(system_root, &Path::new("/").join(relative));
                let target_key = resolved_virtual_path.display().to_string();

                if entry.file_type().is_dir() {
                    if resolved_virtual_path.is_dir() {
                        compiled.merge_rules.push(HymofsMergeRule {
                            target: target_key,
                            source: path.to_path_buf(),
                        });
                        iterator.skip_current_dir();
                    }
                    continue;
                }

                if entry.file_type().is_symlink()
                    && resolved_virtual_path.exists()
                    && resolved_virtual_path.is_dir()
                {
                    crate::scoped_log!(
                        warn,
                        "mount:hymofs",
                        "symlink skip: module={}, path={}, reason=directory_target",
                        module.id,
                        resolved_virtual_path.display()
                    );
                    continue;
                }

                let (file_type, hide_only) = build_dtype(path)?;
                if hide_only {
                    compiled.hide_rules.push(target_key);
                    continue;
                }

                compiled.add_rules.push(HymofsAddRule {
                    target: target_key,
                    source: path.to_path_buf(),
                    file_type,
                });
            }
        }
    }

    Ok(compiled)
}
