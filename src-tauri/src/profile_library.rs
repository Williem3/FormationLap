use crate::{
    CloseSessionSettings, CoreError, LaunchRecipe, ProfileApplication, ProfileSummary,
    RacingProfile,
};
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

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct RacingProfileDocument {
    schema_version: u32,
    id: String,
    name: String,
    primary_sim: ProfileApplication,
    #[serde(default)]
    supporting_applications: Vec<crate::SupportingApplication>,
    #[serde(default)]
    vr_enabled: bool,
    #[serde(default)]
    preferred_vr_launch_mode: Option<crate::VrLaunchMode>,
    #[serde(default)]
    close_session: CloseSessionSettings,
}

impl RacingProfileDocument {
    fn as_profile(&self) -> RacingProfile {
        RacingProfile {
            id: self.id.clone(),
            name: self.name.clone(),
            primary_sim: self.primary_sim.clone(),
            supporting_applications: self.supporting_applications.clone(),
            vr_enabled: self.vr_enabled,
            preferred_vr_launch_mode: self.preferred_vr_launch_mode.clone(),
            close_session: self.close_session.clone(),
        }
    }
}

impl From<RacingProfile> for RacingProfileDocument {
    fn from(profile: RacingProfile) -> Self {
        Self {
            schema_version: PROFILE_SCHEMA_VERSION,
            id: profile.id,
            name: profile.name,
            primary_sim: profile.primary_sim,
            supporting_applications: profile.supporting_applications,
            vr_enabled: profile.vr_enabled,
            preferred_vr_launch_mode: profile.preferred_vr_launch_mode,
            close_session: profile.close_session,
        }
    }
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

    pub(crate) fn selected_profile(&self) -> Option<RacingProfile> {
        self.profiles.first().map(RacingProfileDocument::as_profile)
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
            primary_sim: ProfileApplication {
                id: Uuid::new_v4().to_string(),
                name: primary_sim_name,
                launch_recipe: LaunchRecipe::default(),
                path_needs_repair: true,
            },
            supporting_applications: Vec::new(),
            vr_enabled: false,
            preferred_vr_launch_mode: None,
            close_session: CloseSessionSettings::default(),
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
        let mut profile = self.profiles[profile_index].clone();
        profile.name = name;
        profile.primary_sim.name = primary_sim_name;
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

    pub(crate) fn save(&mut self, profile: RacingProfile) -> Result<(), CoreError> {
        validate_profile_names(&profile.name, &profile.primary_sim.name)?;
        for supporting_application in &profile.supporting_applications {
            if supporting_application.application.name.trim().is_empty() {
                return Err(CoreError::InvalidProfileName("Supporting Application name"));
            }
        }

        let profile_index = self
            .profiles
            .iter()
            .position(|stored| stored.id == profile.id)
            .ok_or_else(|| CoreError::ProfileNotFound(profile.id.clone()))?;
        let profile = RacingProfileDocument::from(profile);
        let profile_id = profile.id.clone();
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
        let source = self
            .profiles
            .iter()
            .find(|profile| profile.id == source_profile_id)
            .cloned()
            .ok_or_else(|| CoreError::ProfileNotFound(source_profile_id.to_owned()))?;
        validate_profile_names(&name, &source.primary_sim.name)?;
        let id = Uuid::new_v4().to_string();
        let mut duplicate = source;
        duplicate.id = id.clone();
        duplicate.name = name;
        duplicate.primary_sim.id = Uuid::new_v4().to_string();
        for supporting_application in &mut duplicate.supporting_applications {
            supporting_application.application.id = Uuid::new_v4().to_string();
        }

        let destination = self.profiles_directory.join(format!("{id}.json"));
        let temporary = self.profiles_directory.join(format!(".{id}.json.tmp"));
        let mut serialized = serde_json::to_vec_pretty(&duplicate)?;
        serialized.push(b'\n');
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)?;
        file.write_all(&serialized)?;
        file.sync_all()?;
        drop(file);
        fs::rename(&temporary, &destination)?;

        self.profiles.push(duplicate);
        self.profiles.sort_by(|left, right| {
            left.name
                .cmp(&right.name)
                .then_with(|| left.id.cmp(&right.id))
        });

        Ok(id)
    }
}
