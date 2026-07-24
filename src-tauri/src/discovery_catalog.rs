use crate::{CatalogPrimarySim, CatalogSupportingApplication, DiscoverySnapshot};
use serde::Deserialize;
use std::{collections::BTreeMap, error::Error, fmt};

const BUNDLED_SIMS: &str = include_str!("../../catalog/sims.json");
const BUNDLED_APPLICATIONS: &str = include_str!("../../catalog/applications.json");
const CATALOG_SCHEMA_VERSION: u32 = 1;

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
}

impl DiscoveryCatalog {
    pub(crate) fn bundled() -> Result<Self, DiscoveryCatalogError> {
        let snapshot = validate_catalog_documents(BUNDLED_SIMS, BUNDLED_APPLICATIONS)?;
        Ok(Self {
            primary_sims: snapshot.primary_sims,
            supporting_applications: snapshot.supporting_applications,
        })
    }

    pub(crate) fn snapshot(&self) -> DiscoverySnapshot {
        DiscoverySnapshot {
            primary_sims: self.primary_sims.clone(),
            supporting_applications: self.supporting_applications.clone(),
        }
    }
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
    })
}
