use crate::{
    ApplicationIcon, CatalogPrimarySim, CatalogSupportingApplication, CatalogUpdateProvider,
    CompatibilityRank, DiscoveredInstallation, DiscoveredPrimarySim,
    DiscoveredSupportingApplication, DiscoverySnapshot, SupportingApplicationRecommendation,
};
use serde::Deserialize;
use std::{
    collections::{BTreeMap, BTreeSet},
    error::Error,
    fmt, fs,
    path::{Component, Path, PathBuf},
};

const BUNDLED_SIMS: &str = include_str!("../../catalog/sims.json");
const BUNDLED_APPLICATIONS: &str = include_str!("../../catalog/applications.json");
const CATALOG_SCHEMA_VERSION: u32 = 1;

/// Explicit roots for limited, targeted local discovery.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct TargetedDiscoverySources {
    pub steam_roots: Vec<PathBuf>,
    pub installed_applications: Vec<WindowsInstalledApplication>,
    pub running_processes: Vec<WindowsRunningProcess>,
    pub known_location_roots: Vec<WindowsKnownLocationRoot>,
}

/// One application record observed from a targeted Windows installed-app key.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WindowsInstalledApplication {
    pub display_name: String,
    pub install_location: PathBuf,
}

/// One Process image observed from the targeted Windows process inventory.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WindowsRunningProcess {
    pub executable_path: PathBuf,
}

/// One allowlisted Windows root used by Curated Catalog relative paths.
#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd)]
#[serde(rename_all = "camelCase")]
pub enum WindowsKnownLocation {
    ProgramFiles,
    ProgramFilesX86,
    LocalAppData,
    ProgramData,
    UserProfile,
}

/// The local path observed for one allowlisted Windows root.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WindowsKnownLocationRoot {
    pub kind: WindowsKnownLocation,
    pub path: PathBuf,
}

impl TargetedDiscoverySources {
    pub(crate) fn windows_defaults() -> Self {
        windows_sources::collect()
    }
}

#[cfg(not(windows))]
mod windows_sources {
    use super::TargetedDiscoverySources;

    pub(super) fn collect() -> TargetedDiscoverySources {
        TargetedDiscoverySources::default()
    }
}

#[cfg(windows)]
mod windows_sources {
    use super::{
        TargetedDiscoverySources, WindowsInstalledApplication, WindowsKnownLocation,
        WindowsKnownLocationRoot, WindowsRunningProcess,
    };
    use std::{
        collections::BTreeSet,
        ffi::{OsStr, OsString},
        mem::size_of,
        os::windows::ffi::{OsStrExt, OsStringExt},
        path::PathBuf,
        ptr::{null, null_mut},
    };
    use windows_sys::Win32::{
        Foundation::{ERROR_NO_MORE_ITEMS, ERROR_SUCCESS},
        System::Registry::{
            HKEY, HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, KEY_READ, KEY_WOW64_32KEY,
            KEY_WOW64_64KEY, REG_EXPAND_SZ, REG_SZ, RegCloseKey, RegEnumKeyExW, RegOpenKeyExW,
            RegQueryValueExW,
        },
    };

    const UNINSTALL_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Uninstall";

    struct OwnedRegistryKey(HKEY);

    impl Drop for OwnedRegistryKey {
        fn drop(&mut self) {
            unsafe {
                RegCloseKey(self.0);
            }
        }
    }

    pub(super) fn collect() -> TargetedDiscoverySources {
        TargetedDiscoverySources {
            steam_roots: steam_roots(),
            installed_applications: installed_applications(),
            running_processes: crate::process_runtime::running_executable_paths()
                .into_iter()
                .map(|executable_path| WindowsRunningProcess { executable_path })
                .collect(),
            known_location_roots: known_location_roots(),
        }
    }

    fn wide_null(value: impl AsRef<OsStr>) -> Vec<u16> {
        value
            .as_ref()
            .encode_wide()
            .chain(std::iter::once(0))
            .collect()
    }

    fn open_key(root: HKEY, path: impl AsRef<OsStr>, access: u32) -> Option<OwnedRegistryKey> {
        let path = wide_null(path);
        let mut key = null_mut();
        let status = unsafe { RegOpenKeyExW(root, path.as_ptr(), 0, access, &mut key) };
        (status == ERROR_SUCCESS).then_some(OwnedRegistryKey(key))
    }

    fn query_string(key: HKEY, value_name: &str) -> Option<String> {
        let value_name = wide_null(value_name);
        let mut value_type = 0;
        let mut byte_count = 0;
        let status = unsafe {
            RegQueryValueExW(
                key,
                value_name.as_ptr(),
                null_mut(),
                &mut value_type,
                null_mut(),
                &mut byte_count,
            )
        };
        if status != ERROR_SUCCESS
            || !matches!(value_type, REG_SZ | REG_EXPAND_SZ)
            || byte_count == 0
        {
            return None;
        }
        let mut buffer = vec![0_u16; (byte_count as usize).div_ceil(size_of::<u16>())];
        let status = unsafe {
            RegQueryValueExW(
                key,
                value_name.as_ptr(),
                null_mut(),
                &mut value_type,
                buffer.as_mut_ptr().cast(),
                &mut byte_count,
            )
        };
        if status != ERROR_SUCCESS {
            return None;
        }
        while buffer.last() == Some(&0) {
            buffer.pop();
        }
        let value = OsString::from_wide(&buffer)
            .to_string_lossy()
            .trim()
            .to_owned();
        (!value.is_empty()).then_some(value)
    }

    fn subkey_names(key: HKEY) -> Vec<OsString> {
        let mut names = Vec::new();
        for index in 0.. {
            let mut buffer = vec![0_u16; 16_384];
            let mut length = u32::try_from(buffer.len()).unwrap_or(u32::MAX);
            let status = unsafe {
                RegEnumKeyExW(
                    key,
                    index,
                    buffer.as_mut_ptr(),
                    &mut length,
                    null(),
                    null_mut(),
                    null_mut(),
                    null_mut(),
                )
            };
            if status == ERROR_NO_MORE_ITEMS {
                break;
            }
            if status != ERROR_SUCCESS {
                continue;
            }
            buffer.truncate(length as usize);
            names.push(OsString::from_wide(&buffer));
        }
        names
    }

    fn installed_applications() -> Vec<WindowsInstalledApplication> {
        let mut applications = Vec::new();
        let mut seen = BTreeSet::new();
        for root in [HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE] {
            for view in [KEY_WOW64_64KEY, KEY_WOW64_32KEY] {
                let Some(uninstall_key) = open_key(root, UNINSTALL_KEY, KEY_READ | view) else {
                    continue;
                };
                for subkey_name in subkey_names(uninstall_key.0) {
                    let Some(application_key) = open_key(uninstall_key.0, &subkey_name, KEY_READ)
                    else {
                        continue;
                    };
                    let Some(display_name) = query_string(application_key.0, "DisplayName") else {
                        continue;
                    };
                    let Some(install_location) = query_string(application_key.0, "InstallLocation")
                    else {
                        continue;
                    };
                    let install_location = PathBuf::from(install_location);
                    let identity = (
                        display_name.to_lowercase(),
                        install_location.to_string_lossy().to_lowercase(),
                    );
                    if seen.insert(identity) {
                        applications.push(WindowsInstalledApplication {
                            display_name,
                            install_location,
                        });
                    }
                }
            }
        }
        applications
    }

    fn steam_roots() -> Vec<PathBuf> {
        let mut roots = Vec::new();
        for view in [KEY_WOW64_64KEY, KEY_WOW64_32KEY] {
            let Some(steam_key) =
                open_key(HKEY_CURRENT_USER, r"Software\Valve\Steam", KEY_READ | view)
            else {
                continue;
            };
            if let Some(steam_path) = query_string(steam_key.0, "SteamPath") {
                roots.push(PathBuf::from(steam_path));
            }
        }
        if let Some(program_files_x86) = std::env::var_os("ProgramFiles(x86)") {
            roots.push(PathBuf::from(program_files_x86).join("Steam"));
        }
        roots.sort();
        roots.dedup();
        roots
    }

    fn known_location_roots() -> Vec<WindowsKnownLocationRoot> {
        [
            ("ProgramFiles", WindowsKnownLocation::ProgramFiles),
            ("ProgramFiles(x86)", WindowsKnownLocation::ProgramFilesX86),
            ("LOCALAPPDATA", WindowsKnownLocation::LocalAppData),
            ("ProgramData", WindowsKnownLocation::ProgramData),
            ("USERPROFILE", WindowsKnownLocation::UserProfile),
        ]
        .into_iter()
        .filter_map(|(environment_name, kind)| {
            std::env::var_os(environment_name).map(|path| WindowsKnownLocationRoot {
                kind,
                path: PathBuf::from(path),
            })
        })
        .collect()
    }
}

#[derive(Debug)]
pub enum DiscoveryCatalogError {
    InvalidDocument(serde_json::Error),
    InvalidSupportingApplicationDocument(serde_json::Error),
    UnsupportedSchema(u32),
    UnsupportedSupportingApplicationSchema(u32),
    DuplicateSimId {
        id: String,
        first_index: usize,
        duplicate_index: usize,
    },
    DuplicateSteamAppId {
        app_id: u32,
        first_location: String,
        duplicate_location: String,
    },
    UnknownCompatibilitySim {
        sim_id: String,
        application_index: usize,
        rule_index: usize,
    },
}

impl fmt::Display for DiscoveryCatalogError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidDocument(error) => write!(formatter, "sim catalog is invalid: {error}"),
            Self::InvalidSupportingApplicationDocument(error) => {
                write!(
                    formatter,
                    "Supporting Application catalog is invalid: {error}"
                )
            }
            Self::UnsupportedSchema(version) => {
                write!(
                    formatter,
                    "sim catalog schema version {version} is unsupported"
                )
            }
            Self::UnsupportedSupportingApplicationSchema(version) => write!(
                formatter,
                "Supporting Application catalog schema version {version} is unsupported"
            ),
            Self::DuplicateSimId {
                id,
                first_index,
                duplicate_index,
            } => write!(
                formatter,
                "duplicate sim id '{id}' at sims[{duplicate_index}].id; first declared at sims[{first_index}].id"
            ),
            Self::DuplicateSteamAppId {
                app_id,
                first_location,
                duplicate_location,
            } => write!(
                formatter,
                "duplicate Steam App ID {app_id} at {duplicate_location}; first declared at {first_location}"
            ),
            Self::UnknownCompatibilitySim {
                sim_id,
                application_index,
                rule_index,
            } => write!(
                formatter,
                "unknown compatibility sim id '{sim_id}' at applications[{application_index}].compatibility[{rule_index}].primarySimId"
            ),
        }
    }
}

impl Error for DiscoveryCatalogError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::InvalidDocument(error) | Self::InvalidSupportingApplicationDocument(error) => {
                Some(error)
            }
            Self::UnsupportedSchema(_)
            | Self::UnsupportedSupportingApplicationSchema(_)
            | Self::DuplicateSimId { .. }
            | Self::DuplicateSteamAppId { .. }
            | Self::UnknownCompatibilitySim { .. } => None,
        }
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SimCatalogDocument {
    schema_version: u32,
    sims: Vec<SimCatalogEntry>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SimCatalogEntry {
    id: String,
    name: String,
    #[serde(default)]
    steam_app_id: Option<u32>,
    #[serde(default)]
    installed_app_matchers: Vec<InstalledApplicationMatcher>,
}

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct InstalledApplicationMatcher {
    display_name: String,
    executable_relative_path: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SupportingApplicationCatalogDocument {
    schema_version: u32,
    applications: Vec<SupportingApplicationCatalogEntry>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SupportingApplicationCatalogEntry {
    id: String,
    name: String,
    #[serde(default)]
    executable_names: Vec<String>,
    #[serde(default)]
    known_locations: Vec<KnownLocationMatcher>,
    #[serde(default)]
    compatibility: Vec<CompatibilityMatcher>,
    #[serde(default)]
    update_provider: Option<CatalogUpdateProvider>,
}

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CompatibilityMatcher {
    primary_sim_id: String,
    rank: CompatibilityRank,
}

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct KnownLocationMatcher {
    root: WindowsKnownLocation,
    executable_relative_path: String,
}

pub(crate) struct DiscoveryCatalog {
    primary_sims: Vec<CatalogPrimarySim>,
    supporting_applications: Vec<CatalogSupportingApplication>,
    installed_app_matchers: BTreeMap<String, Vec<InstalledApplicationMatcher>>,
    supporting_executable_names: BTreeMap<String, Vec<String>>,
    supporting_known_locations: BTreeMap<String, Vec<KnownLocationMatcher>>,
    compatibility_matchers: BTreeMap<String, Vec<CompatibilityMatcher>>,
    update_providers: BTreeMap<String, CatalogUpdateProvider>,
    sources: TargetedDiscoverySources,
}

impl DiscoveryCatalog {
    pub(crate) fn bundled_with_sources(
        sources: TargetedDiscoverySources,
    ) -> Result<Self, DiscoveryCatalogError> {
        let documents = parse_and_validate_catalog_documents(BUNDLED_SIMS, BUNDLED_APPLICATIONS)?;
        let mut primary_sims = Vec::with_capacity(documents.sims.len());
        let mut installed_app_matchers = BTreeMap::new();
        for sim in documents.sims {
            installed_app_matchers.insert(sim.id.clone(), sim.installed_app_matchers);
            primary_sims.push(CatalogPrimarySim {
                id: sim.id,
                name: sim.name,
                steam_app_id: sim.steam_app_id,
            });
        }
        let mut supporting_applications =
            Vec::with_capacity(documents.supporting_applications.len());
        let mut supporting_executable_names = BTreeMap::new();
        let mut supporting_known_locations = BTreeMap::new();
        let mut compatibility_matchers = BTreeMap::new();
        let mut update_providers = BTreeMap::new();
        for application in documents.supporting_applications {
            supporting_executable_names
                .insert(application.id.clone(), application.executable_names);
            supporting_known_locations.insert(application.id.clone(), application.known_locations);
            compatibility_matchers.insert(application.id.clone(), application.compatibility);
            if let Some(update_provider) = application.update_provider {
                update_providers.insert(application.id.clone(), update_provider);
            }
            supporting_applications.push(CatalogSupportingApplication {
                id: application.id,
                name: application.name,
            });
        }
        Ok(Self {
            primary_sims,
            supporting_applications,
            installed_app_matchers,
            supporting_executable_names,
            supporting_known_locations,
            compatibility_matchers,
            update_providers,
            sources,
        })
    }

    pub(crate) fn snapshot(&self) -> DiscoverySnapshot {
        let mut installed_primary_sims =
            discover_steam_installations(&self.primary_sims, &self.sources.steam_roots);
        installed_primary_sims.extend(discover_installed_applications(
            &self.primary_sims,
            &self.installed_app_matchers,
            &self.sources.installed_applications,
        ));
        let mut installed_supporting_applications = discover_running_supporting_applications(
            &self.supporting_applications,
            &self.supporting_executable_names,
            &self.sources.running_processes,
        );
        installed_supporting_applications.extend(discover_known_location_supporting_applications(
            &self.supporting_applications,
            &self.supporting_known_locations,
            &self.sources.known_location_roots,
        ));
        installed_supporting_applications =
            unique_supporting_applications(installed_supporting_applications);
        DiscoverySnapshot {
            primary_sims: self.primary_sims.clone(),
            supporting_applications: self.supporting_applications.clone(),
            installed_primary_sims,
            installed_supporting_applications,
        }
    }

    pub(crate) fn recommendations(
        &self,
        primary_sim_id: &str,
    ) -> Vec<SupportingApplicationRecommendation> {
        let mut recommendations = self
            .supporting_applications
            .iter()
            .filter_map(|application| {
                let rank = self
                    .compatibility_matchers
                    .get(&application.id)?
                    .iter()
                    .find(|matcher| matcher.primary_sim_id == primary_sim_id)?
                    .rank
                    .clone();
                Some(SupportingApplicationRecommendation {
                    id: application.id.clone(),
                    name: application.name.clone(),
                    rank,
                    update_provider: self.update_providers.get(&application.id).cloned(),
                })
            })
            .collect::<Vec<_>>();
        recommendations.sort_by_key(|recommendation| recommendation.rank.clone());
        recommendations
    }
}

fn discover_known_location_supporting_applications(
    supporting_applications: &[CatalogSupportingApplication],
    matchers_by_application: &BTreeMap<String, Vec<KnownLocationMatcher>>,
    known_location_roots: &[WindowsKnownLocationRoot],
) -> Vec<DiscoveredSupportingApplication> {
    let mut discovered = Vec::new();
    for application in supporting_applications {
        let Some(matchers) = matchers_by_application.get(&application.id) else {
            continue;
        };
        for matcher in matchers {
            for known_root in known_location_roots {
                if known_root.kind != matcher.root {
                    continue;
                }
                let Some(executable_path) =
                    safe_catalog_relative_path(&known_root.path, &matcher.executable_relative_path)
                else {
                    continue;
                };
                if !executable_path.is_file() {
                    continue;
                }
                let icon = executable_icon(&executable_path);
                let executable_path = executable_path.canonicalize().unwrap_or(executable_path);
                let executable_path = executable_path.to_string_lossy().into_owned();
                discovered.push(DiscoveredSupportingApplication {
                    id: application.id.clone(),
                    name: application.name.clone(),
                    installation: DiscoveredInstallation::DirectExecutable { executable_path },
                    icon,
                });
            }
        }
    }
    discovered
}

fn unique_supporting_applications(
    applications: Vec<DiscoveredSupportingApplication>,
) -> Vec<DiscoveredSupportingApplication> {
    let mut seen = BTreeSet::new();
    applications
        .into_iter()
        .filter(|application| {
            let installation = match &application.installation {
                DiscoveredInstallation::DirectExecutable { executable_path } => {
                    format!("direct:{executable_path}")
                }
                DiscoveredInstallation::Steam { app_id, .. } => format!("steam:{app_id}"),
            };
            seen.insert((application.id.clone(), installation))
        })
        .collect()
}

fn discover_running_supporting_applications(
    supporting_applications: &[CatalogSupportingApplication],
    executable_names_by_application: &BTreeMap<String, Vec<String>>,
    running_processes: &[WindowsRunningProcess],
) -> Vec<DiscoveredSupportingApplication> {
    let mut executable_paths = BTreeSet::new();
    let mut discovered = Vec::new();
    for application in supporting_applications {
        let Some(executable_names) = executable_names_by_application.get(&application.id) else {
            continue;
        };
        for process in running_processes {
            let Some(file_name) = process
                .executable_path
                .file_name()
                .and_then(|name| name.to_str())
            else {
                continue;
            };
            if !executable_names
                .iter()
                .any(|candidate| candidate.eq_ignore_ascii_case(file_name))
                || !process.executable_path.is_file()
            {
                continue;
            }
            let icon = executable_icon(&process.executable_path);
            let executable_path = process
                .executable_path
                .canonicalize()
                .unwrap_or_else(|_| process.executable_path.clone());
            let executable_path = executable_path.to_string_lossy().into_owned();
            if !executable_paths.insert(executable_path.clone()) {
                continue;
            }
            discovered.push(DiscoveredSupportingApplication {
                id: application.id.clone(),
                name: application.name.clone(),
                installation: DiscoveredInstallation::DirectExecutable { executable_path },
                icon,
            });
        }
    }
    discovered
}

fn discover_installed_applications(
    primary_sims: &[CatalogPrimarySim],
    matchers_by_sim: &BTreeMap<String, Vec<InstalledApplicationMatcher>>,
    installed_applications: &[WindowsInstalledApplication],
) -> Vec<DiscoveredPrimarySim> {
    let mut executable_paths = BTreeSet::new();
    let mut discovered = Vec::new();
    for sim in primary_sims {
        let Some(matchers) = matchers_by_sim.get(&sim.id) else {
            continue;
        };
        for matcher in matchers {
            for installed_application in installed_applications {
                if !matcher
                    .display_name
                    .eq_ignore_ascii_case(&installed_application.display_name)
                {
                    continue;
                }
                let Some(executable_path) = safe_catalog_relative_path(
                    &installed_application.install_location,
                    &matcher.executable_relative_path,
                ) else {
                    continue;
                };
                if !executable_path.is_file() {
                    continue;
                }
                let icon = executable_icon(&executable_path);
                let executable_path = executable_path.canonicalize().unwrap_or(executable_path);
                let executable_path = executable_path.to_string_lossy().into_owned();
                if !executable_paths.insert(executable_path.clone()) {
                    continue;
                }
                discovered.push(DiscoveredPrimarySim {
                    id: sim.id.clone(),
                    name: sim.name.clone(),
                    installation: DiscoveredInstallation::DirectExecutable { executable_path },
                    icon,
                });
            }
        }
    }
    discovered
}

fn safe_catalog_relative_path(root: &Path, relative_path: &str) -> Option<PathBuf> {
    let relative_path = Path::new(relative_path);
    if relative_path.is_absolute()
        || relative_path
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
    {
        return None;
    }
    Some(root.join(relative_path))
}

fn discover_steam_installations(
    primary_sims: &[CatalogPrimarySim],
    steam_roots: &[PathBuf],
) -> Vec<DiscoveredPrimarySim> {
    let library_roots = steam_library_roots(steam_roots);
    primary_sims
        .iter()
        .filter_map(|sim| {
            let app_id = sim.steam_app_id?;
            library_roots.iter().find_map(|library_root| {
                let manifest_path = library_root
                    .join("steamapps")
                    .join(format!("appmanifest_{app_id}.acf"));
                let manifest = fs::read_to_string(manifest_path).ok()?;
                let install_directory = quoted_values_for_key(&manifest, "installdir")
                    .into_iter()
                    .next()?;
                let installation_path = library_root
                    .join("steamapps")
                    .join("common")
                    .join(install_directory);
                if !installation_path.is_dir() {
                    return None;
                }
                let installation_path = installation_path
                    .canonicalize()
                    .unwrap_or(installation_path)
                    .to_string_lossy()
                    .into_owned();
                Some(DiscoveredPrimarySim {
                    id: sim.id.clone(),
                    name: sim.name.clone(),
                    installation: DiscoveredInstallation::Steam {
                        app_id,
                        install_directory: installation_path,
                    },
                    icon: steam_icon(&manifest, steam_roots),
                })
            })
        })
        .collect()
}

fn steam_icon(manifest: &str, steam_roots: &[PathBuf]) -> ApplicationIcon {
    let Some(icon_hash) = quoted_values_for_key(manifest, "icon").into_iter().next() else {
        return ApplicationIcon::Generic;
    };
    steam_roots
        .iter()
        .find_map(|steam_root| {
            fs::read(
                steam_root
                    .join("steam")
                    .join("games")
                    .join(format!("{icon_hash}.ico")),
            )
            .ok()
        })
        .filter(|bytes| !bytes.is_empty())
        .map(|bytes| ApplicationIcon::LocalData {
            media_type: "image/x-icon".to_owned(),
            data_base64: encode_base64(&bytes),
        })
        .unwrap_or(ApplicationIcon::Generic)
}

fn encode_base64(bytes: &[u8]) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut encoded = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let first = chunk[0];
        let second = chunk.get(1).copied().unwrap_or(0);
        let third = chunk.get(2).copied().unwrap_or(0);
        encoded.push(ALPHABET[(first >> 2) as usize] as char);
        encoded.push(ALPHABET[(((first & 0b11) << 4) | (second >> 4)) as usize] as char);
        encoded.push(if chunk.len() > 1 {
            ALPHABET[(((second & 0b1111) << 2) | (third >> 6)) as usize] as char
        } else {
            '='
        });
        encoded.push(if chunk.len() > 2 {
            ALPHABET[(third & 0b11_1111) as usize] as char
        } else {
            '='
        });
    }
    encoded
}

fn executable_icon(executable_path: &Path) -> ApplicationIcon {
    windows_icon::extract(executable_path)
        .filter(|bytes| !bytes.is_empty())
        .map(|bytes| ApplicationIcon::LocalData {
            media_type: "image/x-icon".to_owned(),
            data_base64: encode_base64(&bytes),
        })
        .unwrap_or(ApplicationIcon::Generic)
}

#[cfg(not(windows))]
mod windows_icon {
    use std::path::Path;

    pub(super) fn extract(_executable_path: &Path) -> Option<Vec<u8>> {
        None
    }
}

#[cfg(windows)]
mod windows_icon {
    use std::{
        ffi::OsStr, mem::size_of, os::windows::ffi::OsStrExt, path::Path, ptr::null_mut,
        sync::Mutex,
    };
    use windows_sys::Win32::{
        Graphics::Gdi::{
            BI_RGB, BITMAP, BITMAPINFO, BITMAPINFOHEADER, CreateCompatibleDC, DIB_RGB_COLORS,
            DeleteDC, DeleteObject, GetDIBits, GetObjectW, HBITMAP, HDC, RGBQUAD,
        },
        UI::{
            Shell::{SHFILEINFOW, SHGFI_ICON, SHGFI_LARGEICON, SHGetFileInfoW},
            WindowsAndMessaging::{DestroyIcon, GetIconInfo, HICON, ICONINFO},
        },
    };

    struct OwnedIcon(HICON);
    struct OwnedBitmap(HBITMAP);
    struct OwnedDeviceContext(HDC);
    static SHELL_ICON_EXTRACTION: Mutex<()> = Mutex::new(());

    impl Drop for OwnedIcon {
        fn drop(&mut self) {
            unsafe {
                DestroyIcon(self.0);
            }
        }
    }

    impl Drop for OwnedBitmap {
        fn drop(&mut self) {
            if !self.0.is_null() {
                unsafe {
                    DeleteObject(self.0);
                }
            }
        }
    }

    impl Drop for OwnedDeviceContext {
        fn drop(&mut self) {
            unsafe {
                DeleteDC(self.0);
            }
        }
    }

    #[repr(C)]
    struct MonochromeBitmapInfo {
        header: BITMAPINFOHEADER,
        colors: [RGBQUAD; 2],
    }

    pub(super) fn extract(executable_path: &Path) -> Option<Vec<u8>> {
        let _guard = SHELL_ICON_EXTRACTION.lock().ok()?;
        let path = wide_null(executable_path.as_os_str());
        let mut shell_info = SHFILEINFOW::default();
        let shell_info_size = u32::try_from(size_of::<SHFILEINFOW>()).ok()?;
        if unsafe {
            SHGetFileInfoW(
                path.as_ptr(),
                0,
                &mut shell_info,
                shell_info_size,
                SHGFI_ICON | SHGFI_LARGEICON,
            )
        } == 0
            || shell_info.hIcon.is_null()
        {
            return None;
        }
        let icon = OwnedIcon(shell_info.hIcon);
        encode_icon(icon.0)
    }

    fn wide_null(value: &OsStr) -> Vec<u16> {
        value.encode_wide().chain(std::iter::once(0)).collect()
    }

    fn encode_icon(icon: HICON) -> Option<Vec<u8>> {
        let mut icon_info = ICONINFO::default();
        if unsafe { GetIconInfo(icon, &mut icon_info) } == 0 {
            return None;
        }
        let color = OwnedBitmap(icon_info.hbmColor);
        let mask = OwnedBitmap(icon_info.hbmMask);
        if color.0.is_null() {
            return None;
        }

        let mut bitmap = BITMAP::default();
        let bitmap_size = i32::try_from(size_of::<BITMAP>()).ok()?;
        if unsafe { GetObjectW(color.0, bitmap_size, (&mut bitmap as *mut BITMAP).cast()) } == 0 {
            return None;
        }
        let width = u32::try_from(bitmap.bmWidth).ok()?;
        let height = u32::try_from(bitmap.bmHeight).ok()?;
        if width == 0 || height == 0 {
            return None;
        }

        let device_context = OwnedDeviceContext(unsafe { CreateCompatibleDC(null_mut()) });
        if device_context.0.is_null() {
            return None;
        }
        let pixel_count = usize::try_from(width.checked_mul(height)?).ok()?;
        let mut color_pixels = vec![0_u8; pixel_count.checked_mul(4)?];
        let mut color_info = BITMAPINFO {
            bmiHeader: bitmap_header(width, height, 32, color_pixels.len())?,
            ..BITMAPINFO::default()
        };
        if unsafe {
            GetDIBits(
                device_context.0,
                color.0,
                0,
                height,
                color_pixels.as_mut_ptr().cast(),
                &mut color_info,
                DIB_RGB_COLORS,
            )
        } != i32::try_from(height).ok()?
        {
            return None;
        }

        let mask_row_bytes = usize::try_from(width.div_ceil(32).checked_mul(4)?).ok()?;
        let mut mask_pixels = vec![0_u8; mask_row_bytes.checked_mul(height as usize)?];
        if !mask.0.is_null() {
            let mut mask_info = MonochromeBitmapInfo {
                header: bitmap_header(width, height, 1, mask_pixels.len())?,
                colors: [RGBQUAD::default(), RGBQUAD::default()],
            };
            let mask_result = unsafe {
                GetDIBits(
                    device_context.0,
                    mask.0,
                    0,
                    height,
                    mask_pixels.as_mut_ptr().cast(),
                    (&mut mask_info as *mut MonochromeBitmapInfo).cast(),
                    DIB_RGB_COLORS,
                )
            };
            if mask_result != i32::try_from(height).ok()? {
                mask_pixels.fill(0);
            }
        }
        apply_mask_alpha(
            &mut color_pixels,
            &mask_pixels,
            width as usize,
            height as usize,
            mask_row_bytes,
        );
        build_ico(width, height, color_pixels, mask_pixels)
    }

    fn bitmap_header(
        width: u32,
        height: u32,
        bit_count: u16,
        image_size: usize,
    ) -> Option<BITMAPINFOHEADER> {
        Some(BITMAPINFOHEADER {
            biSize: u32::try_from(size_of::<BITMAPINFOHEADER>()).ok()?,
            biWidth: i32::try_from(width).ok()?,
            biHeight: i32::try_from(height).ok()?,
            biPlanes: 1,
            biBitCount: bit_count,
            biCompression: BI_RGB,
            biSizeImage: u32::try_from(image_size).ok()?,
            biXPelsPerMeter: 0,
            biYPelsPerMeter: 0,
            biClrUsed: 0,
            biClrImportant: 0,
        })
    }

    fn apply_mask_alpha(
        color_pixels: &mut [u8],
        mask_pixels: &[u8],
        width: usize,
        height: usize,
        mask_row_bytes: usize,
    ) {
        let has_alpha = color_pixels.chunks_exact(4).any(|pixel| pixel[3] != 0);
        for y in 0..height {
            for x in 0..width {
                let masked = mask_pixels[y * mask_row_bytes + x / 8] & (0x80_u8 >> (x % 8)) != 0;
                let alpha = &mut color_pixels[(y * width + x) * 4 + 3];
                if masked {
                    *alpha = 0;
                } else if !has_alpha {
                    *alpha = u8::MAX;
                }
            }
        }
    }

    fn build_ico(
        width: u32,
        height: u32,
        color_pixels: Vec<u8>,
        mask_pixels: Vec<u8>,
    ) -> Option<Vec<u8>> {
        let doubled_height = height.checked_mul(2)?;
        let image_size = color_pixels.len().checked_add(mask_pixels.len())?;
        let mut image = Vec::with_capacity(size_of::<BITMAPINFOHEADER>().checked_add(image_size)?);
        push_u32(
            &mut image,
            u32::try_from(size_of::<BITMAPINFOHEADER>()).ok()?,
        );
        push_i32(&mut image, i32::try_from(width).ok()?);
        push_i32(&mut image, i32::try_from(doubled_height).ok()?);
        push_u16(&mut image, 1);
        push_u16(&mut image, 32);
        push_u32(&mut image, BI_RGB);
        push_u32(&mut image, u32::try_from(image_size).ok()?);
        push_i32(&mut image, 0);
        push_i32(&mut image, 0);
        push_u32(&mut image, 0);
        push_u32(&mut image, 0);
        image.extend_from_slice(&color_pixels);
        image.extend_from_slice(&mask_pixels);

        let mut ico = Vec::with_capacity(22_usize.checked_add(image.len())?);
        push_u16(&mut ico, 0);
        push_u16(&mut ico, 1);
        push_u16(&mut ico, 1);
        ico.push(if width >= 256 { 0 } else { width as u8 });
        ico.push(if height >= 256 { 0 } else { height as u8 });
        ico.push(0);
        ico.push(0);
        push_u16(&mut ico, 1);
        push_u16(&mut ico, 32);
        push_u32(&mut ico, u32::try_from(image.len()).ok()?);
        push_u32(&mut ico, 22);
        ico.extend_from_slice(&image);
        Some(ico)
    }

    fn push_u16(output: &mut Vec<u8>, value: u16) {
        output.extend_from_slice(&value.to_le_bytes());
    }

    fn push_u32(output: &mut Vec<u8>, value: u32) {
        output.extend_from_slice(&value.to_le_bytes());
    }

    fn push_i32(output: &mut Vec<u8>, value: i32) {
        output.extend_from_slice(&value.to_le_bytes());
    }
}

fn steam_library_roots(steam_roots: &[PathBuf]) -> Vec<PathBuf> {
    let mut seen = BTreeSet::new();
    let mut libraries = Vec::new();
    for steam_root in steam_roots {
        push_unique_path(&mut libraries, &mut seen, steam_root.clone());
        let library_file = steam_root.join("steamapps").join("libraryfolders.vdf");
        let Some(document) = fs::read_to_string(library_file).ok() else {
            continue;
        };
        for declared_path in quoted_values_for_key(&document, "path") {
            push_unique_path(
                &mut libraries,
                &mut seen,
                Path::new(&declared_path).to_path_buf(),
            );
        }
    }
    libraries
}

fn push_unique_path(paths: &mut Vec<PathBuf>, seen: &mut BTreeSet<PathBuf>, path: PathBuf) {
    if seen.insert(path.clone()) {
        paths.push(path);
    }
}

fn quoted_values_for_key(document: &str, key: &str) -> Vec<String> {
    let tokens = quoted_tokens(document);
    tokens
        .windows(2)
        .filter(|pair| pair[0] == key)
        .map(|pair| pair[1].clone())
        .collect()
}

fn quoted_tokens(document: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut characters = document.chars();
    while let Some(character) = characters.next() {
        if character != '"' {
            continue;
        }
        let mut token = String::new();
        while let Some(character) = characters.next() {
            match character {
                '"' => break,
                '\\' => match characters.next() {
                    Some('\\') => token.push('\\'),
                    Some('"') => token.push('"'),
                    Some(escaped) => {
                        token.push('\\');
                        token.push(escaped);
                    }
                    None => token.push('\\'),
                },
                other => token.push(other),
            }
        }
        tokens.push(token);
    }
    tokens
}

struct ValidatedCatalogDocuments {
    sims: Vec<SimCatalogEntry>,
    supporting_applications: Vec<SupportingApplicationCatalogEntry>,
}

fn parse_and_validate_catalog_documents(
    sims: &str,
    supporting_applications: &str,
) -> Result<ValidatedCatalogDocuments, DiscoveryCatalogError> {
    let document: SimCatalogDocument =
        serde_json::from_str(sims).map_err(DiscoveryCatalogError::InvalidDocument)?;
    if document.schema_version != CATALOG_SCHEMA_VERSION {
        return Err(DiscoveryCatalogError::UnsupportedSchema(
            document.schema_version,
        ));
    }
    let mut sim_ids = BTreeMap::<&str, usize>::new();
    let mut steam_app_ids = BTreeMap::<u32, String>::new();
    for (index, sim) in document.sims.iter().enumerate() {
        if let Some(first_index) = sim_ids.insert(&sim.id, index) {
            return Err(DiscoveryCatalogError::DuplicateSimId {
                id: sim.id.clone(),
                first_index,
                duplicate_index: index,
            });
        }
        if let Some(app_id) = sim.steam_app_id {
            let location = format!("sims[{index}].steamAppId");
            if let Some(first_location) = steam_app_ids.insert(app_id, location.clone()) {
                return Err(DiscoveryCatalogError::DuplicateSteamAppId {
                    app_id,
                    first_location,
                    duplicate_location: location,
                });
            }
        }
    }

    let application_document: SupportingApplicationCatalogDocument =
        serde_json::from_str(supporting_applications)
            .map_err(DiscoveryCatalogError::InvalidSupportingApplicationDocument)?;
    if application_document.schema_version != CATALOG_SCHEMA_VERSION {
        return Err(
            DiscoveryCatalogError::UnsupportedSupportingApplicationSchema(
                application_document.schema_version,
            ),
        );
    }
    for (application_index, application) in application_document.applications.iter().enumerate() {
        for (rule_index, matcher) in application.compatibility.iter().enumerate() {
            if !sim_ids.contains_key(matcher.primary_sim_id.as_str()) {
                return Err(DiscoveryCatalogError::UnknownCompatibilitySim {
                    sim_id: matcher.primary_sim_id.clone(),
                    application_index,
                    rule_index,
                });
            }
        }
    }

    Ok(ValidatedCatalogDocuments {
        sims: document.sims,
        supporting_applications: application_document.applications,
    })
}

pub fn validate_catalog_documents(
    sims: &str,
    supporting_applications: &str,
) -> Result<DiscoverySnapshot, DiscoveryCatalogError> {
    let documents = parse_and_validate_catalog_documents(sims, supporting_applications)?;
    Ok(DiscoverySnapshot {
        primary_sims: documents
            .sims
            .into_iter()
            .map(|sim| CatalogPrimarySim {
                id: sim.id,
                name: sim.name,
                steam_app_id: sim.steam_app_id,
            })
            .collect(),
        supporting_applications: documents
            .supporting_applications
            .into_iter()
            .map(|application| CatalogSupportingApplication {
                id: application.id,
                name: application.name,
            })
            .collect(),
        installed_primary_sims: Vec::new(),
        installed_supporting_applications: Vec::new(),
    })
}
