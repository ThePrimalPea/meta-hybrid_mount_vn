// Copyright 2026 Hybrid Mount Developers
// SPDX-License-Identifier: GPL-3.0-or-later

use anyhow::Result;

use crate::conf::{
    cli::{Cli, Commands},
    cli_handlers,
};

pub fn run(cli: &Cli, command: &Commands) -> Result<()> {
    match command {
        Commands::GenConfig { output, force } => cli_handlers::handle_gen_config(output, *force),
        Commands::ShowConfig => cli_handlers::handle_show_config(cli),
        Commands::SaveConfig { payload } => cli_handlers::handle_save_config(payload),
        Commands::SaveModuleRules { module, payload } => {
            cli_handlers::handle_save_module_rules(module, payload)
        }
        Commands::Modules => cli_handlers::handle_modules(cli),
    }
}
