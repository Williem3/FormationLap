use crate::{
    CatalogPrimarySim, CatalogSupportingApplication, DiscoveredInstallation, DiscoveredPrimarySim,
    DiscoverySnapshot,
};
use serde::Deserialize;
use std::{
    collections::{BTreeMap, BTreeSet},
    error::Error,
    fmt, fs,
    path::{Path, PathBuf},
};

const BUNDLED_SIMS: &str = include_str!("../../catalog/sims.json");
const BUNDLED_APPLICATIONS: &str = include_str!("../../catalog/applications.json");
const CATALOG_SCHEMA_VERSION: u32 = 1;

/// Explicit roots for limited, targeted local discovery.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct TargetedDiscoverySources {
    pub steam_roots: Vec<PathBuf>,
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
            | Self::DuplicateSteamAppId { .. } => None,
        }
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SimCatalogDocument {
    schema_version: u32,
    sims: Vec<CatalogPrimarySim>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SupportingApplicationCatalogDocument {
    schema_version: u32,
    applications: Vec<CatalogSupportingApplication>,
}

pub(crate) struct DiscoveryCatalog {
    primary_sims: Vec<CatalogPrimarySim>,
    supporting_applications: Vec<CatalogSupportingApplication>,
    sources: TargetedDiscoverySources,
}

impl DiscoveryCatalog {
    pub(crate) fn bundled_with_sources(
        sources: TargetedDiscoverySources,
    ) -> Result<Self, DiscoveryCatalogError> {
        let snapshot = validate_catalog_documents(BUNDLED_SIMS, BUNDLED_APPLICATIONS)?;
        Ok(Self {
            primary_sims: snapshot.primary_sims,
            supporting_applications: snapshot.supporting_applications,
            sources,
        })
    }

    pub(crate) fn snapshot(&self) -> DiscoverySnapshot {
        DiscoverySnapshot {
            primary_sims: self.primary_sims.clone(),
            supporting_applications: self.supporting_applications.clone(),
            installed_primary_sims: discover_steam_installations(
                &self.primary_sims,
                &self.sources.steam_roots,
            ),
        }
    }
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
                })
            })
        })
        .collect()
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

pub fn validate_catalog_documents(
    sims: &str,
    supporting_applications: &str,
) -> Result<DiscoverySnapshot, DiscoveryCatalogError> {
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

    Ok(DiscoverySnapshot {
        primary_sims: document.sims,
        supporting_applications: application_document.applications,
        installed_primary_sims: Vec::new(),
    })
}
