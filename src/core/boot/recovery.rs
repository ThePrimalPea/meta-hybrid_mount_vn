// Copyright 2026 Hybrid Mount Developers
// SPDX-License-Identifier: GPL-3.0-or-later

use std::{
    collections::{HashMap, HashSet},
    fs::OpenOptions,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};

use crate::{
    conf::config::Config,
    core::{MountController, recovery::ModuleStageFailure},
    defs, sys, utils,
};

pub fn run(config: Config) -> Result<()> {
    let module_dirs = list_module_dirs(&config.moduledir)?;
    let max_restarts = module_dirs.len().saturating_add(1);
    let mut restart_round = 0usize;
    let mut auto_skipped = HashSet::new();
    let mut unattributed_retry_used = false;
    log::info!(
        "[stage:recovery] initialized: module_candidates={}, restart_limit={}",
        module_dirs.len(),
        max_restarts
    );

    loop {
        let attempt = restart_round + 1;
        let mnt_base = utils::get_mnt();
        sys::fs::ensure_dir_exists(&mnt_base)?;
        log::info!(
            "[stage:recovery] attempt {}/{} started with runtime mount {}",
            attempt,
            max_restarts,
            mnt_base.display()
        );

        let daemon_result = (|| -> Result<()> {
            MountController::new(config.clone(), &mnt_base)
                .init_storage(&mnt_base)
                .context("Failed to initialize storage")?
                .scan_and_sync()
                .context("Failed to scan and sync modules")?
                .generate_plan()
                .context("Failed to generate mount plan")?
                .execute()
                .context("Failed to execute mount plan")?
                .finalize()
                .context("Failed to finalize boot sequence")?;
            Ok(())
        })();

        match daemon_result {
            Ok(()) => {
                log_completion(&auto_skipped);
                return Ok(());
            }
            Err(e) => {
                if let Some(module_failure) = e.downcast_ref::<ModuleStageFailure>() {
                    if module_failure.module_ids.is_empty() {
                        log::error!(
                            "[stage:recovery] {} failure did not include module ids",
                            module_failure.stage
                        );
                        if !unattributed_retry_used {
                            unattributed_retry_used = true;
                            restart_round += 1;
                            if restart_round > max_restarts {
                                return abort_on_retry_limit(max_restarts);
                            }
                            log::warn!(
                                "[event:recovery_retry_unattributed] stage={} next_attempt={}/{}",
                                module_failure.stage,
                                restart_round + 1,
                                max_restarts
                            );
                            continue;
                        }
                        log::error!(
                            "[event:recovery_retry_unattributed] exhausted=true stage={}",
                            module_failure.stage
                        );
                    } else {
                        log::warn!(
                            "[stage:recovery] detected {} failure for modules: {}",
                            module_failure.stage,
                            module_failure.module_ids.join(", ")
                        );
                    }

                    let action = mark_failed_modules(
                        &module_failure.module_ids,
                        &module_dirs,
                        &mut auto_skipped,
                    )?;

                    if !action.already_marked.is_empty() {
                        log::debug!(
                            "[stage:recovery] already marked modules ignored: {}",
                            action.already_marked.join(", ")
                        );
                    }
                    if !action.unknown_modules.is_empty() {
                        log::error!(
                            "[event:recovery_unknown_modules] stage={} attempt={}/{} modules={}",
                            module_failure.stage,
                            attempt,
                            max_restarts,
                            action.unknown_modules.join(",")
                        );
                    }

                    if !action.newly_marked.is_empty() {
                        restart_round += 1;
                        log::warn!(
                            "[event:recovery_mark_skip] stage={} attempt={}/{} modules={}",
                            module_failure.stage,
                            attempt,
                            max_restarts,
                            action.newly_marked.join(",")
                        );
                        if restart_round > max_restarts {
                            return abort_on_retry_limit(max_restarts);
                        }
                        log::info!(
                            "[event:recovery_restart] next_attempt={}/{}",
                            restart_round + 1,
                            max_restarts
                        );
                        continue;
                    }

                    log::error!(
                        "[stage:recovery] no newly marked modules for {} failure; aborting to avoid retry loop",
                        module_failure.stage
                    );
                }

                let err_msg = format!("{:#}", e).replace('\n', " -> ");
                log::error!("[stage:recovery] unrecoverable error: {}", err_msg);
                crate::core::inventory::model::update_crash_description(&err_msg);
                return Err(e);
            }
        }
    }
}

struct MarkOutcome {
    newly_marked: Vec<String>,
    already_marked: Vec<String>,
    unknown_modules: Vec<String>,
}

fn log_completion(auto_skipped: &HashSet<String>) {
    if auto_skipped.is_empty() {
        log::info!("[stage:recovery] completed without auto-skip");
        return;
    }

    let mut skipped: Vec<String> = auto_skipped.iter().cloned().collect();
    skipped.sort();
    log::warn!(
        "[stage:recovery] completed after auto-skipping modules: {}",
        skipped.join(", ")
    );
}

fn abort_on_retry_limit(max_restarts: usize) -> Result<()> {
    let loop_error = anyhow::anyhow!(
        "Auto-recovery reached restart limit ({} rounds), aborting to avoid loop",
        max_restarts
    );
    log::error!("[stage:recovery] {}", loop_error);
    crate::core::inventory::model::update_crash_description(&loop_error.to_string());
    Err(loop_error)
}

fn mark_failed_modules(
    module_ids: &[String],
    module_dirs: &HashMap<String, PathBuf>,
    auto_skipped: &mut HashSet<String>,
) -> Result<MarkOutcome> {
    let mut newly_marked = Vec::new();
    let mut already_marked = Vec::new();
    let mut unknown_modules = Vec::new();

    for module_id in module_ids {
        if auto_skipped.contains(module_id) {
            already_marked.push(module_id.clone());
            continue;
        }
        if let Some(module_dir) = module_dirs.get(module_id) {
            create_skip_mount_marker(module_dir)?;
            auto_skipped.insert(module_id.clone());
            newly_marked.push(module_id.clone());
        } else {
            unknown_modules.push(module_id.clone());
        }
    }

    Ok(MarkOutcome {
        newly_marked,
        already_marked,
        unknown_modules,
    })
}

fn list_module_dirs(base: &Path) -> Result<HashMap<String, PathBuf>> {
    let mut modules = HashMap::new();
    if !base.exists() {
        return Ok(modules);
    }

    for entry in std::fs::read_dir(base)?.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let id = entry.file_name().to_string_lossy().to_string();
        if crate::core::inventory::is_reserved_module_dir(&id) {
            continue;
        }
        modules.insert(id, path);
    }

    Ok(modules)
}

fn create_skip_mount_marker(module_dir: &Path) -> Result<()> {
    let marker = module_dir.join(defs::SKIP_MOUNT_FILE_NAME);
    log::info!(
        "[stage:recovery] creating skip marker at {}",
        marker.display()
    );
    OpenOptions::new()
        .create(true)
        .truncate(false)
        .write(true)
        .open(&marker)
        .with_context(|| format!("Failed to create {}", marker.display()))?;
    log::debug!(
        "[stage:recovery] skip marker ready for module dir {}",
        module_dir.display()
    );
    Ok(())
}
