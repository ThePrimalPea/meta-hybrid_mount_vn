// Copyright (C) 2026 YuzakiKokuban <heibanbaize@gmail.com>
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use anyhow::Result;

use crate::{
    conf::{
        cli::{
            ApiCommands, Cli, Commands, HideCommands, KasumiCmdlineCommands, KasumiCommands,
            KasumiHideUidsCommands, KasumiKstatCommands, KasumiMapsCommands, KasumiRuleCommands,
            KasumiUnameCommands, LkmCommands,
        },
        cli_handlers,
    },
    core::api,
};

fn run_api_command<F>(f: F) -> Result<()>
where
    F: FnOnce() -> Result<()>,
{
    match f() {
        Ok(()) => Ok(()),
        Err(err) => {
            api::print_json_error(&err);
            Ok(())
        }
    }
}

pub fn run(cli: &Cli, command: &Commands) -> Result<()> {
    let _ = crate::utils::init_logging();

    match command {
        Commands::GenConfig { output, force } => cli_handlers::handle_gen_config(output, *force),
        Commands::ShowConfig => cli_handlers::handle_show_config(cli),
        Commands::SaveConfig { payload } => cli_handlers::handle_save_config(cli, payload),
        Commands::SaveFullConfig { payload } => cli_handlers::handle_save_full_config(cli, payload),
        Commands::SaveModuleRules { module, payload } => {
            cli_handlers::handle_save_module_rules(cli, module, payload)
        }
        Commands::SaveAllModuleRules { payload } => {
            cli_handlers::handle_save_all_module_rules(cli, payload)
        }
        Commands::Modules => cli_handlers::handle_modules(cli),
        Commands::State => cli_handlers::handle_state(),
        Commands::Logs { lines } => cli_handlers::handle_logs(*lines),
        Commands::Api { command } => run_api_command(|| match command {
            ApiCommands::System => cli_handlers::handle_api_system(cli),
            ApiCommands::Storage => cli_handlers::handle_api_storage(),
            ApiCommands::MountStats => cli_handlers::handle_api_mount_stats(),
            ApiCommands::MountTopology => cli_handlers::handle_api_mount_topology(cli),
            ApiCommands::Partitions => cli_handlers::handle_api_partitions(cli),
            ApiCommands::Lkm => cli_handlers::handle_api_lkm(cli),
            ApiCommands::Features => cli_handlers::handle_api_features(),
            ApiCommands::Hooks => cli_handlers::handle_api_hooks(cli),
        }),
        Commands::Lkm { command } => match command {
            LkmCommands::Load => cli_handlers::handle_lkm_load(cli),
            LkmCommands::Unload => cli_handlers::handle_lkm_unload(cli),
            LkmCommands::Status => cli_handlers::handle_lkm_status(cli),
            LkmCommands::SetAutoload { state } => {
                cli_handlers::handle_lkm_set_autoload(cli, state.enabled())
            }
            LkmCommands::SetKmi { kmi } => cli_handlers::handle_lkm_set_kmi(cli, kmi),
            LkmCommands::ClearKmi => cli_handlers::handle_lkm_clear_kmi(cli),
        },
        Commands::Hide { command } => match command {
            HideCommands::List => cli_handlers::handle_hide_list(),
            HideCommands::Add { path } => cli_handlers::handle_hide_add(cli, path),
            HideCommands::Remove { path } => cli_handlers::handle_hide_remove(path),
            HideCommands::Apply => cli_handlers::handle_hide_apply(cli),
        },
        Commands::Kasumi { command } => match command {
            KasumiCommands::Status => cli_handlers::handle_kasumi_status(cli),
            KasumiCommands::List => cli_handlers::handle_kasumi_list(cli),
            KasumiCommands::Version => cli_handlers::handle_kasumi_version(cli),
            KasumiCommands::Features => cli_handlers::handle_kasumi_features(),
            KasumiCommands::Hooks => cli_handlers::handle_kasumi_hooks(),
            KasumiCommands::Clear => cli_handlers::handle_kasumi_clear(),
            KasumiCommands::ReleaseConnection => cli_handlers::handle_kasumi_release_connection(),
            KasumiCommands::InvalidateCache => cli_handlers::handle_kasumi_invalidate_cache(),
            KasumiCommands::FixMounts => cli_handlers::handle_kasumi_fix_mounts(),
            KasumiCommands::Enable => cli_handlers::handle_kasumi_set_enabled(cli, true),
            KasumiCommands::Disable => cli_handlers::handle_kasumi_set_enabled(cli, false),
            KasumiCommands::Hidexattr { state } => {
                cli_handlers::handle_kasumi_set_hidexattr(cli, state.enabled())
            }
            KasumiCommands::SetMirror { path } => cli_handlers::handle_kasumi_set_mirror(cli, path),
            KasumiCommands::Debug { state } => {
                cli_handlers::handle_kasumi_set_debug(cli, state.enabled())
            }
            KasumiCommands::Stealth { state } => {
                cli_handlers::handle_kasumi_set_stealth(cli, state.enabled())
            }
            KasumiCommands::MountHide {
                state,
                path_pattern,
            } => cli_handlers::handle_kasumi_set_mount_hide(
                cli,
                state.enabled(),
                path_pattern.as_deref(),
            ),
            KasumiCommands::MapsSpoof { state } => {
                cli_handlers::handle_kasumi_set_maps_spoof(cli, state.enabled())
            }
            KasumiCommands::StatfsSpoof {
                state,
                path,
                f_type,
            } => cli_handlers::handle_kasumi_set_statfs_spoof(
                cli,
                state.enabled(),
                path.as_deref(),
                *f_type,
            ),
            KasumiCommands::Uname { command } => match command {
                KasumiUnameCommands::Set {
                    sysname,
                    nodename,
                    release,
                    version,
                    machine,
                    domainname,
                } => cli_handlers::handle_kasumi_set_uname(
                    cli,
                    sysname.as_deref(),
                    nodename.as_deref(),
                    release.as_deref(),
                    version.as_deref(),
                    machine.as_deref(),
                    domainname.as_deref(),
                ),
                KasumiUnameCommands::Clear => cli_handlers::handle_kasumi_clear_uname(cli),
            },
            KasumiCommands::Cmdline { command } => match command {
                KasumiCmdlineCommands::Set { value } => {
                    cli_handlers::handle_kasumi_set_cmdline(cli, value)
                }
                KasumiCmdlineCommands::Clear => cli_handlers::handle_kasumi_clear_cmdline(cli),
            },
            KasumiCommands::HideUids { command } => match command {
                KasumiHideUidsCommands::Set { uids } => {
                    cli_handlers::handle_kasumi_set_hide_uids(cli, uids)
                }
                KasumiHideUidsCommands::Clear => cli_handlers::handle_kasumi_clear_hide_uids(cli),
            },
            KasumiCommands::Maps { command } => match command {
                KasumiMapsCommands::Add {
                    target_ino,
                    target_dev,
                    spoofed_ino,
                    spoofed_dev,
                    path,
                } => cli_handlers::handle_kasumi_add_maps_rule(
                    cli,
                    *target_ino,
                    *target_dev,
                    *spoofed_ino,
                    *spoofed_dev,
                    path,
                ),
                KasumiMapsCommands::Clear => cli_handlers::handle_kasumi_clear_maps_rules(cli),
            },
            KasumiCommands::Kstat { command } => match command {
                KasumiKstatCommands::Upsert {
                    target_ino,
                    target_path,
                    spoofed_ino,
                    spoofed_dev,
                    spoofed_nlink,
                    spoofed_size,
                    spoofed_atime_sec,
                    spoofed_atime_nsec,
                    spoofed_mtime_sec,
                    spoofed_mtime_nsec,
                    spoofed_ctime_sec,
                    spoofed_ctime_nsec,
                    spoofed_blksize,
                    spoofed_blocks,
                    is_static,
                } => cli_handlers::handle_kasumi_upsert_kstat_rule(
                    cli,
                    *target_ino,
                    target_path,
                    *spoofed_ino,
                    *spoofed_dev,
                    *spoofed_nlink,
                    *spoofed_size,
                    *spoofed_atime_sec,
                    *spoofed_atime_nsec,
                    *spoofed_mtime_sec,
                    *spoofed_mtime_nsec,
                    *spoofed_ctime_sec,
                    *spoofed_ctime_nsec,
                    *spoofed_blksize,
                    *spoofed_blocks,
                    *is_static,
                ),
                KasumiKstatCommands::ClearConfig => {
                    cli_handlers::handle_kasumi_clear_kstat_rules_config(cli)
                }
            },
            KasumiCommands::Rule { command } => match command {
                KasumiRuleCommands::Add {
                    target,
                    source,
                    file_type,
                } => cli_handlers::handle_kasumi_rule_add(cli, target, source, *file_type),
                KasumiRuleCommands::Merge { target, source } => {
                    cli_handlers::handle_kasumi_rule_merge(cli, target, source)
                }
                KasumiRuleCommands::Hide { path } => {
                    cli_handlers::handle_kasumi_rule_hide(cli, path)
                }
                KasumiRuleCommands::Delete { path } => {
                    cli_handlers::handle_kasumi_rule_delete(cli, path)
                }
                KasumiRuleCommands::AddDir {
                    target_base,
                    source_dir,
                } => cli_handlers::handle_kasumi_rule_add_dir(cli, target_base, source_dir),
                KasumiRuleCommands::RemoveDir {
                    target_base,
                    source_dir,
                } => cli_handlers::handle_kasumi_rule_remove_dir(cli, target_base, source_dir),
            },
        },
    }
}
