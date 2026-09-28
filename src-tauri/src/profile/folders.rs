//! User-defined folders for organizing the custom mod order.
//!
//! The layout is stored in a sidecar file in the profile directory instead of the database,
//! so the database schema stays compatible with upstream Gale.
//!
//! `Profile::mods` stays the source of truth for which mods are installed. Whenever the layout
//! is changed, `Profile::mods` is reordered to match the flattened layout, so the custom sort
//! order (and everything else that relies on it) stays consistent.

use std::{
    collections::{HashMap, HashSet},
    fs,
    path::{Path, PathBuf},
};

use eyre::{Context, Result};
use serde::{Deserialize, Serialize};
use tracing::warn;
use uuid::Uuid;

use super::Profile;
use crate::util::fs::{JsonStyle, write_json};

pub const FILE_NAME: &str = "gale_mod_folders.json";
const FILE_VERSION: u32 = 1;
const MAX_NAME_LEN: usize = 100;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase", tag = "type")]
pub enum LayoutEntry {
    Mod { uuid: Uuid },
    Folder(ModFolder),
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ModFolder {
    pub id: Uuid,
    pub name: String,
    #[serde(default)]
    pub collapsed: bool,
    #[serde(default)]
    pub mods: Vec<Uuid>,
}

#[derive(Serialize, Deserialize)]
struct LayoutFile {
    version: u32,
    entries: Vec<LayoutEntry>,
}

fn file_path(profile_dir: &Path) -> PathBuf {
    profile_dir.join(FILE_NAME)
}

/// Reads the stored layout of a profile. Returns an empty layout if there is none.
pub fn load(profile_dir: &Path) -> Vec<LayoutEntry> {
    let path = file_path(profile_dir);

    if !path.exists() {
        return Vec::new();
    }

    match crate::util::fs::read_json::<LayoutFile>(&path) {
        Ok(file) => file.entries,
        Err(err) => {
            warn!(
                "failed to read mod folders from {}: {:#}",
                path.display(),
                err
            );
            Vec::new()
        }
    }
}

/// Writes the layout to the profile directory, or removes the file if there are no folders.
///
/// The file is written to a temporary path and then renamed, since duplicated profiles
/// may share the file through a hard link.
pub fn save(profile_dir: &Path, entries: &[LayoutEntry]) -> Result<()> {
    let path = file_path(profile_dir);

    if !has_folders(entries) {
        if path.exists() {
            fs::remove_file(&path).context("failed to remove mod folders file")?;
        }
        return Ok(());
    }

    let temp_path = path.with_extension("json.tmp");
    let file = LayoutFile {
        version: FILE_VERSION,
        entries: entries.to_vec(),
    };

    write_json(&temp_path, &file, JsonStyle::Pretty).context("failed to write mod folders")?;
    fs::rename(&temp_path, &path).context("failed to replace mod folders file")?;

    Ok(())
}

fn has_folders(entries: &[LayoutEntry]) -> bool {
    entries
        .iter()
        .any(|entry| matches!(entry, LayoutEntry::Folder(_)))
}

fn flatten(entries: &[LayoutEntry]) -> impl Iterator<Item = Uuid> + '_ {
    entries.iter().flat_map(|entry| match entry {
        LayoutEntry::Mod { uuid } => std::slice::from_ref(uuid).iter().copied(),
        LayoutEntry::Folder(folder) => folder.mods.iter().copied(),
    })
}

fn entry_contains(entry: &LayoutEntry, uuid: Uuid) -> bool {
    match entry {
        LayoutEntry::Mod { uuid: other } => *other == uuid,
        LayoutEntry::Folder(folder) => folder.mods.contains(&uuid),
    }
}

/// Makes a layout valid for the given (ordered) list of installed mods:
///
/// - drops mods that aren't installed, duplicate mods and duplicate folders,
/// - inserts installed mods that are missing from the layout right after the
///   mod that precedes them in `installed`.
fn reconcile(stored: &[LayoutEntry], installed: &[Uuid]) -> Vec<LayoutEntry> {
    let installed_set: HashSet<Uuid> = installed.iter().copied().collect();
    let mut seen = HashSet::new();
    let mut folder_ids = HashSet::new();

    let mut result = Vec::with_capacity(stored.len());

    for entry in stored {
        match entry {
            LayoutEntry::Mod { uuid } => {
                if installed_set.contains(uuid) && seen.insert(*uuid) {
                    result.push(LayoutEntry::Mod { uuid: *uuid });
                }
            }
            LayoutEntry::Folder(folder) => {
                if !folder_ids.insert(folder.id) {
                    continue;
                }

                let mods = folder
                    .mods
                    .iter()
                    .filter(|uuid| installed_set.contains(uuid) && seen.insert(**uuid))
                    .copied()
                    .collect();

                let mut name: String = folder.name.trim().chars().take(MAX_NAME_LEN).collect();
                if name.is_empty() {
                    name = "Folder".to_owned();
                }

                result.push(LayoutEntry::Folder(ModFolder {
                    id: folder.id,
                    name,
                    collapsed: folder.collapsed,
                    mods,
                }));
            }
        }
    }

    let mut prev: Option<Uuid> = None;
    for &uuid in installed {
        if !seen.contains(&uuid) {
            let index = prev
                .and_then(|prev| result.iter().position(|entry| entry_contains(entry, prev)))
                .map_or(0, |index| index + 1);

            result.insert(index, LayoutEntry::Mod { uuid });
            seen.insert(uuid);
        }

        prev = Some(uuid);
    }

    result
}

impl Profile {
    fn installed_uuids(&self) -> Vec<Uuid> {
        self.mods
            .iter()
            .map(|profile_mod| profile_mod.uuid())
            .collect()
    }

    /// The current layout of the mod list, which always contains every installed mod exactly once.
    pub fn mod_layout(&self) -> Vec<LayoutEntry> {
        reconcile(&self.folder_layout, &self.installed_uuids())
    }

    /// Replaces the layout and reorders the mods to match it.
    pub fn set_mod_layout(&mut self, entries: Vec<LayoutEntry>) -> Result<()> {
        let layout = reconcile(&entries, &self.installed_uuids());

        let order: HashMap<Uuid, usize> = flatten(&layout)
            .enumerate()
            .map(|(index, uuid)| (uuid, index))
            .collect();
        self.mods
            .sort_by_key(|profile_mod| order.get(&profile_mod.uuid()).copied());

        self.folder_layout = if has_folders(&layout) {
            layout
        } else {
            Vec::new()
        };

        save(&self.path, &self.folder_layout)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn uuid(n: u128) -> Uuid {
        Uuid::from_u128(n)
    }

    fn m(n: u128) -> LayoutEntry {
        LayoutEntry::Mod { uuid: uuid(n) }
    }

    fn folder(id: u128, mods: &[u128]) -> LayoutEntry {
        LayoutEntry::Folder(ModFolder {
            id: uuid(id),
            name: "Folder".to_owned(),
            collapsed: false,
            mods: mods.iter().map(|n| uuid(*n)).collect(),
        })
    }

    #[test]
    fn empty_layout_follows_installed_order() {
        let installed = [uuid(1), uuid(2), uuid(3)];
        assert_eq!(reconcile(&[], &installed), vec![m(1), m(2), m(3)]);
    }

    #[test]
    fn removes_uninstalled_and_duplicate_mods() {
        let stored = [m(1), folder(100, &[2, 9, 1]), m(9), m(3)];
        let installed = [uuid(1), uuid(2), uuid(3)];

        assert_eq!(
            reconcile(&stored, &installed),
            vec![m(1), folder(100, &[2]), m(3)]
        );
    }

    #[test]
    fn keeps_empty_folders() {
        let stored = [folder(100, &[]), m(1)];
        let installed = [uuid(1)];

        assert_eq!(reconcile(&stored, &installed), stored.to_vec());
    }

    #[test]
    fn inserts_new_mods_after_predecessor() {
        let stored = [folder(100, &[1, 2]), m(3)];
        // 4 was installed after 2, 5 at the very start
        let installed = [uuid(5), uuid(1), uuid(2), uuid(4), uuid(3)];

        assert_eq!(
            reconcile(&stored, &installed),
            vec![m(5), folder(100, &[1, 2]), m(4), m(3)]
        );
    }

    #[test]
    fn drops_duplicate_folders() {
        let stored = [folder(100, &[1]), folder(100, &[2])];
        let installed = [uuid(1), uuid(2)];

        assert_eq!(
            reconcile(&stored, &installed),
            vec![folder(100, &[1]), m(2)]
        );
    }

    #[test]
    fn sanitizes_folder_names() {
        let mut long = folder(100, &[]);
        if let LayoutEntry::Folder(f) = &mut long {
            f.name = format!("  {}  ", "a".repeat(500));
        }

        let result = reconcile(&[long], &[]);
        let LayoutEntry::Folder(f) = &result[0] else {
            panic!()
        };
        assert_eq!(f.name.len(), MAX_NAME_LEN);
    }
}
