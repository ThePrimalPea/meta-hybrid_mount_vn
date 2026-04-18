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
#[cfg(any(target_os = "linux", target_os = "android"))]
use std::process::{Command, Output};

use anyhow::Result;
#[cfg(any(target_os = "linux", target_os = "android"))]
use anyhow::{Context, bail};
#[cfg(any(target_os = "linux", target_os = "android"))]
use ksu::NukeExt4Sysfs;
#[cfg(any(target_os = "linux", target_os = "android"))]
use procfs::process::Process;

#[cfg(any(target_os = "linux", target_os = "android"))]
fn probe_ext4_procfs_node(path: &Path) -> Result<Option<std::path::PathBuf>> {
    let path_str = path
        .to_str()
        .context("nuke target path contains invalid utf-8")?;

    let process = Process::myself().context("failed to open self procfs handle")?;
    let mountinfo = process.mountinfo().context("failed to read mountinfo")?;
    let mount = mountinfo
        .into_iter()
        .find(|m| m.mount_point.to_string_lossy() == path_str)
        .context("nuke target is not a mount point")?;

    if mount.fs_type != "ext4" {
        bail!(
            "nuke target is not ext4: path={}, fs_type={}",
            path.display(),
            mount.fs_type
        );
    }

    let source_id = mount
        .mount_source
        .as_ref()
        .and_then(|s| {
            let source = s.as_str();
            source
                .trim()
                .rsplit('/')
                .next()
                .map(std::string::ToString::to_string)
        })
        .filter(|s| !s.is_empty())
        .context("unable to infer ext4 procfs node from mount source")?;

    Ok(Some(Path::new("/proc/fs/ext4").join(source_id)))
}

#[cfg(any(target_os = "linux", target_os = "android"))]
fn execute_ksu_nuke(path: &Path) -> Result<()> {
    let mut nuke = NukeExt4Sysfs::new();
    nuke.add(path);
    nuke.execute()
}

#[cfg(any(target_os = "linux", target_os = "android"))]
fn apatch_nuke_strict_verify() -> bool {
    std::env::var("HYBRID_MOUNT_APATCH_NUKE_STRICT_VERIFY")
        .ok()
        .map(|value| {
            matches!(
                value.trim().to_ascii_lowercase().as_str(),
                "1" | "true" | "yes" | "on"
            )
        })
        .unwrap_or(false)
}

#[cfg(any(target_os = "linux", target_os = "android"))]
fn execute_apatch_nuke(path: &Path) -> Result<()> {
    let kp_bin = std::env::var("HYBRID_MOUNT_APATCH_KP_BIN")
        .unwrap_or_else(|_| "/data/adb/ap/bin/kptools".to_string());
    if !Path::new(&kp_bin).exists() {
        bail!("apatch kp tool not found: {kp_bin}");
    }

    let kpm_module = std::env::var("HYBRID_MOUNT_APATCH_KPM_MODULE")
        .unwrap_or_else(|_| format!("{}/kpm/nuke_ext4_sysfs.kpm", crate::defs::HYBRID_MOUNT_DIR));
    if !Path::new(&kpm_module).exists() {
        bail!("apatch kpm module not found: {kpm_module}");
    }
    let kpm_id =
        std::env::var("HYBRID_MOUNT_APATCH_KPM_ID").unwrap_or_else(|_| "nuke_ext4_sysfs".into());
    let call_mode =
        std::env::var("HYBRID_MOUNT_APATCH_KPM_CALL_MODE").unwrap_or_else(|_| "control".into());
    let strict_verify = apatch_nuke_strict_verify();
    let procfs_node = probe_ext4_procfs_node(path).ok().flatten();
    let before_exists = procfs_node.as_ref().is_some_and(|node| node.exists());

    let load_output = Command::new(&kp_bin)
        .args(["kpm", "load", &kpm_module])
        .output()
        .with_context(|| format!("failed to load kpm module with {kp_bin}"))?;
    if !load_output.status.success() {
        bail!(
            "kpm load failed: module={kpm_module}, code={:?}, output={}",
            load_output.status.code(),
            format_output(&load_output)
        );
    }

    let path_str = path.to_string_lossy().to_string();
    let call_output = if call_mode.eq_ignore_ascii_case("nr") {
        let nr = std::env::var("HYBRID_MOUNT_APATCH_KPM_UNUSED_NR")
            .context("HYBRID_MOUNT_APATCH_KPM_UNUSED_NR is required when call mode is 'nr'")?;
        let _ = nr
            .parse::<u32>()
            .with_context(|| format!("invalid unused nr value: {nr}"))?;
        Command::new(&kp_bin)
            .args(["kpm", "call", &nr, &path_str])
            .output()
            .with_context(|| format!("failed to call kpm unused nr with {kp_bin}"))
    } else {
        let control_name = std::env::var("HYBRID_MOUNT_APATCH_KPM_CONTROL")
            .unwrap_or_else(|_| "nuke_ext4_sysfs".to_string());
        if control_name
            .chars()
            .any(|c| !(c.is_ascii_alphanumeric() || c == '_' || c == '-' || c == '.'))
        {
            bail!("invalid kpm control name: {control_name}");
        }
        Command::new(&kp_bin)
            .args(["kpm", "control", &control_name, &path_str])
            .output()
            .with_context(|| format!("failed to call kpm control with {kp_bin}"))
    }?;

    let unload_output = Command::new(&kp_bin)
        .args(["kpm", "unload", &kpm_id])
        .output()
        .with_context(|| format!("failed to unload kpm module with {kp_bin}"))?;
    if !unload_output.status.success() {
        crate::scoped_log!(
            warn,
            "nuke",
            "kpm unload failed: module={}, code={:?}, output={}",
            kpm_id,
            unload_output.status.code(),
            format_output(&unload_output)
        );
    }

    let call_rc = extract_kpm_rc(&call_output);
    if !call_output.status.success() {
        bail!(
            "kpm invoke failed: mode={call_mode}, code={:?}, output={}",
            call_output.status.code(),
            format_output(&call_output)
        );
    }
    if let Some(rc) = call_rc
        && rc < 0
    {
        if !strict_verify && rc == -(libc::EEXIST as i64) {
            crate::scoped_log!(
                warn,
                "nuke",
                "kpm invoke returned -EEXIST in best-effort mode: mode={}, rc={}, output={}",
                call_mode,
                rc,
                format_output(&call_output)
            );
        } else {
            bail!(
                "kpm invoke reported failure: mode={call_mode}, rc={rc}, output={}",
                format_output(&call_output)
            );
        }
    }

    if let Some(node) = procfs_node {
        let after_exists = node.exists();
        if after_exists {
            if strict_verify {
                bail!("procfs node still present after nuke: {}", node.display());
            }
            crate::scoped_log!(
                warn,
                "nuke",
                "procfs node still present after nuke (best-effort mode): path={}, before_exists={}, after_exists={}",
                node.display(),
                before_exists,
                after_exists
            );
        } else {
            crate::scoped_log!(
                debug,
                "nuke",
                "procfs node verification passed: path={}, before_exists={}, after_exists={}",
                node.display(),
                before_exists,
                after_exists
            );
        }
    }

    Ok(())
}

#[cfg(any(target_os = "linux", target_os = "android"))]
fn extract_kpm_rc(output: &Output) -> Option<i64> {
    [
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    ]
    .into_iter()
    .find_map(|text| {
        text.split_whitespace().find_map(|token| {
            token.strip_prefix("rc=").and_then(|value| {
                value
                    .trim_end_matches(|c: char| !matches!(c, '-' | '0'..='9'))
                    .parse::<i64>()
                    .ok()
            })
        })
    })
}

#[cfg(any(target_os = "linux", target_os = "android"))]
fn format_output(output: &Output) -> String {
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();

    match (stdout.is_empty(), stderr.is_empty()) {
        (true, true) => "<empty>".to_string(),
        (false, true) => format!("stdout={stdout}"),
        (true, false) => format!("stderr={stderr}"),
        (false, false) => format!("stdout={stdout}; stderr={stderr}"),
    }
}

pub fn nuke_path(path: &Path) -> Result<()> {
    #[cfg(any(target_os = "linux", target_os = "android"))]
    {
        let result = if ksu::version().is_some() {
            execute_ksu_nuke(path)
        } else {
            execute_apatch_nuke(path)
        };

        match result {
            Ok(()) => {
                crate::scoped_log!(debug, "nuke", "execute success: path={}", path.display());
                Ok(())
            }
            Err(e) => {
                crate::scoped_log!(
                    warn,
                    "nuke",
                    "execute failed: path={}, error={:#}",
                    path.display(),
                    e
                );
                Err(e)
            }
        }
    }

    #[cfg(not(any(target_os = "linux", target_os = "android")))]
    {
        let _ = path;
        Ok(())
    }
}
