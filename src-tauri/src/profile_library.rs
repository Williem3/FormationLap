use crate::{CoreError, ProfileSummary};
use serde::{Deserialize, Serialize};
use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
};
use uuid::Uuid;

const PROFILE_SCHEMA_VERSION: u32 = 1;

fn validate_profile_names(name: &str, primary_sim_name: &str) -> Result<(), CoreError> {
    if name.trim().is_empty() {
        return Err(CoreError::InvalidProfileName("Racing Profile name"));
    }
    if primary_sim_name.trim().is_empty() {
        return Err(CoreError::InvalidProfileName("Primary Sim name"));
    }

    Ok(())
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct PrimarySimDocument {
    name: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct RacingProfileDocument {
    schema_version: u32,
    id: String,
    name: String,
    primary_sim: PrimarySimDocument,
}

pub(crate) struct ProfileLibrary {
    backups_directory: PathBuf,
    profiles_directory: PathBuf,
    profiles: Vec<RacingProfileDocument>,
}

impl ProfileLibrary {
    pub(crate) fn open(storage_root: impl AsRef<Path>) -> Result<Self, CoreError> {
        let profiles_directory = storage_root.as_ref().join("profiles");
        let backups_directory = storage_root.as_ref().join("backups");
        fs::create_dir_all(&profiles_directory)?;
        fs::create_dir_all(&backups_directory)?;

        let mut profiles = Vec::new();
        for entry in fs::read_dir(&profiles_directory)? {
            let path = entry?.path();
            if path.extension().and_then(|extension| extension.to_str()) != Some("json") {
                continue;
            }

            let document: RacingProfileDocument = serde_json::from_slice(&fs::read(&path)?)?;
            if document.schema_version != PROFILE_SCHEMA_VERSION {
                return Err(CoreError::UnsupportedProfileSchema(document.schema_version));
            }
            validate_profile_names(&document.name, &document.primary_sim.name)?;
            profiles.push(document);
        }

        profiles.sort_by(|left, right| {
            left.name
                .cmp(&right.name)
                .then_with(|| left.id.cmp(&right.id))
        });

        Ok(Self {
            backups_directory,
            profiles_directory,
            profiles,
        })
    }

    pub(crate) fn summaries(&self) -> Vec<ProfileSummary> {
        self.profiles
            .iter()
            .map(|profile| ProfileSummary {
                id: profile.id.clone(),
                name: profile.name.clone(),
                primary_sim_name: profile.primary_sim.name.clone(),
            })
            .collect()
    }

    pub(crate) fn create(
        &mut self,
        name: String,
        primary_sim_name: String,
    ) -> Result<String, CoreError> {
        validate_profile_names(&name, &primary_sim_name)?;

        let id = Uuid::new_v4().to_string();
        let profile = RacingProfileDocument {
            schema_version: PROFILE_SCHEMA_VERSION,
            id: id.clone(),
            name,
            primary_sim: PrimarySimDocument {
                name: primary_sim_name,
            },
        };
        let destination = self.profiles_directory.join(format!("{id}.json"));
        let temporary = self.profiles_directory.join(format!(".{id}.json.tmp"));
        let mut serialized = serde_json::to_vec_pretty(&profile)?;
        serialized.push(b'\n');

        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)?;
        file.write_all(&serialized)?;
        file.sync_all()?;
        drop(file);
        fs::rename(&temporary, &destination)?;

        self.profiles.push(profile);
        self.profiles.sort_by(|left, right| {
            left.name
                .cmp(&right.name)
                .then_with(|| left.id.cmp(&right.id))
        });

        Ok(id)
    }

    pub(crate) fn edit(
        &mut self,
        profile_id: &str,
        name: String,
        primary_sim_name: String,
    ) -> Result<(), CoreError> {
        validate_profile_names(&name, &primary_sim_name)?;
        let profile_index = self
            .profiles
            .iter()
            .position(|profile| profile.id == profile_id)
            .ok_or_else(|| CoreError::ProfileNotFound(profile_id.to_owned()))?;
        let profile = RacingProfileDocument {
            schema_version: PROFILE_SCHEMA_VERSION,
            id: profile_id.to_owned(),
            name,
            primary_sim: PrimarySimDocument {
                name: primary_sim_name,
            },
        };
        let destination = self.profiles_directory.join(format!("{profile_id}.json"));
        let temporary = self
            .profiles_directory
            .join(format!(".{profile_id}.json.tmp"));
        let backup = self.backups_directory.join(format!("{profile_id}.json"));
        let mut serialized = serde_json::to_vec_pretty(&profile)?;
        serialized.push(b'\n');

        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)?;
        file.write_all(&serialized)?;
        file.sync_all()?;
        drop(file);

        if backup.exists() {
            fs::remove_file(&backup)?;
        }
        fs::rename(&destination, &backup)?;
        if let Err(error) = fs::rename(&temporary, &destination) {
            let _ = fs::rename(&backup, &destination);
            return Err(error.into());
        }

        self.profiles[profile_index] = profile;
        self.profiles.sort_by(|left, right| {
            left.name
                .cmp(&right.name)
                .then_with(|| left.id.cmp(&right.id))
        });

        Ok(())
    }

    pub(crate) fn delete(&mut self, profile_id: &str) -> Result<(), CoreError> {
        let profile_index = self
            .profiles
            .iter()
            .position(|profile| profile.id == profile_id)
            .ok_or_else(|| CoreError::ProfileNotFound(profile_id.to_owned()))?;
        let destination = self.profiles_directory.join(format!("{profile_id}.json"));
        let backup = self.backups_directory.join(format!("{profile_id}.json"));

        if backup.exists() {
            fs::remove_file(&backup)?;
        }
        fs::rename(destination, backup)?;
        self.profiles.remove(profile_index);

        Ok(())
    }

    pub(crate) fn duplicate(
        &mut self,
        source_profile_id: &str,
        name: String,
    ) -> Result<String, CoreError> {
        let primary_sim_name = self
            .profiles
            .iter()
            .find(|profile| profile.id == source_profile_id)
            .map(|profile| profile.primary_sim.name.clone())
            .ok_or_else(|| CoreError::ProfileNotFound(source_profile_id.to_owned()))?;

        self.create(name, primary_sim_name)
    }
}
