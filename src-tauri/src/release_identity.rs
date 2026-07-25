use crate::ELEVATED_HELPER_PROTOCOL_VERSION;
use base64::{Engine as _, engine::general_purpose::STANDARD};
use minisign_verify::{PublicKey, Signature};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    error::Error,
    fmt, fs,
    io::Read,
    path::{Path, PathBuf},
};

const RELEASE_IDENTITY_SCHEMA_VERSION: u16 = 1;
const RELEASE_IDENTITY_FILE: &str = "formation-lap-release-identity.json";
const MAX_RELEASE_IDENTITY_BYTES: u64 = 16_384;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ReleaseIdentityError(String);

impl ReleaseIdentityError {
    fn new(message: impl Into<String>) -> Self {
        Self(message.into())
    }
}

impl fmt::Display for ReleaseIdentityError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl Error for ReleaseIdentityError {}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ReleaseIdentityManifest {
    schema_version: u16,
    main_executable_sha256: String,
    helper_sha256: String,
    version: String,
    protocol_version: u16,
    release_channel: String,
    signature: String,
}

impl ReleaseIdentityManifest {
    fn signing_payload(&self) -> String {
        format!(
            concat!(
                "formation-lap-release-identity-v1\n",
                "mainExecutableSha256={}\n",
                "helperSha256={}\n",
                "version={}\n",
                "protocolVersion={}\n",
                "releaseChannel={}\n"
            ),
            self.main_executable_sha256,
            self.helper_sha256,
            self.version,
            self.protocol_version,
            self.release_channel,
        )
    }
}

pub(crate) fn validate_expected_application_pair(
    main_executable: &Path,
    helper_executable: &Path,
) -> Result<(PathBuf, PathBuf), ReleaseIdentityError> {
    let main = canonical_file(main_executable, "Formation Lap executable")?;
    let helper = canonical_file(helper_executable, "elevated helper")?;
    if !file_name_matches(&main, "formation-lap.exe") {
        return Err(ReleaseIdentityError::new(
            "caller is not named formation-lap.exe",
        ));
    }
    if !file_name_matches(&helper, "formation-lap-elevated-helper.exe") {
        return Err(ReleaseIdentityError::new(
            "helper has an unexpected executable name",
        ));
    }
    if main.parent() != helper.parent() {
        return Err(ReleaseIdentityError::new(
            "Formation Lap and its elevated helper are not canonical siblings",
        ));
    }
    Ok((main, helper))
}

pub(crate) fn verify_runtime_release_identity(
    main_executable: &Path,
    helper_executable: &Path,
) -> Result<(), ReleaseIdentityError> {
    let (main, helper) = validate_expected_application_pair(main_executable, helper_executable)?;

    if cfg!(debug_assertions) {
        return Ok(());
    }

    let channel = option_env!("FORMATION_LAP_RELEASE_CHANNEL").ok_or_else(|| {
        ReleaseIdentityError::new("release channel is not compiled into this build")
    })?;
    match channel {
        "preview" => verify_preview_release_identity(&main, &helper),
        "beta" | "stable" => Err(ReleaseIdentityError::new(
            "signed release identity verification is not available",
        )),
        _ => Err(ReleaseIdentityError::new(
            "compiled release channel is not recognized",
        )),
    }
}

fn verify_preview_release_identity(
    main_executable: &Path,
    helper_executable: &Path,
) -> Result<(), ReleaseIdentityError> {
    let public_key = decode_public_key(
        option_env!("FORMATION_LAP_RELEASE_IDENTITY_PUBLIC_KEY")
            .ok_or_else(|| ReleaseIdentityError::new("release identity public key is missing"))?,
    )?;
    let installation_directory = main_executable.parent().ok_or_else(|| {
        ReleaseIdentityError::new("Formation Lap executable has no installation directory")
    })?;
    let manifest_path = installation_directory.join(RELEASE_IDENTITY_FILE);
    verify_preview_manifest_with(
        &manifest_path,
        main_executable,
        helper_executable,
        env!("CARGO_PKG_VERSION"),
        "preview",
        |payload, signature| {
            public_key
                .verify(payload, signature, true)
                .map_err(|_| ReleaseIdentityError::new("release identity signature was rejected"))
        },
    )
}

fn verify_preview_manifest_with(
    manifest_path: &Path,
    main_executable: &Path,
    helper_executable: &Path,
    expected_version: &str,
    expected_channel: &str,
    verify_signature: impl FnOnce(&[u8], &Signature) -> Result<(), ReleaseIdentityError>,
) -> Result<(), ReleaseIdentityError> {
    let manifest_bytes = read_bounded_file(manifest_path, MAX_RELEASE_IDENTITY_BYTES)?;
    let manifest: ReleaseIdentityManifest = serde_json::from_slice(&manifest_bytes)
        .map_err(|_| ReleaseIdentityError::new("release identity manifest is invalid"))?;
    if manifest.schema_version != RELEASE_IDENTITY_SCHEMA_VERSION
        || manifest.version != expected_version
        || manifest.protocol_version != ELEVATED_HELPER_PROTOCOL_VERSION
        || manifest.release_channel != expected_channel
    {
        return Err(ReleaseIdentityError::new(
            "release identity metadata does not match this build",
        ));
    }
    validate_sha256(&manifest.main_executable_sha256)?;
    validate_sha256(&manifest.helper_sha256)?;
    let signature = decode_signature(&manifest.signature)?;
    verify_signature(manifest.signing_payload().as_bytes(), &signature)?;

    if sha256_file(main_executable)? != manifest.main_executable_sha256 {
        return Err(ReleaseIdentityError::new(
            "Formation Lap executable hash does not match the release identity",
        ));
    }
    if sha256_file(helper_executable)? != manifest.helper_sha256 {
        return Err(ReleaseIdentityError::new(
            "elevated helper hash does not match the release identity",
        ));
    }
    Ok(())
}

fn canonical_file(path: &Path, label: &str) -> Result<PathBuf, ReleaseIdentityError> {
    let canonical = fs::canonicalize(path)
        .map_err(|error| ReleaseIdentityError::new(format!("{label} is unavailable: {error}")))?;
    if !canonical.is_file() {
        return Err(ReleaseIdentityError::new(format!(
            "{label} is not a regular file"
        )));
    }
    Ok(canonical)
}

fn file_name_matches(path: &Path, expected: &str) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.eq_ignore_ascii_case(expected))
}

fn read_bounded_file(path: &Path, maximum: u64) -> Result<Vec<u8>, ReleaseIdentityError> {
    let file = fs::File::open(path).map_err(|error| {
        ReleaseIdentityError::new(format!("release identity is missing: {error}"))
    })?;
    let metadata = file.metadata().map_err(|error| {
        ReleaseIdentityError::new(format!("release identity metadata is unavailable: {error}"))
    })?;
    if metadata.len() > maximum {
        return Err(ReleaseIdentityError::new(
            "release identity exceeds its size limit",
        ));
    }
    let mut bytes = Vec::with_capacity(usize::try_from(metadata.len()).unwrap_or(0));
    file.take(maximum + 1)
        .read_to_end(&mut bytes)
        .map_err(|error| {
            ReleaseIdentityError::new(format!("release identity could not be read: {error}"))
        })?;
    if u64::try_from(bytes.len()).unwrap_or(u64::MAX) > maximum {
        return Err(ReleaseIdentityError::new(
            "release identity exceeds its size limit",
        ));
    }
    Ok(bytes)
}

fn sha256_file(path: &Path) -> Result<String, ReleaseIdentityError> {
    let mut file = fs::File::open(path).map_err(|error| {
        ReleaseIdentityError::new(format!("release executable could not be opened: {error}"))
    })?;
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let read = file.read(&mut buffer).map_err(|error| {
            ReleaseIdentityError::new(format!("release executable could not be hashed: {error}"))
        })?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

fn validate_sha256(value: &str) -> Result<(), ReleaseIdentityError> {
    if value.len() != 64
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(ReleaseIdentityError::new(
            "release identity contains an invalid SHA-256 digest",
        ));
    }
    Ok(())
}

fn decode_public_key(encoded: &str) -> Result<PublicKey, ReleaseIdentityError> {
    let decoded = STANDARD
        .decode(encoded)
        .map_err(|_| ReleaseIdentityError::new("release identity public key is not Base64"))?;
    let decoded = std::str::from_utf8(&decoded)
        .map_err(|_| ReleaseIdentityError::new("release identity public key is not UTF-8"))?;
    PublicKey::decode(decoded)
        .map_err(|_| ReleaseIdentityError::new("release identity public key is invalid"))
}

fn decode_signature(encoded: &str) -> Result<Signature, ReleaseIdentityError> {
    let decoded = STANDARD
        .decode(encoded)
        .map_err(|_| ReleaseIdentityError::new("release identity signature is not Base64"))?;
    let decoded = std::str::from_utf8(&decoded)
        .map_err(|_| ReleaseIdentityError::new("release identity signature is not UTF-8"))?;
    Signature::decode(decoded)
        .map_err(|_| ReleaseIdentityError::new("release identity signature is invalid"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    const PUBLIC_KEY_TEXT: &str = "untrusted comment: minisign public key E7620F1842B4E81F\n\
RWQf6LRCGA9i53mlYecO4IzT51TGPpvWucNSCh1CBM0QTaLn73Y7GFO3";
    const SIGNATURE_TEXT: &str = "untrusted comment: signature from minisign secret key\n\
RWQf6LRCGA9i59SLOFxz6NxvASXDJeRtuZykwQepbDEGt87ig1BNpWaVWuNrm73YiIiJbq71Wi+dP9eKL8OC351vwIasSSbXxwA=\n\
trusted comment: timestamp:1555779966\tfile:test\n\
QtKMXWyYcwdpZAlPF7tE2ENJkRd1ujvKjlj1m9RtHTBnZPa5WKU5uWRs5GoP5M/VqE81QFuMKI5k/SfNQUaOAA==";
    const RELEASE_IDENTITY_PUBLIC_KEY: &str = "dW50cnVzdGVkIGNvbW1lbnQ6IG1pbmlzaWduIHB1YmxpYyBrZXk6IDhDM0M1RTU4OTZGMEE0OUEKUldTYXBQQ1dXRjQ4akxUbkVnVm8rcGRxKzloOXhlWFNiaVF3c0phcTFjNHRia1oxNWg1VVRPVmkK";
    const RELEASE_IDENTITY_SIGNATURE: &str = "dW50cnVzdGVkIGNvbW1lbnQ6IHNpZ25hdHVyZSBmcm9tIHRhdXJpIHNlY3JldCBrZXkKUlVTYXBQQ1dXRjQ4akpvaktNUEpSQXpqclBCZHVFK2liUkRVRzhoWDZiOUlZMjRBajdrYlpsSHlyUTI4T1JtUGVPS0ZrSGpHa3hZalRXWDlhOEJRQUM5TzF4djNDSGNzWWdnPQp0cnVzdGVkIGNvbW1lbnQ6IHRpbWVzdGFtcDoxNzg0OTUwOTYwCWZpbGU6cmVsZWFzZV9pZGVudGl0eV9wYXlsb2FkLnR4dApYdmdBSytvU2laTUdNUi9kUktpeTloQm1zUHBXMUhtWFM0RTNSSGwzZWdNV0NFRmthbm1lRTlSaldPNzFzWUpCRWdsbmJmcWt6VFBJa0xVUFRSMnZEUT09Cg==";

    struct TempDirectory(PathBuf);

    impl TempDirectory {
        fn new() -> Self {
            let unique = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("test clock should follow the Unix epoch")
                .as_nanos();
            let path = std::env::temp_dir().join(format!(
                "formation-lap-release-identity-{}-{unique}",
                std::process::id()
            ));
            fs::create_dir_all(&path).expect("temporary identity directory should be created");
            Self(path)
        }
    }

    impl Drop for TempDirectory {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn known_minisign_fixture_is_accepted_and_tampering_is_rejected() {
        let public_key = PublicKey::decode(PUBLIC_KEY_TEXT).expect("fixture key should decode");
        let signature = Signature::decode(SIGNATURE_TEXT).expect("fixture signature should decode");
        assert!(public_key.verify(b"test", &signature, true).is_ok());
        assert!(public_key.verify(b"tampered", &signature, true).is_err());
    }

    #[test]
    fn preview_manifest_binds_both_executable_hashes_and_build_metadata() {
        let temporary = TempDirectory::new();
        let main = temporary.0.join("formation-lap.exe");
        let helper = temporary.0.join("formation-lap-elevated-helper.exe");
        let manifest_path = temporary.0.join(RELEASE_IDENTITY_FILE);
        fs::write(&main, b"main release bytes").expect("main fixture should write");
        fs::write(&helper, b"helper release bytes").expect("helper fixture should write");
        let manifest = ReleaseIdentityManifest {
            schema_version: RELEASE_IDENTITY_SCHEMA_VERSION,
            main_executable_sha256: sha256_file(&main).expect("main fixture should hash"),
            helper_sha256: sha256_file(&helper).expect("helper fixture should hash"),
            version: "0.9.0-preview.1".to_owned(),
            protocol_version: ELEVATED_HELPER_PROTOCOL_VERSION,
            release_channel: "preview".to_owned(),
            signature: RELEASE_IDENTITY_SIGNATURE.to_owned(),
        };
        assert_eq!(
            manifest.signing_payload(),
            include_str!("../tests/fixtures/release_identity_payload.txt")
        );
        fs::write(
            &manifest_path,
            serde_json::to_vec(&manifest).expect("manifest should serialize"),
        )
        .expect("manifest fixture should write");

        let public_key =
            decode_public_key(RELEASE_IDENTITY_PUBLIC_KEY).expect("fixture key should decode");
        verify_preview_manifest_with(
            &manifest_path,
            &main,
            &helper,
            "0.9.0-preview.1",
            "preview",
            |payload, signature| {
                public_key.verify(payload, signature, true).map_err(|_| {
                    ReleaseIdentityError::new("fixture release identity signature was rejected")
                })
            },
        )
        .expect("matching executable bytes and metadata should be accepted");

        fs::write(&helper, b"tampered helper bytes").expect("helper fixture should be tampered");
        assert!(
            verify_preview_manifest_with(
                &manifest_path,
                &main,
                &helper,
                "0.9.0-preview.1",
                "preview",
                |_payload, _signature| Ok(()),
            )
            .expect_err("tampered helper bytes must be rejected")
            .to_string()
            .contains("helper hash")
        );
    }

    #[test]
    fn application_pair_requires_exact_canonical_sibling_names() {
        let temporary = TempDirectory::new();
        let main = temporary.0.join("formation-lap.exe");
        let helper = temporary.0.join("formation-lap-elevated-helper.exe");
        fs::write(&main, b"main").expect("main fixture should write");
        fs::write(&helper, b"helper").expect("helper fixture should write");
        validate_expected_application_pair(&main, &helper)
            .expect("the exact canonical sibling pair should validate");

        let renamed = temporary.0.join("renamed-main.exe");
        fs::write(&renamed, b"main").expect("renamed fixture should write");
        assert!(validate_expected_application_pair(&renamed, &helper).is_err());
    }
}
