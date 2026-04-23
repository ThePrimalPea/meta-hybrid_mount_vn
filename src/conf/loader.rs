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

use std::path::Path;

use anyhow::{Context, Result};

use crate::{
    conf::{cli::Cli, config::Config},
    defs,
};

fn load_default_config(allow_invalid_fallback: bool) -> Result<Config> {
    let default_path = Path::new(defs::CONFIG_FILE);
    if !default_path.exists() {
        return Ok(Config::default());
    }

    match Config::load_optional_from_file(default_path) {
        Ok(config) => Ok(config),
        Err(err) if allow_invalid_fallback => {
            crate::scoped_log!(
                warn,
                "config",
                "load_default failed, fallback=defaults: {:#}",
                err
            );
            Ok(Config::default())
        }
        Err(err) => Err(err).with_context(|| {
            format!(
                "Failed to load config from default path: {}",
                default_path.display()
            )
        }),
    }
}

pub fn load_config(cli: &Cli) -> Result<Config> {
    if let Some(config_path) = &cli.config {
        return Config::load_optional_from_file(config_path).with_context(|| {
            format!(
                "Failed to load config from custom path: {}",
                config_path.display()
            )
        });
    }

    load_default_config(true)
}

pub fn load_startup_config(cli: &Cli) -> Result<Config> {
    if let Some(config_path) = &cli.config {
        return Config::load_optional_from_file(config_path).with_context(|| {
            format!(
                "Failed to load config from custom path: {}",
                config_path.display()
            )
        });
    }

    load_default_config(false)
}
