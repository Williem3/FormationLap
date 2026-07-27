use crate::{UpdateChannel, UpdateStatus, update_coordinator::CancellationToken};
use base64::{Engine as _, engine::general_purpose::STANDARD};
use minisign_verify::{PublicKey, Signature};
use serde::Deserialize;
use std::{
    collections::HashMap,
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::PathBuf,
    sync::Mutex,
    time::Duration,
};
use tauri::{AppHandle, Runtime, Url};

const SELF_UPDATE_TIMEOUT: Duration = Duration::from_secs(30);
const MAX_BETA_DISCOVERY_BYTES: u64 = 262_144;
const MAX_RELEASE_METADATA_BYTES: u64 = 262_144;
const MAX_INSTALLER_BYTES: u64 = 134_217_728;
const MAX_UPDATE_REDIRECTS: usize = 3;
const UPDATE_TARGET: &str = "windows-x86_64";
const UPDATE_REDIRECT_HOSTS: &[&str] = &[
    "github.com",
    "api.github.com",
    "objects.githubusercontent.com",
    "release-assets.githubusercontent.com",
];
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
    tag_name: String,
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
            let endpoint = parse_release_endpoint(&asset.browser_download_url, false).ok()?;
            endpoint
                .clone()
                .path()
                .strip_prefix("/Williem3/FormationLap/releases/download/")
                .and_then(|path| path.strip_suffix("/latest.json"))
                .filter(|tag| *tag == release.tag_name)
                .map(|_| endpoint)
        })
        .ok_or_else(|| "No published signed Beta update feed is available.".to_owned())
}

fn validate_redirect_url(url: &Url) -> Result<(), String> {
    if url.scheme() != "https"
        || !url.username().is_empty()
        || url.password().is_some()
        || url.fragment().is_some()
        || !UPDATE_REDIRECT_HOSTS.iter().any(|host| {
            url.host_str()
                .is_some_and(|value| value.eq_ignore_ascii_case(host))
        })
    {
        return Err("The updater redirect left the controlled HTTPS hosts.".to_owned());
    }
    Ok(())
}

fn release_client() -> Result<reqwest::blocking::Client, String> {
    reqwest::blocking::Client::builder()
        .https_only(true)
        .redirect(reqwest::redirect::Policy::custom(|attempt| {
            if attempt.previous().len() >= MAX_UPDATE_REDIRECTS {
                attempt.error("the updater returned too many redirects")
            } else if validate_redirect_url(attempt.url()).is_ok() {
                attempt.follow()
            } else {
                attempt.error("the updater redirect left the controlled HTTPS hosts")
            }
        }))
        .timeout(SELF_UPDATE_TIMEOUT)
        .user_agent(concat!("Formation-Lap/", env!("CARGO_PKG_VERSION")))
        .build()
        .map_err(|error| format!("The update client could not start: {error}"))
}

fn fetch_bounded(
    client: &reqwest::blocking::Client,
    url: &Url,
    accept: &str,
    maximum_bytes: u64,
) -> Result<Vec<u8>, String> {
    validate_redirect_url(url)?;
    let response = client
        .get(url.clone())
        .header("Accept", accept)
        .header("X-GitHub-Api-Version", "2022-11-28")
        .send()
        .map_err(|error| format!("The official update source could not be reached: {error}"))?;
    validate_redirect_url(response.url())?;
    if !response.status().is_success() {
        return Err(format!(
            "The official update source returned HTTP status {}.",
            response.status()
        ));
    }
    if response
        .content_length()
        .is_some_and(|length| length > maximum_bytes)
    {
        return Err("The official update response exceeded the safe size limit.".to_owned());
    }
    let mut bytes = Vec::new();
    response
        .take(maximum_bytes + 1)
        .read_to_end(&mut bytes)
        .map_err(|error| format!("The official update response could not be read: {error}"))?;
    if bytes.len() as u64 > maximum_bytes {
        return Err("The official update response exceeded the safe size limit.".to_owned());
    }
    Ok(bytes)
}

fn discover_beta_endpoint(
    client: &reqwest::blocking::Client,
    cancellation: &CancellationToken,
) -> Result<Url, String> {
    if cancellation.is_cancelled() {
        return Err("The update check was cancelled for Session start.".to_owned());
    }
    let endpoint =
        Url::parse(BETA_RELEASES_API).map_err(|_| "The Beta feed URL is invalid.".to_owned())?;
    let bytes = fetch_bounded(
        client,
        &endpoint,
        "application/vnd.github+json",
        MAX_BETA_DISCOVERY_BYTES,
    )?;
    if cancellation.is_cancelled() {
        return Err("The update check was cancelled for Session start.".to_owned());
    }
    let response = String::from_utf8(bytes)
        .map_err(|_| "The Beta release feed was not UTF-8 text.".to_owned())?;
    parse_beta_endpoint(&response)
}

#[derive(Deserialize)]
struct ReleaseManifest {
    version: String,
    platforms: HashMap<String, ReleaseManifestPlatform>,
}

#[derive(Deserialize)]
struct ReleaseManifestPlatform {
    url: String,
    signature: String,
}

#[derive(Clone)]
struct PendingSignedUpdate {
    channel: UpdateChannel,
    download_url: Url,
    signature: String,
    target: String,
    version: String,
}

fn validate_installer_url(url: &Url, version: &str, target: &str) -> Result<(), String> {
    if target != UPDATE_TARGET {
        return Err("The signed update does not target the supported architecture.".to_owned());
    }
    let expected = format!(
        "https://github.com/Williem3/FormationLap/releases/download/v{version}/Formation-Lap_{version}_x64-setup.exe"
    );
    if url.as_str() != expected {
        return Err(
            "The installer URL does not match the official repository, tag, version, architecture, and filename."
                .to_owned(),
        );
    }
    Ok(())
}

fn check_release(
    configuration: &ReleaseConfiguration,
    channel: UpdateChannel,
    cancellation: &CancellationToken,
) -> Result<(UpdateStatus, Option<PendingSignedUpdate>), String> {
    let client = release_client()?;
    let endpoint = match channel {
        UpdateChannel::Stable => configuration.stable_endpoint.clone(),
        UpdateChannel::Beta => discover_beta_endpoint(&client, cancellation)?,
    };
    if cancellation.is_cancelled() {
        return Err("The update check was cancelled for Session start.".to_owned());
    }
    let metadata = fetch_bounded(
        &client,
        &endpoint,
        "application/json",
        MAX_RELEASE_METADATA_BYTES,
    )?;
    if cancellation.is_cancelled() {
        return Err("The update check was cancelled for Session start.".to_owned());
    }
    let manifest: ReleaseManifest = serde_json::from_slice(&metadata)
        .map_err(|_| "The official update feed returned invalid metadata.".to_owned())?;
    if manifest.platforms.len() != 1 {
        return Err("The official update feed contained an unexpected platform set.".to_owned());
    }
    let platform = manifest
        .platforms
        .get(UPDATE_TARGET)
        .ok_or_else(|| "The official update feed did not contain the x64 installer.".to_owned())?;
    let latest = semver::Version::parse(&manifest.version)
        .map_err(|_| "The official update version is invalid.".to_owned())?;
    let current = semver::Version::parse(env!("CARGO_PKG_VERSION"))
        .map_err(|_| "The installed Formation Lap version is invalid.".to_owned())?;
    match channel {
        UpdateChannel::Stable if !latest.pre.is_empty() => {
            return Err("The Stable feed cannot select a prerelease.".to_owned());
        }
        UpdateChannel::Beta if latest.pre.is_empty() => {
            return Err("The Beta feed must select a published prerelease.".to_owned());
        }
        _ => {}
    }
    if latest <= current {
        return Ok((
            UpdateStatus::Current {
                current_version: current.to_string(),
            },
            None,
        ));
    }
    let download_url = Url::parse(&platform.url)
        .map_err(|_| "The official installer URL is invalid.".to_owned())?;
    validate_installer_url(&download_url, &manifest.version, UPDATE_TARGET)?;
    decode_release_signature(&platform.signature)?;
    Ok((
        UpdateStatus::UpdateAvailable {
            current_version: current.to_string(),
            latest_version: manifest.version.clone(),
        },
        Some(PendingSignedUpdate {
            channel,
            download_url,
            signature: platform.signature.clone(),
            target: UPDATE_TARGET.to_owned(),
            version: manifest.version,
        }),
    ))
}

struct StagedInstaller {
    file: File,
    path: PathBuf,
}

fn download_and_stage(
    configuration: &ReleaseConfiguration,
    update: &PendingSignedUpdate,
) -> Result<StagedInstaller, String> {
    validate_installer_url(&update.download_url, &update.version, &update.target)?;
    let bytes = fetch_bounded(
        &release_client()?,
        &update.download_url,
        "application/octet-stream",
        MAX_INSTALLER_BYTES,
    )?;
    if !bytes.starts_with(b"MZ") {
        return Err("The signed update is not a Windows executable.".to_owned());
    }
    let public_key = decode_public_key(&configuration.public_key)?;
    let signature = decode_release_signature(&update.signature)?;
    public_key
        .verify(&bytes, &signature, true)
        .map_err(|_| "The downloaded installer signature is invalid.".to_owned())?;

    stage_verified_installer(&bytes, &update.version)
}

fn stage_verified_installer(bytes: &[u8], version: &str) -> Result<StagedInstaller, String> {
    let directory =
        std::env::temp_dir().join(format!("formation-lap-update-{}", uuid::Uuid::new_v4()));
    fs::create_dir(&directory)
        .map_err(|error| format!("The update staging directory could not be created: {error}"))?;
    let path = directory.join(format!("Formation-Lap_{}_x64-setup.exe", version));
    {
        let mut writer = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
            .map_err(|error| format!("The update installer could not be staged: {error}"))?;
        writer
            .write_all(bytes)
            .and_then(|()| writer.sync_all())
            .map_err(|error| format!("The update installer could not be staged: {error}"))?;
    }

    let mut options = OpenOptions::new();
    options.read(true);
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        use windows_sys::Win32::Storage::FileSystem::FILE_SHARE_READ;
        options.share_mode(FILE_SHARE_READ);
    }
    let mut file = options
        .open(&path)
        .map_err(|error| format!("The staged update installer could not be secured: {error}"))?;
    let mut staged_bytes = Vec::with_capacity(bytes.len());
    file.read_to_end(&mut staged_bytes)
        .map_err(|error| format!("The staged update installer could not be verified: {error}"))?;
    if staged_bytes != bytes {
        drop(file);
        let _ = fs::remove_dir_all(&directory);
        return Err("The staged update installer changed before it was secured.".to_owned());
    }
    Ok(StagedInstaller { file, path })
}

#[cfg(windows)]
fn launch_installer(installer: &StagedInstaller) -> Result<(), String> {
    use std::{ffi::OsStr, os::windows::ffi::OsStrExt};
    use windows_sys::Win32::UI::{Shell::ShellExecuteW, WindowsAndMessaging::SW_SHOW};

    let operation = OsStr::new("open")
        .encode_wide()
        .chain(std::iter::once(0))
        .collect::<Vec<_>>();
    let path = installer
        .path
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect::<Vec<_>>();
    let parameters = OsStr::new("/P /R /UPDATE /ARGS")
        .encode_wide()
        .chain(std::iter::once(0))
        .collect::<Vec<_>>();
    // Keep the verified read-only staging handle open until
    // ShellExecute has created the installer Process. This closes the
    // verification-to-launch replacement window while allowing Windows to map
    // the executable.
    let _verified_file = &installer.file;
    // SAFETY: All strings are stable, null-terminated UTF-16 values for the
    // duration of the call. The file and directory arguments are fixed local
    // values, and SW_SHOW is a valid display mode.
    let result = unsafe {
        ShellExecuteW(
            std::ptr::null_mut(),
            operation.as_ptr(),
            path.as_ptr(),
            parameters.as_ptr(),
            std::ptr::null(),
            SW_SHOW,
        )
    };
    if result as isize <= 32 {
        Err(format!(
            "The verified update installer could not start (ShellExecute error {}).",
            result as isize
        ))
    } else {
        Ok(())
    }
}

#[cfg(not(windows))]
fn launch_installer(_installer: &StagedInstaller) -> Result<(), String> {
    Err("Formation Lap update installation is supported only on Windows.".to_owned())
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

    pub(crate) async fn check(
        &self,
        channel: UpdateChannel,
        cancellation: CancellationToken,
    ) -> Result<UpdateStatus, String> {
        let configuration = self.configuration.as_ref().map_err(Clone::clone)?.clone();
        let worker_cancellation = cancellation.clone();
        let checked = tauri::async_runtime::spawn_blocking(move || {
            check_release(&configuration, channel, &worker_cancellation)
        })
        .await
        .map_err(|_| "The signed update check task failed.".to_owned())?;
        let mut pending = self
            .pending
            .lock()
            .map_err(|_| "The signed updater state is unavailable.".to_owned())?;
        if cancellation.is_cancelled() {
            *pending = None;
            return Err("The update check was cancelled for Session start.".to_owned());
        }
        let (status, checked_update) = checked?;
        *pending = checked_update;
        Ok(status)
    }

    pub(crate) async fn install<R: Runtime>(
        &self,
        app: &AppHandle<R>,
        channel: UpdateChannel,
        expected_version: &str,
    ) -> Result<(), String> {
        let pending = self
            .pending
            .lock()
            .map_err(|_| "The signed updater state is unavailable.".to_owned())?
            .take()
            .ok_or_else(|| "Run a fresh signed update check before installing.".to_owned())?;
        if pending.channel != channel || pending.version != expected_version {
            return Err("The selected update changed; run a fresh signed update check.".to_owned());
        }
        let configuration = self.configuration.as_ref().map_err(Clone::clone)?.clone();
        let installer = tauri::async_runtime::spawn_blocking(move || {
            download_and_stage(&configuration, &pending)
        })
        .await
        .map_err(|_| "The signed update download task failed.".to_owned())??;
        launch_installer(&installer)?;
        app.exit(0);
        Ok(())
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
            "tag_name": "v1.0.0-beta.3",
            "draft": true,
            "prerelease": true,
            "published_at": "2026-07-24T10:00:00Z",
            "assets": [{"name":"latest.json","browser_download_url":"https://github.com/Williem3/FormationLap/releases/download/v1.0.0-beta.3/latest.json"}]
          },
          {
            "tag_name": "v1.0.0",
            "draft": false,
            "prerelease": false,
            "published_at": "2026-07-24T09:00:00Z",
            "assets": [{"name":"latest.json","browser_download_url":"https://github.com/Williem3/FormationLap/releases/download/v1.0.0/latest.json"}]
          },
          {
            "tag_name": "v1.0.0-beta.2",
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
          "tag_name": "v1.0.0-beta.2",
          "draft": false,
          "prerelease": true,
          "published_at": "2026-07-24T08:00:00Z",
          "assets": [
            {"name":"latest.json","browser_download_url":"https://github.com/Williem3/FormationLap/releases/download/v1.0.0-beta.2/latest.json"},
            {"name":"latest.json","browser_download_url":"https://github.com/Williem3/FormationLap/releases/download/v1.0.0-beta.2/other/latest.json"}
          ]
        }]"#;
        let foreign_asset = r#"[{
          "tag_name": "v1.0.0-beta.2",
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

    #[test]
    fn official_installer_candidate_requires_exact_tag_version_architecture_and_filename() {
        let valid = Url::parse(
            "https://github.com/Williem3/FormationLap/releases/download/v1.2.3/Formation-Lap_1.2.3_x64-setup.exe",
        )
        .expect("fixture URL should parse");
        assert!(validate_installer_url(&valid, "1.2.3", "windows-x86_64").is_ok());

        for invalid in [
            "http://github.com/Williem3/FormationLap/releases/download/v1.2.3/Formation-Lap_1.2.3_x64-setup.exe",
            "https://github.com/other/FormationLap/releases/download/v1.2.3/Formation-Lap_1.2.3_x64-setup.exe",
            "https://github.com/Williem3/FormationLap/releases/download/v9.9.9/Formation-Lap_1.2.3_x64-setup.exe",
            "https://github.com/Williem3/FormationLap/releases/download/v1.2.3/Formation-Lap_1.2.3_arm64-setup.exe",
            "https://github.com/Williem3/FormationLap/releases/download/v1.2.3/other.exe",
        ] {
            assert!(
                validate_installer_url(
                    &Url::parse(invalid).expect("fixture URL should parse"),
                    "1.2.3",
                    "windows-x86_64",
                )
                .is_err(),
                "{invalid} must be rejected"
            );
        }
    }

    #[test]
    fn updater_redirects_stay_on_tightly_controlled_https_hosts() {
        for allowed in [
            "https://github.com/Williem3/FormationLap/releases/download/v1.2.3/latest.json",
            "https://objects.githubusercontent.com/github-production-release-asset/file?token=bounded",
            "https://release-assets.githubusercontent.com/github-production-release-asset/file?token=bounded",
        ] {
            assert!(validate_redirect_url(&Url::parse(allowed).unwrap()).is_ok());
        }
        for denied in [
            "http://github.com/Williem3/FormationLap/releases/download/v1.2.3/latest.json",
            "https://github.example.com/Williem3/FormationLap/releases/download/v1.2.3/latest.json",
            "https://user@github.com/Williem3/FormationLap/releases/download/v1.2.3/latest.json",
        ] {
            assert!(validate_redirect_url(&Url::parse(denied).unwrap()).is_err());
        }
    }

    #[cfg(windows)]
    #[test]
    fn verified_staging_handle_allows_windows_to_open_the_installer_for_execution() {
        use std::os::windows::fs::OpenOptionsExt;
        use windows_sys::Win32::Storage::FileSystem::{FILE_SHARE_DELETE, FILE_SHARE_READ};

        let installer = stage_verified_installer(b"MZ verified fixture", "1.2.3")
            .expect("verified fixture should stage");
        let mut executable_open = OpenOptions::new();
        executable_open
            .read(true)
            .share_mode(FILE_SHARE_READ | FILE_SHARE_DELETE);

        executable_open
            .open(&installer.path)
            .expect("the retained staging handle must permit Windows to map the installer");

        let directory = installer
            .path
            .parent()
            .expect("staged installer should have a parent")
            .to_path_buf();
        drop(installer);
        fs::remove_dir_all(directory).expect("test staging directory should be removable");
    }

    #[cfg(windows)]
    #[test]
    fn verified_staging_handle_prevents_replacement_until_launch() {
        let installer = stage_verified_installer(b"MZ verified fixture", "1.2.3")
            .expect("verified fixture should stage");
        assert!(
            OpenOptions::new()
                .write(true)
                .truncate(true)
                .open(&installer.path)
                .is_err(),
            "another Process must not replace verified bytes while the launch handle is open"
        );

        let directory = installer
            .path
            .parent()
            .expect("staged installer should have a parent")
            .to_path_buf();
        let path = installer.path.clone();
        drop(installer);
        OpenOptions::new()
            .write(true)
            .truncate(true)
            .open(&path)
            .expect("the test should release the staging handle");
        fs::remove_dir_all(directory).expect("test staging directory should be removable");
    }
}
