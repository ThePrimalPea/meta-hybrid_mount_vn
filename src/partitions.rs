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

use std::{collections::HashSet, fs, path::Path};

use crate::defs;

fn partition_root_exists(name: &str) -> bool {
    fs::symlink_metadata(Path::new("/").join(name)).is_ok()
}

pub fn discover_partition_names(_moduledir: &Path, extra_partitions: &[String]) -> Vec<String> {
    let mut names = defs::MANAGED_PARTITIONS
        .iter()
        .copied()
        .filter(|partition| partition_root_exists(partition))
        .map(str::to_string)
        .collect::<Vec<_>>();

    names.extend(
        extra_partitions
            .iter()
            .filter(|partition| partition_root_exists(partition))
            .cloned(),
    );

    names.sort();
    names.dedup();
    names
}

pub fn managed_partition_names(moduledir: &Path, extra_partitions: &[String]) -> Vec<String> {
    discover_partition_names(moduledir, extra_partitions)
}

pub fn managed_partition_set(moduledir: &Path, extra_partitions: &[String]) -> HashSet<String> {
    managed_partition_names(moduledir, extra_partitions)
        .into_iter()
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_keep_existing_root_partitions() {
        let partitions = discover_partition_names(Path::new("/unused"), &[]);

        for name in &partitions {
            assert!(partition_root_exists(name));
        }
    }

    #[test]
    fn extra_partitions_require_existing_root() {
        let extras = vec![
            "tmp".to_string(),
            "__definitely_not_a_real_partition__".to_string(),
        ];

        let partitions = discover_partition_names(Path::new("/unused"), &extras);

        assert!(partitions.contains(&"tmp".to_string()));
        assert!(!partitions.contains(&"__definitely_not_a_real_partition__".to_string()));
    }
}
