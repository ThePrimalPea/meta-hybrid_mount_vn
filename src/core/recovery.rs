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

use std::{error::Error as StdError, fmt};

use anyhow::Error;

#[derive(Debug, Clone, Copy)]
pub enum FailureStage {
    Sync,
    Execute,
}

impl fmt::Display for FailureStage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Sync => write!(f, "sync"),
            Self::Execute => write!(f, "execute"),
        }
    }
}

#[derive(Debug)]
pub struct ModuleStageFailure {
    pub stage: FailureStage,
    pub module_ids: Vec<String>,
    pub source: Error,
}

impl ModuleStageFailure {
    pub fn new(stage: FailureStage, module_ids: Vec<String>, source: Error) -> Self {
        Self {
            stage,
            module_ids,
            source,
        }
    }
}

impl fmt::Display for ModuleStageFailure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.module_ids.is_empty() {
            write!(
                f,
                "module stage failure during {}: {}",
                self.stage, self.source
            )
        } else {
            write!(
                f,
                "module stage failure during {} for [{}]: {}",
                self.stage,
                self.module_ids.join(", "),
                self.source
            )
        }
    }
}

impl StdError for ModuleStageFailure {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        Some(self.source.as_ref())
    }
}
