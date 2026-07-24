use crate::{CoreError, ProfileSummary};
use serde::{Deserialize, Serialize};
use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
};
use uuid::Uuid;

const PROFILE_SCHEMA_VERSION: u32 = 1;

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
    profiles_directory: PathBuf,
    profiles: Vec<RacingProfileDocument>,
}

impl ProfileLibrary {
    pub(crate) fn open(storage_root: impl AsRef<Path>) -> Result<Self, CoreError> {
        let profiles_directory = storage_root.as_ref().join("profiles");
        fs::create_dir_all(&profiles_directory)?;

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
            profiles.push(document);
        }

        profiles.sort_by(|left, right| {
            left.name
                .cmp(&right.name)
                .then_with(|| left.id.cmp(&right.id))
        });

        Ok(Self {
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
}
