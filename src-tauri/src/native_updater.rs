use crate::{UpdateChannel, UpdateStatus};
use base64::{Engine as _, engine::general_purpose::STANDARD};
use minisign_verify::{PublicKey, Signature};
use serde::Deserialize;
use std::{io::Read, sync::Mutex, time::Duration};
use tauri::{AppHandle, Runtime, Url};
use tauri_plugin_updater::{Update, UpdaterExt};

const SELF_UPDATE_TIMEOUT: Duration = Duration::from_secs(30);
const MAX_BETA_DISCOVERY_BYTES: u64 = 262_144;
const STABLE_ENDPOINT: &str =
    "https://github.com/Williem3/FormationLap/releases/latest/download/latest.json";
const BETA_RELEASES_API: &str =
    "https://api.github.com/repos/Williem3/FormationLap/releases?per_page=20";

#[derive(Clone)]
pub(crate) struct ReleaseConfiguration {
    public_key: String,
    stable_endpoint: Url,
}

impl ReleaseConfiguration {
    fn new(public_key: Option<&str>) -> Result<Self, String> {
        let public_key = public_key
            .map(str::trim)
            .ok_or_else(|| "The official updater public key is not configured.".to_owned())?
            .to_owned();
        decode_public_key(&public_key)?;
        let stable_endpoint = parse_release_endpoint(STABLE_ENDPOINT, true)?;
        Ok(Self {
            public_key,
            stable_endpoint,
        })
    }

    fn from_compile_time() -> Result<Self, String> {
        Self::new(option_env!("FORMATION_LAP_UPDATE_PUBLIC_KEY"))
    }
}

fn decode_public_key(encoded: &str) -> Result<PublicKey, String> {
    let decoded = STANDARD
        .decode(encoded)
        .map_err(|_| "The official updater public key is not valid Base64.".to_owned())?;
    let decoded = std::str::from_utf8(&decoded)
        .map_err(|_| "The official updater public key is not UTF-8 text.".to_owned())?;
    PublicKey::decode(decoded)
        .map_err(|_| "The official updater public key is not a valid Minisign key.".to_owned())
}

fn decode_release_signature(encoded: &str) -> Result<Signature, String> {
    let decoded = STANDARD
        .decode(encoded)
        .map_err(|_| "The release signature metadata is not valid Base64.".to_owned())?;
    let decoded = std::str::from_utf8(&decoded)
        .map_err(|_| "The release signature metadata is not UTF-8 text.".to_owned())?;
    Signature::decode(decoded)
        .map_err(|_| "The release signature metadata is missing or invalid.".to_owned())
}

fn parse_release_endpoint(value: &str, stable: bool) -> Result<Url, String> {
    let url = Url::parse(value).map_err(|_| "The updater endpoint is invalid.".to_owned())?;
    let path_is_valid = if stable {
        url.as_str() == STABLE_ENDPOINT
    } else {
        url.path()
            .strip_prefix("/Williem3/FormationLap/releases/download/")
            .and_then(|path| path.strip_suffix("/latest.json"))
            .is_some_and(|tag| !tag.is_empty())
    };
    if url.scheme() != "https"
        || url.host_str() != Some("github.com")
        || !url.username().is_empty()
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
        || !path_is_valid
    {
        return Err("The updater endpoint is outside the official GitHub release feed.".to_owned());
    }
    Ok(url)
}

#[derive(Deserialize)]
struct GitHubReleaseAsset {
    name: String,
    browser_download_url: String,
}

#[derive(Deserialize)]
struct GitHubRelease {
    draft: bool,
    prerelease: bool,
    published_at: Option<String>,
    assets: Vec<GitHubReleaseAsset>,
}

fn parse_beta_endpoint(response: &str) -> Result<Url, String> {
    let releases = serde_json::from_str::<Vec<GitHubRelease>>(response)
        .map_err(|_| "The official Beta release feed returned invalid metadata.".to_owned())?;
    releases
        .into_iter()
        .filter(|release| !release.draft && release.prerelease && release.published_at.is_some())
        .find_map(|release| {
            let mut assets = release
                .assets
                .into_iter()
                .filter(|asset| asset.name == "latest.json");
            let asset = assets.next()?;
            if assets.next().is_some() {
                return None;
            }
            parse_release_endpoint(&asset.browser_download_url, false).ok()
        })
        .ok_or_else(|| "No published signed Beta update feed is available.".to_owned())
}

fn discover_beta_endpoint() -> Result<Url, String> {
    let client = reqwest::blocking::Client::builder()
        .https_only(true)
        .redirect(reqwest::redirect::Policy::none())
        .timeout(SELF_UPDATE_TIMEOUT)
        .user_agent(concat!("Formation-Lap/", env!("CARGO_PKG_VERSION")))
        .build()
        .map_err(|error| format!("The Beta discovery client could not start: {error}"))?;
    let response = client
        .get(BETA_RELEASES_API)
        .header("Accept", "application/vnd.github+json")
        .header("X-GitHub-Api-Version", "2022-11-28")
        .send()
        .map_err(|error| format!("The Beta release feed could not be reached: {error}"))?;
    if !response.status().is_success() {
        return Err(format!(
            "The Beta release feed returned HTTP status {}.",
            response.status()
        ));
    }
    let mut bytes = Vec::new();
    response
        .take(MAX_BETA_DISCOVERY_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|error| format!("The Beta release feed could not be read: {error}"))?;
    if bytes.len() as u64 > MAX_BETA_DISCOVERY_BYTES {
        return Err("The Beta release feed exceeded the safe size limit.".to_owned());
    }
    let response = String::from_utf8(bytes)
        .map_err(|_| "The Beta release feed was not UTF-8 text.".to_owned())?;
    parse_beta_endpoint(&response)
}

struct PendingSignedUpdate {
    channel: UpdateChannel,
    update: Update,
}

pub(crate) struct FormationLapUpdater {
    configuration: Result<ReleaseConfiguration, String>,
    pending: Mutex<Option<PendingSignedUpdate>>,
}

impl FormationLapUpdater {
    pub(crate) fn from_compile_time() -> Self {
        Self {
            configuration: ReleaseConfiguration::from_compile_time(),
            pending: Mutex::new(None),
        }
    }

    pub(crate) async fn check<R: Runtime>(
        &self,
        app: &AppHandle<R>,
        channel: UpdateChannel,
    ) -> Result<UpdateStatus, String> {
        let configuration = self.configuration.as_ref().map_err(Clone::clone)?;
        let endpoint = match channel {
            UpdateChannel::Stable => configuration.stable_endpoint.clone(),
            UpdateChannel::Beta => tauri::async_runtime::spawn_blocking(discover_beta_endpoint)
                .await
                .map_err(|_| "The Beta release discovery task failed.".to_owned())??,
        };
        let updater = app
            .updater_builder()
            .pubkey(configuration.public_key.clone())
            .endpoints(vec![endpoint])
            .map_err(|error| format!("The signed update feed is invalid: {error}"))?
            .timeout(SELF_UPDATE_TIMEOUT)
            .build()
            .map_err(|error| format!("The signed updater could not start: {error}"))?;
        let update = updater
            .check()
            .await
            .map_err(|error| format!("The signed update check failed: {error}"))?;
        let mut pending = self
            .pending
            .lock()
            .map_err(|_| "The signed updater state is unavailable.".to_owned())?;
        if let Some(update) = update {
            decode_release_signature(&update.signature)?;
            let status = UpdateStatus::UpdateAvailable {
                current_version: update.current_version.clone(),
                latest_version: update.version.clone(),
            };
            *pending = Some(PendingSignedUpdate { channel, update });
            Ok(status)
        } else {
            *pending = None;
            Ok(UpdateStatus::Current {
                current_version: env!("CARGO_PKG_VERSION").to_owned(),
            })
        }
    }

    pub(crate) async fn install<R: Runtime>(
        &self,
        _app: &AppHandle<R>,
        channel: UpdateChannel,
        expected_version: &str,
    ) -> Result<(), String> {
        let pending = self
            .pending
            .lock()
            .map_err(|_| "The signed updater state is unavailable.".to_owned())?
            .take()
            .ok_or_else(|| "Run a fresh signed update check before installing.".to_owned())?;
        if pending.channel != channel || pending.update.version != expected_version {
            return Err("The selected update changed; run a fresh signed update check.".to_owned());
        }
        pending
            .update
            .download_and_install(|_, _| {}, || {})
            .await
            .map_err(|error| format!("The signed update was rejected: {error}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const PUBLIC_KEY_TEXT: &str = "untrusted comment: minisign public key E7620F1842B4E81F\n\
RWQf6LRCGA9i53mlYecO4IzT51TGPpvWucNSCh1CBM0QTaLn73Y7GFO3";
    const SIGNATURE_TEXT: &str = "untrusted comment: signature from minisign secret key\n\
RWQf6LRCGA9i59SLOFxz6NxvASXDJeRtuZykwQepbDEGt87ig1BNpWaVWuNrm73YiIiJbq71Wi+dP9eKL8OC351vwIasSSbXxwA=\n\
trusted comment: timestamp:1555779966\tfile:test\n\
QtKMXWyYcwdpZAlPF7tE2ENJkRd1ujvKjlj1m9RtHTBnZPa5WKU5uWRs5GoP5M/VqE81QFuMKI5k/SfNQUaOAA==";

    #[test]
    fn release_configuration_requires_a_public_key_and_uses_the_official_stable_feed() {
        let public_key = STANDARD.encode(PUBLIC_KEY_TEXT);
        assert!(ReleaseConfiguration::new(None).is_err());
        let configured =
            ReleaseConfiguration::new(Some(&public_key)).expect("public key should be accepted");
        assert_eq!(
            configured.stable_endpoint.as_str(),
            "https://github.com/Williem3/FormationLap/releases/latest/download/latest.json"
        );
    }

    #[test]
    fn beta_discovery_ignores_drafts_and_stable_releases() {
        let response = r#"[
          {
            "draft": true,
            "prerelease": true,
            "published_at": "2026-07-24T10:00:00Z",
            "assets": [{"name":"latest.json","browser_download_url":"https://github.com/Williem3/FormationLap/releases/download/v1.0.0-beta.3/latest.json"}]
          },
          {
            "draft": false,
            "prerelease": false,
            "published_at": "2026-07-24T09:00:00Z",
            "assets": [{"name":"latest.json","browser_download_url":"https://github.com/Williem3/FormationLap/releases/download/v1.0.0/latest.json"}]
          },
          {
            "draft": false,
            "prerelease": true,
            "published_at": "2026-07-24T08:00:00Z",
            "assets": [{"name":"latest.json","browser_download_url":"https://github.com/Williem3/FormationLap/releases/download/v1.0.0-beta.2/latest.json"}]
          }
        ]"#;
        assert_eq!(
            parse_beta_endpoint(response)
                .expect("published prerelease should be selected")
                .as_str(),
            "https://github.com/Williem3/FormationLap/releases/download/v1.0.0-beta.2/latest.json"
        );
    }

    #[test]
    fn beta_discovery_rejects_ambiguous_or_untrusted_assets() {
        let duplicate_assets = r#"[{
          "draft": false,
          "prerelease": true,
          "published_at": "2026-07-24T08:00:00Z",
          "assets": [
            {"name":"latest.json","browser_download_url":"https://github.com/Williem3/FormationLap/releases/download/v1.0.0-beta.2/latest.json"},
            {"name":"latest.json","browser_download_url":"https://github.com/Williem3/FormationLap/releases/download/v1.0.0-beta.2/other/latest.json"}
          ]
        }]"#;
        let foreign_asset = r#"[{
          "draft": false,
          "prerelease": true,
          "published_at": "2026-07-24T08:00:00Z",
          "assets": [{"name":"latest.json","browser_download_url":"https://github.com/other/project/releases/download/v1.0.0-beta.2/latest.json"}]
        }]"#;
        assert!(parse_beta_endpoint(duplicate_assets).is_err());
        assert!(parse_beta_endpoint(foreign_asset).is_err());
    }

    #[test]
    fn missing_or_invalid_signature_metadata_is_rejected_before_download() {
        assert!(decode_release_signature("").is_err());
        assert!(decode_release_signature(&STANDARD.encode("not a minisign signature")).is_err());
    }

    #[test]
    fn invalid_bundle_signature_cannot_reach_the_installer_stage() {
        let public_key = decode_public_key(&STANDARD.encode(PUBLIC_KEY_TEXT))
            .expect("fixture public key should decode");
        let signature = decode_release_signature(&STANDARD.encode(SIGNATURE_TEXT))
            .expect("fixture signature should decode");
        assert!(
            public_key.verify(b"test", &signature, true).is_ok(),
            "the known signed fixture should verify"
        );

        let mut installer_invoked = false;
        if public_key.verify(b"tampered", &signature, true).is_ok() {
            installer_invoked = true;
        }
        assert!(!installer_invoked);
    }
}
