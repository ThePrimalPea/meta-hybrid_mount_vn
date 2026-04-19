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
    fs::{self},
    io::{BufRead, BufReader},
    path::Path,
    sync::OnceLock,
};

use anyhow::Result;
use regex_lite::Regex;
use serde::Serialize;

use super::discovery;
use crate::{conf::config, core::runtime_state::RuntimeState, domain::ModuleRules};

static MODULE_PROP_REGEX: OnceLock<Regex> = OnceLock::new();

#[derive(Default)]
struct ModuleProp {
    name: String,
    version: String,
    author: String,
    description: String,
}

fn normalize_module_prop(module_id: &str, mut prop: ModuleProp) -> ModuleProp {
    if prop.name.trim().is_empty() {
        prop.name = module_id.to_string();
    }
    if prop.version.trim().is_empty() {
        prop.version = "unknown".to_string();
    }
    if prop.author.trim().is_empty() {
        prop.author = "unknown".to_string();
    }
    if prop.description.trim().is_empty() {
        prop.description = "No description".to_string();
    }
    prop
}

impl From<&Path> for ModuleProp {
    fn from(path: &Path) -> Self {
        let mut prop = ModuleProp::default();
        let re = MODULE_PROP_REGEX.get_or_init(|| {
            Regex::new(r"^([a-zA-Z0-9_.]+)=(.*)$").expect("Failed to compile module prop regex")
        });

        if let Ok(file) = fs::File::open(path) {
            for line in BufReader::new(file).lines().map_while(Result::ok) {
                if let Some(caps) = re.captures(line.trim()) {
                    let k = caps.get(1).map_or("", |m| m.as_str());
                    let v = caps.get(2).map_or("", |m| m.as_str());

                    match k {
                        "name" => prop.name = v.to_string(),
                        "version" => prop.version = v.to_string(),
                        "author" => prop.author = v.to_string(),
                        "description" => prop.description = v.to_string(),
                        _ => {}
                    }
                }
            }
        }
        prop
    }
}

#[derive(Serialize)]
struct ModuleInfo {
    id: String,
    name: String,
    version: String,
    author: String,
    description: String,
    mode: String,
    strategy: String,
    path: String,
    enabled: bool,
    is_mounted: bool,
    rules: ModuleRules,
}

impl ModuleInfo {
    fn new(module: discovery::Module, mounted_set: &HashSet<&str>) -> Self {
        let prop = normalize_module_prop(
            &module.id,
            ModuleProp::from(module.source_path.join("module.prop").as_path()),
        );

        Self {
            is_mounted: mounted_set.contains(module.id.as_str()),
            id: module.id,
            name: prop.name,
            version: prop.version,
            author: prop.author,
            description: prop.description,
            mode: module.rules.default_mode.as_module_mode_label().to_string(),
            strategy: module.rules.default_mode.as_strategy().to_string(),
            path: module.source_path.display().to_string(),
            enabled: true,
            rules: module.rules,
        }
    }
}

pub fn print_list(config: &config::Config) -> Result<()> {
    let modules = discovery::scan(&config.moduledir, config)?;
    let runtime_state = RuntimeState::load().unwrap_or_default();
    let mounted_ids = runtime_state.mounted_module_ids();

    let infos: Vec<ModuleInfo> = modules
        .into_iter()
        .map(|module| ModuleInfo::new(module, &mounted_ids))
        .collect();

    println!("{}", serde_json::to_string(&infos)?);

    Ok(())
}
