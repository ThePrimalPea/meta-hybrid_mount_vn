// Copyright 2026 https://github.com/Tools-cx-app/meta-magic_mount

mod utils;

use std::{
    collections::HashSet,
    error::Error as StdError,
    fmt, fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result, bail};
use rustix::mount::{
    MountFlags, MountPropagationFlags, UnmountFlags, mount, mount_bind, mount_change, mount_move,
    mount_remount, unmount,
};

#[cfg(any(target_os = "linux", target_os = "android"))]
use crate::mount::umount_mgr::send_umountable;
use crate::{
    mount::{
        magic_mount::utils::{clone_symlink, collect_module_files, mount_mirror},
        node::{Node, NodeFileType},
    },
    sys::fs::ensure_dir_exists,
};

#[derive(Debug)]
pub struct MagicMountModuleFailure {
    pub module_ids: Vec<String>,
    pub source: anyhow::Error,
}

impl MagicMountModuleFailure {
    pub fn new(module_ids: Vec<String>, source: anyhow::Error) -> Self {
        Self { module_ids, source }
    }
}

impl fmt::Display for MagicMountModuleFailure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.module_ids.is_empty() {
            write!(f, "magic mount module failure: {}", self.source)
        } else {
            write!(
                f,
                "magic mount module failure for [{}]: {}",
                self.module_ids.join(", "),
                self.source
            )
        }
    }
}

impl StdError for MagicMountModuleFailure {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        Some(self.source.as_ref())
    }
}

fn collect_module_ids(node: &Node, ids: &mut HashSet<String>) {
    if let Some(module_path) = &node.module_path
        && let Some(module_id) = crate::utils::extract_module_id(module_path)
    {
        ids.insert(module_id);
    }

    for child in node.children.values() {
        collect_module_ids(child, ids);
    }
}

fn infer_module_ids(node: &Node) -> Vec<String> {
    let mut ids = HashSet::new();
    collect_module_ids(node, &mut ids);
    let mut module_ids: Vec<String> = ids.into_iter().collect();
    module_ids.sort();
    module_ids
}

fn wrap_with_module_context(err: anyhow::Error, node: &Node) -> anyhow::Error {
    let module_ids = infer_module_ids(node);
    if module_ids.is_empty() {
        err
    } else {
        MagicMountModuleFailure::new(module_ids, err).into()
    }
}

#[derive(Debug, Default)]
struct MountStats {
    mounted_files: u32,
    ignored_files: u32,
    mounted_symlinks: u32,
}

impl MountStats {
    fn record_file(&mut self) {
        self.mounted_files += 1;
    }

    fn record_ignored(&mut self) {
        self.ignored_files += 1;
    }

    fn record_symlink(&mut self) {
        self.mounted_symlinks += 1;
    }
}

#[derive(Debug, Default)]
struct MountContext {
    stats: MountStats,
}

struct MagicMount {
    node: Node,
    path: PathBuf,
    work_dir_path: PathBuf,
    has_tmpfs: bool,
    #[cfg(any(target_os = "linux", target_os = "android"))]
    umount: bool,
}

impl MagicMount {
    fn new<P>(
        node: &Node,
        path: P,
        work_dir_path: P,
        has_tmpfs: bool,
        #[cfg(any(target_os = "linux", target_os = "android"))] umount: bool,
    ) -> Self
    where
        P: AsRef<Path>,
    {
        Self {
            path: path.as_ref().join(&node.name),
            work_dir_path: work_dir_path.as_ref().join(&node.name),
            node: node.clone(),
            has_tmpfs,
            #[cfg(any(target_os = "linux", target_os = "android"))]
            umount,
        }
    }

    fn do_mount(&mut self, context: &mut MountContext) -> Result<()> {
        match self.node.file_type {
            NodeFileType::Symlink => self.symlink(context),
            NodeFileType::RegularFile => self.regular_file(context),
            NodeFileType::Directory => self.directory(context),
            NodeFileType::Whiteout => {
                log::debug!("file {} is removed", self.path.display());
                Ok(())
            }
        }
    }
}

impl MagicMount {
    fn symlink(&self, context: &mut MountContext) -> Result<()> {
        if let Some(module_path) = &self.node.module_path {
            log::debug!(
                "create module symlink {} -> {}",
                module_path.display(),
                self.work_dir_path.display()
            );
            clone_symlink(module_path, &self.work_dir_path).with_context(|| {
                format!(
                    "create module symlink {} -> {}",
                    module_path.display(),
                    self.work_dir_path.display(),
                )
            })?;
            context.stats.record_symlink();
            Ok(())
        } else {
            bail!("cannot mount root symlink {}!", self.path.display());
        }
    }

    fn regular_file(&self, context: &mut MountContext) -> Result<()> {
        let target = if self.has_tmpfs {
            fs::File::create(&self.work_dir_path)?;
            &self.work_dir_path
        } else {
            &self.path
        };

        let Some(module_path) = self.node.module_path.as_ref() else {
            bail!("cannot mount root file {}!", self.path.display());
        };

        log::debug!(
            "mount module file {} -> {}",
            module_path.display(),
            self.work_dir_path.display()
        );

        mount_bind(module_path, target).with_context(|| {
            #[cfg(any(target_os = "linux", target_os = "android"))]
            if self.umount {
                let _ = send_umountable(target);
            }
            format!(
                "mount module file {} -> {}",
                module_path.display(),
                self.work_dir_path.display(),
            )
        })?;

        if let Err(e) = mount_remount(target, MountFlags::RDONLY | MountFlags::BIND, "") {
            log::warn!("make file {} ro: {e:#?}", target.display());
        }

        context.stats.record_file();
        Ok(())
    }

    fn directory(&mut self, context: &mut MountContext) -> Result<()> {
        let mut tmpfs = !self.has_tmpfs && self.node.replace && self.node.module_path.is_some();

        if !self.has_tmpfs && !tmpfs {
            for (name, node) in &mut self.node.children {
                let real_path = self.path.join(name);
                let need = match node.file_type {
                    NodeFileType::Symlink => true,
                    NodeFileType::Whiteout => real_path.exists(),
                    _ => {
                        if let Ok(metadata) = real_path.symlink_metadata() {
                            let file_type = NodeFileType::from(metadata.file_type());
                            file_type != self.node.file_type || file_type == NodeFileType::Symlink
                        } else {
                            true
                        }
                    }
                };
                if need {
                    if self.node.module_path.is_none() {
                        log::error!(
                            "cannot create tmpfs on {}, ignore: {name}",
                            self.path.display()
                        );
                        context.stats.record_ignored();
                        node.skip = true;
                        continue;
                    }
                    tmpfs = true;
                    break;
                }
            }
        }
        let has_tmpfs = tmpfs || self.has_tmpfs;

        if has_tmpfs {
            utils::tmpfs_skeleton(&self.path, &self.work_dir_path, &self.node)?;
        }

        if tmpfs {
            mount_bind(&self.work_dir_path, &self.work_dir_path).with_context(|| {
                format!(
                    "creating tmpfs for {} at {}",
                    self.path.display(),
                    self.work_dir_path.display(),
                )
            })?;
        }

        if self.path.exists() && !self.node.replace {
            self.mount_path(has_tmpfs, context)?;
        }

        if self.node.replace {
            if self.node.module_path.is_none() {
                bail!(
                    "dir {} is declared as replaced but it is root!",
                    self.path.display()
                );
            }
            log::debug!("dir {} is replaced", self.path.display());
        }

        for (name, node) in &self.node.children {
            if node.skip {
                continue;
            }

            if let Err(e) = {
                Self::new(
                    node,
                    &self.path,
                    &self.work_dir_path,
                    has_tmpfs,
                    #[cfg(any(target_os = "linux", target_os = "android"))]
                    self.umount,
                )
                .do_mount(context)
            }
            .with_context(|| format!("magic mount {}/{name}", self.path.display()))
            {
                if has_tmpfs {
                    return Err(wrap_with_module_context(e, node));
                }
                log::error!("mount child {}/{name} failed: {e:#?}", self.path.display());
            }
        }

        if tmpfs {
            log::debug!(
                "moving tmpfs {} -> {}",
                self.work_dir_path.display(),
                self.path.display()
            );

            if let Err(e) = mount_remount(
                &self.work_dir_path,
                MountFlags::RDONLY | MountFlags::BIND,
                "",
            ) {
                log::warn!("make dir {} ro: {e:#?}", self.path.display());
            }
            mount_move(&self.work_dir_path, &self.path).with_context(|| {
                format!(
                    "moving tmpfs {} -> {}",
                    self.work_dir_path.display(),
                    self.path.display()
                )
            })?;
            if let Err(e) = mount_change(&self.path, MountPropagationFlags::PRIVATE) {
                log::warn!("make dir {} private: {e:#?}", self.path.display());
            }

            #[cfg(any(target_os = "linux", target_os = "android"))]
            if self.umount {
                let _ = send_umountable(&self.path);
            }
        }
        Ok(())
    }
}

impl MagicMount {
    fn mount_path(&mut self, has_tmpfs: bool, context: &mut MountContext) -> Result<()> {
        for entry in self.path.read_dir()?.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            let mut failed_node: Option<Node> = None;
            let result = {
                if let Some(node) = self.node.children.remove(&name) {
                    if node.skip {
                        continue;
                    }
                    failed_node = Some(node.clone());

                    Self::new(
                        &node,
                        &self.path,
                        &self.work_dir_path,
                        has_tmpfs,
                        #[cfg(any(target_os = "linux", target_os = "android"))]
                        self.umount,
                    )
                    .do_mount(context)
                    .with_context(|| format!("magic mount {}/{name}", self.path.display()))
                } else if has_tmpfs {
                    mount_mirror(&self.path, &self.work_dir_path, &entry)
                        .with_context(|| format!("mount mirror {}/{name}", self.path.display()))
                } else {
                    Ok(())
                }
            };

            if let Err(e) = result {
                if has_tmpfs {
                    if let Some(node) = failed_node.as_ref() {
                        return Err(wrap_with_module_context(e, node));
                    }
                    return Err(e);
                }
                log::error!("mount child {}/{name} failed: {e:#?}", self.path.display());
            }
        }

        Ok(())
    }
}

pub fn magic_mount<P>(
    tmp_path: P,
    module_dir: &Path,
    mount_source: &str,
    extra_partitions: &[String],
    need_id: HashSet<String>,
    #[cfg(any(target_os = "linux", target_os = "android"))] umount: bool,
    #[cfg(not(any(target_os = "linux", target_os = "android")))] _umount: bool,
) -> Result<()>
where
    P: AsRef<Path>,
{
    let mut context = MountContext::default();

    if let Some(root) = collect_module_files(module_dir, extra_partitions, need_id)? {
        log::debug!("collected: {root:?}");
        let tmp_root = tmp_path.as_ref();
        let tmp_dir = tmp_root.join("workdir");
        ensure_dir_exists(&tmp_dir)?;

        mount(mount_source, &tmp_dir, "tmpfs", MountFlags::empty(), None).context("mount tmp")?;
        mount_change(&tmp_dir, MountPropagationFlags::PRIVATE).context("make tmp private")?;

        let ret = MagicMount::new(
            &root,
            Path::new("/"),
            tmp_dir.as_path(),
            false,
            #[cfg(any(target_os = "linux", target_os = "android"))]
            umount,
        )
        .do_mount(&mut context)
        .map_err(|e| wrap_with_module_context(e, &root));

        if let Err(e) = unmount(&tmp_dir, UnmountFlags::DETACH) {
            log::error!("failed to umount tmp {e}");
        }
        fs::remove_dir(tmp_dir).ok();

        log::info!(
            "mounted files: {}, mounted symlinks: {}, ignored files: {}",
            context.stats.mounted_files,
            context.stats.mounted_symlinks,
            context.stats.ignored_files
        );

        ret
    } else {
        log::info!("no modules to mount, skipping!");
        Ok(())
    }
}
