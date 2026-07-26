use crate::{
    CloseSessionSettings, CoreError, LaunchRecipe, NewRacingProfile, ProfileApplication,
    ProfileReviewStatus, ProfileSummary, RacingProfile, ShutdownStrategy, SupportingApplication,
    atomic_file::replace_with_backup,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashSet,
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
};
use uuid::Uuid;

const PROFILE_SCHEMA_VERSION: u32 = 2;
const LEGACY_PROFILE_SCHEMA_VERSION: u32 = 1;
const PORTABLE_PROFILE_SCHEMA_VERSION: u32 = 1;

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
    #[serde(default)]
    review_status: ProfileReviewStatus,
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
            review_status: ProfileReviewStatus::Approved,
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct PortableProfileApplication {
    name: String,
    launch_recipe: LaunchRecipe,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct PortableSupportingApplication {
    application: PortableProfileApplication,
    requirement: crate::ApplicationRequirement,
    keep_running: bool,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct PortableRacingProfileDocument {
    schema_version: u32,
    name: String,
    primary_sim: PortableProfileApplication,
    supporting_applications: Vec<PortableSupportingApplication>,
    vr_enabled: bool,
    preferred_vr_launch_mode: Option<crate::VrLaunchMode>,
    close_session: CloseSessionSettings,
}

impl PortableRacingProfileDocument {
    fn from_stored(profile: &RacingProfileDocument) -> Self {
        Self {
            schema_version: PORTABLE_PROFILE_SCHEMA_VERSION,
            name: profile.name.clone(),
            primary_sim: PortableProfileApplication {
                name: profile.primary_sim.name.clone(),
                launch_recipe: profile.primary_sim.launch_recipe.clone(),
            },
            supporting_applications: profile
                .supporting_applications
                .iter()
                .map(|supporting_application| PortableSupportingApplication {
                    application: PortableProfileApplication {
                        name: supporting_application.application.name.clone(),
                        launch_recipe: supporting_application.application.launch_recipe.clone(),
                    },
                    requirement: supporting_application.requirement.clone(),
                    keep_running: supporting_application.keep_running,
                })
                .collect(),
            vr_enabled: profile.vr_enabled,
            preferred_vr_launch_mode: profile.preferred_vr_launch_mode.clone(),
            close_session: profile.close_session.clone(),
        }
    }
}

pub(crate) struct ProfileLibrary {
    backups_directory: PathBuf,
    profiles_directory: PathBuf,
    profiles: Vec<StoredProfile>,
}

#[derive(Clone)]
struct StoredProfile {
    document: RacingProfileDocument,
    source_path: PathBuf,
}

impl ProfileLibrary {
    pub(crate) fn open(storage_root: impl AsRef<Path>) -> Result<Self, CoreError> {
        let profiles_directory = storage_root.as_ref().join("profiles");
        let backups_directory = storage_root.as_ref().join("backups");
        fs::create_dir_all(&profiles_directory)?;
        fs::create_dir_all(&backups_directory)?;
        Self::recover_interrupted_replacements(&profiles_directory, &backups_directory)?;

        let mut profiles = Vec::new();
        for entry in fs::read_dir(&profiles_directory)? {
            let entry = entry?;
            if !entry.file_type()?.is_file() {
                continue;
            }
            let path = entry.path();
            if path.extension().and_then(|extension| extension.to_str()) != Some("json") {
                continue;
            }

            let document = Self::load_live_document(&path, &backups_directory)?;
            let (mut document, source_path) = if Self::identity_matches_source(&document, &path) {
                (document, path)
            } else {
                Self::repair_legacy_identity(
                    &profiles_directory,
                    &backups_directory,
                    &path,
                    document,
                )?
            };
            if document.schema_version == LEGACY_PROFILE_SCHEMA_VERSION {
                document.schema_version = PROFILE_SCHEMA_VERSION;
                Self::persist_migration(&source_path, &backups_directory, &document)?;
            }
            // Review status is user-writable JSON. A privileged recipe is never
            // trusted after a disk reload: only this running native process may
            // hold a freshly reviewed privileged configuration.
            if Self::applications(&document).any(|application| {
                Self::recipe_requires_privileged_approval(&application.launch_recipe)
            }) {
                document.review_status = ProfileReviewStatus::NeedsReview;
            }
            profiles.push(StoredProfile {
                document,
                source_path,
            });
        }

        profiles.sort_by(|left, right| {
            left.document
                .name
                .cmp(&right.document.name)
                .then_with(|| left.document.id.cmp(&right.document.id))
        });

        Ok(Self {
            backups_directory,
            profiles_directory,
            profiles,
        })
    }

    fn load_document(path: &Path) -> Result<RacingProfileDocument, CoreError> {
        let document: RacingProfileDocument = serde_json::from_slice(&fs::read(path)?)?;
        if !matches!(
            document.schema_version,
            LEGACY_PROFILE_SCHEMA_VERSION | PROFILE_SCHEMA_VERSION
        ) {
            return Err(CoreError::UnsupportedProfileSchema(document.schema_version));
        }
        validate_profile_names(&document.name, &document.primary_sim.name)?;

        Ok(document)
    }

    fn load_live_document(
        path: &Path,
        backups_directory: &Path,
    ) -> Result<RacingProfileDocument, CoreError> {
        match Self::load_document(path) {
            Ok(document) => Ok(document),
            Err(CoreError::InvalidProfileDocument(_)) | Err(CoreError::InvalidProfileName(_)) => {
                let file_name = path
                    .file_name()
                    .ok_or_else(|| std::io::Error::other("profile path has no file name"))?;
                let backup = backups_directory.join(file_name);
                let document = Self::load_document(&backup)?;

                fs::remove_file(path)?;
                fs::rename(backup, path)?;

                Ok(document)
            }
            Err(error) => Err(error),
        }
    }

    fn identity_matches_source(document: &RacingProfileDocument, source_path: &Path) -> bool {
        Self::is_canonical_uuid(&document.id)
            && source_path.file_stem().and_then(|stem| stem.to_str()) == Some(document.id.as_str())
    }

    fn is_canonical_uuid(profile_id: &str) -> bool {
        Uuid::parse_str(profile_id).is_ok_and(|uuid| uuid.to_string() == profile_id)
    }

    fn repair_legacy_identity(
        profiles_directory: &Path,
        backups_directory: &Path,
        source_path: &Path,
        mut document: RacingProfileDocument,
    ) -> Result<(RacingProfileDocument, PathBuf), CoreError> {
        let (profile_id, destination, temporary, backup) = loop {
            let profile_id = Uuid::new_v4().to_string();
            let destination = profiles_directory.join(format!("{profile_id}.json"));
            let temporary = profiles_directory.join(format!(".{profile_id}.json.tmp"));
            let backup = backups_directory.join(format!("{profile_id}.legacy.json"));
            if !destination.exists() && !temporary.exists() && !backup.exists() {
                break (profile_id, destination, temporary, backup);
            }
        };

        document.id = profile_id;
        document.schema_version = PROFILE_SCHEMA_VERSION;
        Self::write_temporary_document(&temporary, &document)?;
        if let Err(error) = fs::rename(source_path, &backup) {
            let _ = fs::remove_file(&temporary);
            return Err(error.into());
        }
        if let Err(error) = fs::rename(&temporary, &destination) {
            let _ = fs::rename(&backup, source_path);
            let _ = fs::remove_file(&temporary);
            return Err(error.into());
        }

        Ok((document, destination))
    }

    fn write_temporary_document(
        temporary: &Path,
        document: &RacingProfileDocument,
    ) -> Result<(), CoreError> {
        let mut serialized = serde_json::to_vec_pretty(document)?;
        serialized.push(b'\n');
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(temporary)?;
        if let Err(error) = file.write_all(&serialized).and_then(|()| file.sync_all()) {
            drop(file);
            let _ = fs::remove_file(temporary);
            return Err(error.into());
        }
        Ok(())
    }

    fn sort_profiles(&mut self) {
        self.profiles.sort_by(|left, right| {
            left.document
                .name
                .cmp(&right.document.name)
                .then_with(|| left.document.id.cmp(&right.document.id))
        });
    }

    fn profile_index(&self, profile_id: &str) -> Result<usize, CoreError> {
        if !Self::is_canonical_uuid(profile_id) {
            return Err(CoreError::ProfileNotFound(profile_id.to_owned()));
        }
        self.profiles
            .iter()
            .position(|profile| profile.document.id == profile_id)
            .ok_or_else(|| CoreError::ProfileNotFound(profile_id.to_owned()))
    }

    fn persisted_paths(
        &self,
        profile_index: usize,
    ) -> Result<(PathBuf, PathBuf, PathBuf), CoreError> {
        let stored = &self.profiles[profile_index];
        if !Self::identity_matches_source(&stored.document, &stored.source_path)
            || stored.source_path.parent() != Some(self.profiles_directory.as_path())
        {
            return Err(std::io::Error::other(
                "stored profile identity no longer matches its trusted source path",
            )
            .into());
        }
        let file_name = stored
            .source_path
            .file_name()
            .ok_or_else(|| std::io::Error::other("profile path has no file name"))?;
        let temporary = self
            .profiles_directory
            .join(format!(".{}.json.tmp", stored.document.id));
        let backup = self.backups_directory.join(file_name);
        Ok((stored.source_path.clone(), temporary, backup))
    }

    fn recover_interrupted_replacements(
        profiles_directory: &Path,
        backups_directory: &Path,
    ) -> Result<(), CoreError> {
        for entry in fs::read_dir(profiles_directory)? {
            let temporary = entry?.path();
            let Some(file_name) = temporary.file_name().and_then(|name| name.to_str()) else {
                continue;
            };
            let Some(profile_id) = file_name
                .strip_prefix('.')
                .and_then(|name| name.strip_suffix(".json.tmp"))
            else {
                continue;
            };
            let destination = profiles_directory.join(format!("{profile_id}.json"));
            if destination.exists() {
                fs::remove_file(temporary)?;
                continue;
            }

            let backup = backups_directory.join(format!("{profile_id}.json"));
            if !backup.exists() {
                continue;
            }
            Self::load_document(&backup)?;

            fs::remove_file(temporary)?;
            fs::rename(backup, destination)?;
        }

        Ok(())
    }

    fn persist_migration(
        destination: &Path,
        backups_directory: &Path,
        document: &RacingProfileDocument,
    ) -> Result<(), CoreError> {
        let profile_id = &document.id;
        let temporary = destination
            .parent()
            .ok_or_else(|| std::io::Error::other("profile path has no parent"))?
            .join(format!(".{profile_id}.json.tmp"));
        let backup = backups_directory.join(format!("{profile_id}.json"));
        let mut serialized = serde_json::to_vec_pretty(document)?;
        serialized.push(b'\n');
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)?;
        file.write_all(&serialized)?;
        file.sync_all()?;
        drop(file);

        replace_with_backup(destination, &temporary, &backup)?;

        Ok(())
    }

    pub(crate) fn summaries(&self) -> Vec<ProfileSummary> {
        self.profiles
            .iter()
            .map(|profile| ProfileSummary {
                id: profile.document.id.clone(),
                name: profile.document.name.clone(),
                primary_sim_name: profile.document.primary_sim.name.clone(),
                primary_sim_application_id: Some(profile.document.primary_sim.id.clone()),
                review_status: profile.document.review_status.clone(),
            })
            .collect()
    }

    pub(crate) fn profiles(&self) -> impl Iterator<Item = RacingProfile> + '_ {
        self.profiles
            .iter()
            .map(|profile| profile.document.as_profile())
    }

    pub(crate) fn configured_application_count(&self) -> usize {
        self.profiles
            .iter()
            .map(|profile| 1 + profile.document.supporting_applications.len())
            .sum()
    }

    pub(crate) fn selected_profile(&self) -> Option<RacingProfile> {
        self.profiles
            .first()
            .map(|profile| profile.document.as_profile())
    }

    pub(crate) fn contains(&self, profile_id: &str) -> bool {
        Self::is_canonical_uuid(profile_id)
            && self
                .profiles
                .iter()
                .any(|profile| profile.document.id == profile_id)
    }

    pub(crate) fn review_status(&self, profile_id: &str) -> Result<ProfileReviewStatus, CoreError> {
        Ok(self.profiles[self.profile_index(profile_id)?]
            .document
            .review_status
            .clone())
    }

    pub(crate) fn profile(&self, profile_id: &str) -> Option<RacingProfile> {
        self.profiles
            .iter()
            .find(|profile| profile.document.id == profile_id)
            .map(|profile| profile.document.as_profile())
    }

    pub(crate) fn export(&self, profile_id: &str) -> Result<String, CoreError> {
        let profile = self
            .profiles
            .iter()
            .find(|profile| profile.document.id == profile_id)
            .ok_or_else(|| CoreError::ProfileNotFound(profile_id.to_owned()))?;
        let mut document = serde_json::to_string_pretty(
            &PortableRacingProfileDocument::from_stored(&profile.document),
        )?;
        document.push('\n');
        Ok(document)
    }

    pub(crate) fn import(&mut self, serialized: &str) -> Result<String, CoreError> {
        let portable: PortableRacingProfileDocument = serde_json::from_str(serialized)?;
        if portable.schema_version != PORTABLE_PROFILE_SCHEMA_VERSION {
            return Err(CoreError::UnsupportedProfileSchema(portable.schema_version));
        }
        validate_profile_names(&portable.name, &portable.primary_sim.name)?;
        for supporting_application in &portable.supporting_applications {
            if supporting_application.application.name.trim().is_empty() {
                return Err(CoreError::InvalidProfileName("Supporting Application name"));
            }
        }

        let profile_id = Uuid::new_v4().to_string();
        let primary_sim = ProfileApplication {
            id: Uuid::new_v4().to_string(),
            name: portable.primary_sim.name,
            path_needs_repair: Self::path_needs_repair(&portable.primary_sim.launch_recipe),
            launch_recipe: portable.primary_sim.launch_recipe,
        };
        let supporting_applications = portable
            .supporting_applications
            .into_iter()
            .map(|supporting_application| {
                let path_needs_repair =
                    Self::path_needs_repair(&supporting_application.application.launch_recipe);
                crate::SupportingApplication {
                    application: ProfileApplication {
                        id: Uuid::new_v4().to_string(),
                        name: supporting_application.application.name,
                        launch_recipe: supporting_application.application.launch_recipe,
                        path_needs_repair,
                    },
                    requirement: supporting_application.requirement,
                    keep_running: supporting_application.keep_running,
                }
            })
            .collect();
        let profile = RacingProfileDocument {
            schema_version: PROFILE_SCHEMA_VERSION,
            id: profile_id.clone(),
            name: portable.name,
            primary_sim,
            supporting_applications,
            vr_enabled: portable.vr_enabled,
            preferred_vr_launch_mode: portable.preferred_vr_launch_mode,
            close_session: portable.close_session,
            review_status: ProfileReviewStatus::NeedsReview,
        };
        let destination = self.profiles_directory.join(format!("{profile_id}.json"));
        let temporary = self
            .profiles_directory
            .join(format!(".{profile_id}.json.tmp"));
        Self::write_temporary_document(&temporary, &profile)?;
        fs::rename(&temporary, &destination)?;

        self.profiles.push(StoredProfile {
            document: profile,
            source_path: destination,
        });
        self.sort_profiles();

        Ok(profile_id)
    }

    fn path_needs_repair(recipe: &LaunchRecipe) -> bool {
        let source_needs_repair = match &recipe.source {
            crate::LaunchSource::DirectExecutable { executable_path } => {
                Self::executable_needs_repair(executable_path)
            }
            crate::LaunchSource::Steam { .. } => false,
        };
        let working_directory_needs_repair =
            recipe
                .working_directory
                .as_deref()
                .is_some_and(|directory| {
                    let path = Path::new(directory);
                    !path.is_absolute() || !path.is_dir()
                });
        let monitored_path_needs_repair = recipe
            .monitored_executable_path
            .as_deref()
            .is_some_and(Self::executable_needs_repair);
        let custom_stop_needs_repair = match &recipe.shutdown_strategy {
            ShutdownStrategy::CustomStop {
                executable_path, ..
            } => Self::executable_needs_repair(executable_path),
            _ => false,
        };

        source_needs_repair
            || working_directory_needs_repair
            || monitored_path_needs_repair
            || custom_stop_needs_repair
    }

    fn executable_needs_repair(executable_path: &str) -> bool {
        let path = Path::new(executable_path);
        if !path.is_absolute()
            || !path.is_file()
            || path
                .extension()
                .and_then(|extension| extension.to_str())
                .is_none_or(|extension| !extension.eq_ignore_ascii_case("exe"))
        {
            return true;
        }
        path.file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| {
                [
                    "cmd.exe",
                    "powershell.exe",
                    "pwsh.exe",
                    "wscript.exe",
                    "cscript.exe",
                    "mshta.exe",
                    "rundll32.exe",
                ]
                .iter()
                .any(|blocked| name.eq_ignore_ascii_case(blocked))
            })
    }

    pub(crate) fn create_complete(
        &mut self,
        profile: NewRacingProfile,
    ) -> Result<String, CoreError> {
        validate_profile_names(&profile.name, &profile.primary_sim.name)?;
        for supporting_application in &profile.supporting_applications {
            if supporting_application.application.name.trim().is_empty() {
                return Err(CoreError::InvalidProfileName("Supporting Application name"));
            }
        }

        let id = Uuid::new_v4().to_string();
        let profile = RacingProfileDocument {
            schema_version: PROFILE_SCHEMA_VERSION,
            id: id.clone(),
            name: profile.name,
            primary_sim: ProfileApplication {
                id: Uuid::new_v4().to_string(),
                name: profile.primary_sim.name,
                path_needs_repair: Self::path_needs_repair(&profile.primary_sim.launch_recipe),
                launch_recipe: profile.primary_sim.launch_recipe,
            },
            supporting_applications: profile
                .supporting_applications
                .into_iter()
                .map(|supporting| SupportingApplication {
                    application: ProfileApplication {
                        id: Uuid::new_v4().to_string(),
                        name: supporting.application.name,
                        path_needs_repair: Self::path_needs_repair(
                            &supporting.application.launch_recipe,
                        ),
                        launch_recipe: supporting.application.launch_recipe,
                    },
                    requirement: supporting.requirement,
                    keep_running: supporting.keep_running,
                })
                .collect(),
            vr_enabled: profile.vr_enabled,
            preferred_vr_launch_mode: profile.preferred_vr_launch_mode,
            close_session: profile.close_session,
            review_status: ProfileReviewStatus::Approved,
        };
        let destination = self.profiles_directory.join(format!("{id}.json"));
        let temporary = self.profiles_directory.join(format!(".{id}.json.tmp"));
        Self::write_temporary_document(&temporary, &profile)?;
        fs::rename(&temporary, &destination)?;

        self.profiles.push(StoredProfile {
            document: profile,
            source_path: destination,
        });
        self.sort_profiles();

        Ok(id)
    }

    pub(crate) fn discard_created(&mut self, profile_id: &str) -> Result<(), CoreError> {
        let profile_index = self.profile_index(profile_id)?;
        let destination = self.profiles[profile_index].source_path.clone();
        fs::remove_file(destination)?;
        self.profiles.remove(profile_index);
        Ok(())
    }

    pub(crate) fn edit(
        &mut self,
        profile_id: &str,
        name: String,
        primary_sim_name: String,
    ) -> Result<(), CoreError> {
        validate_profile_names(&name, &primary_sim_name)?;
        let profile_index = self.profile_index(profile_id)?;
        let (destination, temporary, backup) = self.persisted_paths(profile_index)?;
        let mut profile = self.profiles[profile_index].document.clone();
        profile.name = name;
        profile.primary_sim.name = primary_sim_name;
        Self::write_temporary_document(&temporary, &profile)?;
        replace_with_backup(&destination, &temporary, &backup)?;

        self.profiles[profile_index].document = profile;
        self.sort_profiles();

        Ok(())
    }

    pub(crate) fn save(&mut self, mut profile: RacingProfile) -> Result<(), CoreError> {
        validate_profile_names(&profile.name, &profile.primary_sim.name)?;
        for supporting_application in &profile.supporting_applications {
            if supporting_application.application.name.trim().is_empty() {
                return Err(CoreError::InvalidProfileName("Supporting Application name"));
            }
        }

        let profile_index = self.profile_index(&profile.id)?;
        let (destination, temporary, backup) = self.persisted_paths(profile_index)?;
        let stored_profile = self.profiles[profile_index].document.clone();
        profile.primary_sim.id = stored_profile.primary_sim.id.clone();
        profile.primary_sim.path_needs_repair =
            Self::path_needs_repair(&profile.primary_sim.launch_recipe);

        let existing_supporting_ids = stored_profile
            .supporting_applications
            .iter()
            .map(|supporting| supporting.application.id.clone())
            .collect::<HashSet<_>>();
        let mut retained_supporting_ids = HashSet::new();
        for supporting in &mut profile.supporting_applications {
            let incoming_id = &supporting.application.id;
            if !existing_supporting_ids.contains(incoming_id)
                || !retained_supporting_ids.insert(incoming_id.clone())
            {
                supporting.application.id = Uuid::new_v4().to_string();
                retained_supporting_ids.insert(supporting.application.id.clone());
            }
            supporting.application.path_needs_repair =
                Self::path_needs_repair(&supporting.application.launch_recipe);
        }

        let privileged_recipe_changed = Self::privileged_recipe_changed(&stored_profile, &profile);
        let mut profile = RacingProfileDocument::from(profile);
        profile.review_status = if stored_profile.review_status == ProfileReviewStatus::NeedsReview
            || privileged_recipe_changed
        {
            ProfileReviewStatus::NeedsReview
        } else {
            ProfileReviewStatus::Approved
        };
        Self::write_temporary_document(&temporary, &profile)?;
        replace_with_backup(&destination, &temporary, &backup)?;

        self.profiles[profile_index].document = profile;
        self.sort_profiles();

        Ok(())
    }

    pub(crate) fn approve(
        &mut self,
        profile_id: &str,
        configuration_reviewed: bool,
        approved_privileged_application_ids: &[String],
    ) -> Result<(), CoreError> {
        if !configuration_reviewed {
            return Err(CoreError::InvalidProfileApproval(
                "executable configuration was not confirmed".to_owned(),
            ));
        }
        let profile_index = self.profile_index(profile_id)?;
        let profile = &self.profiles[profile_index].document;
        if Self::applications(profile).any(|application| application.path_needs_repair) {
            return Err(CoreError::InvalidProfileApproval(
                "one or more executable paths must be repaired first".to_owned(),
            ));
        }

        let required = Self::applications(profile)
            .filter(|application| {
                Self::recipe_requires_privileged_approval(&application.launch_recipe)
            })
            .map(|application| application.id.clone())
            .collect::<HashSet<_>>();
        let approved = approved_privileged_application_ids
            .iter()
            .cloned()
            .collect::<HashSet<_>>();
        if approved.len() != approved_privileged_application_ids.len() || approved != required {
            return Err(CoreError::InvalidProfileApproval(
                "every elevated or custom-stop application must be approved exactly once"
                    .to_owned(),
            ));
        }

        let (destination, temporary, backup) = self.persisted_paths(profile_index)?;
        let mut approved_profile = profile.clone();
        approved_profile.review_status = ProfileReviewStatus::Approved;
        Self::write_temporary_document(&temporary, &approved_profile)?;
        replace_with_backup(&destination, &temporary, &backup)?;
        self.profiles[profile_index].document = approved_profile;

        Ok(())
    }

    fn applications(profile: &RacingProfileDocument) -> impl Iterator<Item = &ProfileApplication> {
        std::iter::once(&profile.primary_sim).chain(
            profile
                .supporting_applications
                .iter()
                .map(|supporting| &supporting.application),
        )
    }

    fn recipe_requires_privileged_approval(recipe: &LaunchRecipe) -> bool {
        recipe.elevated
            || matches!(
                &recipe.shutdown_strategy,
                ShutdownStrategy::CustomStop { .. }
            )
    }

    fn privileged_recipe_changed(stored: &RacingProfileDocument, updated: &RacingProfile) -> bool {
        std::iter::once(&updated.primary_sim)
            .chain(
                updated
                    .supporting_applications
                    .iter()
                    .map(|supporting| &supporting.application),
            )
            .any(|updated_application| {
                let stored_application = Self::applications(stored)
                    .find(|stored_application| stored_application.id == updated_application.id);
                match stored_application {
                    Some(stored_application) => !Self::privileged_recipe_matches(
                        &stored_application.launch_recipe,
                        &updated_application.launch_recipe,
                    ),
                    None => Self::recipe_requires_privileged_approval(
                        &updated_application.launch_recipe,
                    ),
                }
            })
    }

    fn privileged_recipe_matches(stored: &LaunchRecipe, updated: &LaunchRecipe) -> bool {
        let stored_requires = Self::recipe_requires_privileged_approval(stored);
        let updated_requires = Self::recipe_requires_privileged_approval(updated);
        if !stored_requires && !updated_requires {
            return true;
        }
        if stored_requires != updated_requires || stored.elevated != updated.elevated {
            return false;
        }
        if updated.elevated
            && (stored.source != updated.source
                || stored.arguments != updated.arguments
                || stored.working_directory != updated.working_directory)
        {
            return false;
        }
        if matches!(
            &stored.shutdown_strategy,
            ShutdownStrategy::CustomStop { .. }
        ) || matches!(
            &updated.shutdown_strategy,
            ShutdownStrategy::CustomStop { .. }
        ) {
            return stored.shutdown_strategy == updated.shutdown_strategy;
        }
        true
    }

    pub(crate) fn delete(&mut self, profile_id: &str) -> Result<(), CoreError> {
        let profile_index = self.profile_index(profile_id)?;
        let (destination, _temporary, backup) = self.persisted_paths(profile_index)?;

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
        let source_index = self.profile_index(source_profile_id)?;
        let source = self.profiles[source_index].document.clone();
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
        Self::write_temporary_document(&temporary, &duplicate)?;
        fs::rename(&temporary, &destination)?;

        self.profiles.push(StoredProfile {
            document: duplicate,
            source_path: destination,
        });
        self.sort_profiles();

        Ok(id)
    }
}
