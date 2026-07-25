use crate::ELEVATED_HELPER_PROTOCOL_VERSION;
use base64::{Engine as _, engine::general_purpose::STANDARD};
use minisign_verify::{PublicKey, Signature};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    error::Error,
    fmt, fs,
    io::{Read, Seek, SeekFrom},
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    authenticode_signer_sha256: Option<String>,
    signature: String,
}

impl ReleaseIdentityManifest {
    fn signing_payload(&self) -> String {
        let mut payload = format!(
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
        );
        if let Some(signer) = &self.authenticode_signer_sha256 {
            payload.push_str(&format!("authenticodeSignerSha256={signer}\n"));
        }
        payload
    }
}

pub(crate) struct VerifiedReleaseIdentity {
    _main_executable: fs::File,
    _helper_executable: fs::File,
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
) -> Result<VerifiedReleaseIdentity, ReleaseIdentityError> {
    let (main, helper) = validate_expected_application_pair(main_executable, helper_executable)?;
    let mut main_file = open_locked_executable(&main, "Formation Lap executable")?;
    let mut helper_file = open_locked_executable(&helper, "elevated helper")?;

    if cfg!(debug_assertions) {
        return Ok(VerifiedReleaseIdentity {
            _main_executable: main_file,
            _helper_executable: helper_file,
        });
    }

    let channel = option_env!("FORMATION_LAP_RELEASE_CHANNEL").ok_or_else(|| {
        ReleaseIdentityError::new("release channel is not compiled into this build")
    })?;
    match channel {
        "preview" => verify_preview_release_identity(&main, &mut main_file, &mut helper_file)?,
        "beta" | "stable" => verify_signed_release_identity(
            &main,
            &mut main_file,
            &helper,
            &mut helper_file,
            channel,
        )?,
        _ => Err(ReleaseIdentityError::new(
            "compiled release channel is not recognized",
        ))?,
    }
    Ok(VerifiedReleaseIdentity {
        _main_executable: main_file,
        _helper_executable: helper_file,
    })
}

fn verify_preview_release_identity(
    main_executable: &Path,
    main_file: &mut fs::File,
    helper_file: &mut fs::File,
) -> Result<(), ReleaseIdentityError> {
    let public_key = decode_public_key(
        option_env!("FORMATION_LAP_RELEASE_IDENTITY_PUBLIC_KEY")
            .ok_or_else(|| ReleaseIdentityError::new("release identity public key is missing"))?,
    )?;
    let installation_directory = main_executable.parent().ok_or_else(|| {
        ReleaseIdentityError::new("Formation Lap executable has no installation directory")
    })?;
    let manifest_path = installation_directory.join(RELEASE_IDENTITY_FILE);
    let manifest = verify_release_manifest_with(
        &manifest_path,
        main_file,
        helper_file,
        env!("CARGO_PKG_VERSION"),
        "preview",
        |payload, signature| {
            public_key
                .verify(payload, signature, true)
                .map_err(|_| ReleaseIdentityError::new("release identity signature was rejected"))
        },
    )?;
    if manifest.authenticode_signer_sha256.is_some() {
        return Err(ReleaseIdentityError::new(
            "preview release identity must not claim an Authenticode signer",
        ));
    }
    Ok(())
}

fn verify_signed_release_identity(
    main_executable: &Path,
    main_file: &mut fs::File,
    helper_executable: &Path,
    helper_file: &mut fs::File,
    expected_channel: &str,
) -> Result<(), ReleaseIdentityError> {
    #[cfg(windows)]
    {
        let public_key = decode_public_key(
            option_env!("FORMATION_LAP_RELEASE_IDENTITY_PUBLIC_KEY").ok_or_else(|| {
                ReleaseIdentityError::new("release identity public key is missing")
            })?,
        )?;
        let installation_directory = main_executable.parent().ok_or_else(|| {
            ReleaseIdentityError::new("Formation Lap executable has no installation directory")
        })?;
        let manifest_path = installation_directory.join(RELEASE_IDENTITY_FILE);
        verify_signed_release_identity_with(
            &manifest_path,
            main_executable,
            main_file,
            helper_executable,
            helper_file,
            env!("CARGO_PKG_VERSION"),
            expected_channel,
            |payload, signature| {
                public_key.verify(payload, signature, true).map_err(|_| {
                    ReleaseIdentityError::new("release identity signature was rejected")
                })
            },
            authenticode_signer_sha256,
        )
    }
    #[cfg(not(windows))]
    {
        let _ = (
            main_executable,
            main_file,
            helper_executable,
            helper_file,
            expected_channel,
        );
        Err(ReleaseIdentityError::new(
            "signed release identity requires Windows trust policy",
        ))
    }
}

#[allow(clippy::too_many_arguments)]
fn verify_signed_release_identity_with(
    manifest_path: &Path,
    main_executable: &Path,
    main_file: &mut fs::File,
    helper_executable: &Path,
    helper_file: &mut fs::File,
    expected_version: &str,
    expected_channel: &str,
    verify_signature: impl FnOnce(&[u8], &Signature) -> Result<(), ReleaseIdentityError>,
    mut verify_authenticode: impl FnMut(&Path, &fs::File) -> Result<String, ReleaseIdentityError>,
) -> Result<(), ReleaseIdentityError> {
    let manifest = verify_release_manifest_with(
        manifest_path,
        main_file,
        helper_file,
        expected_version,
        expected_channel,
        verify_signature,
    )?;
    let approved_signer = manifest.authenticode_signer_sha256.ok_or_else(|| {
        ReleaseIdentityError::new("signed release identity has no approved Authenticode signer")
    })?;
    validate_sha256(&approved_signer)?;

    let main_signer = verify_authenticode(main_executable, main_file)?;
    let helper_signer = verify_authenticode(helper_executable, helper_file)?;
    validate_sha256(&main_signer)?;
    validate_sha256(&helper_signer)?;
    if main_signer != helper_signer {
        return Err(ReleaseIdentityError::new(
            "Formation Lap and its elevated helper have different Authenticode signers",
        ));
    }
    if main_signer != approved_signer {
        return Err(ReleaseIdentityError::new(
            "Authenticode signer is not approved by the release identity",
        ));
    }
    Ok(())
}

fn verify_release_manifest_with(
    manifest_path: &Path,
    main_file: &mut fs::File,
    helper_file: &mut fs::File,
    expected_version: &str,
    expected_channel: &str,
    verify_signature: impl FnOnce(&[u8], &Signature) -> Result<(), ReleaseIdentityError>,
) -> Result<ReleaseIdentityManifest, ReleaseIdentityError> {
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

    if sha256_open_file(main_file)? != manifest.main_executable_sha256 {
        return Err(ReleaseIdentityError::new(
            "Formation Lap executable hash does not match the release identity",
        ));
    }
    if sha256_open_file(helper_file)? != manifest.helper_sha256 {
        return Err(ReleaseIdentityError::new(
            "elevated helper hash does not match the release identity",
        ));
    }
    Ok(manifest)
}

fn open_locked_executable(path: &Path, label: &str) -> Result<fs::File, ReleaseIdentityError> {
    let mut options = fs::OpenOptions::new();
    options.read(true);
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        use windows_sys::Win32::Storage::FileSystem::FILE_SHARE_READ;
        options.share_mode(FILE_SHARE_READ);
    }
    options.open(path).map_err(|error| {
        ReleaseIdentityError::new(format!(
            "{label} could not be locked for verification: {error}"
        ))
    })
}

#[cfg(windows)]
fn authenticode_signer_sha256(
    path: &Path,
    file: &fs::File,
) -> Result<String, ReleaseIdentityError> {
    use std::{
        ffi::c_void,
        os::windows::{ffi::OsStrExt, io::AsRawHandle},
    };
    use windows_sys::Win32::Security::WinTrust::{
        WINTRUST_ACTION_GENERIC_VERIFY_V2, WINTRUST_DATA, WINTRUST_FILE_INFO,
        WTD_CACHE_ONLY_URL_RETRIEVAL, WTD_CHOICE_FILE, WTD_REVOCATION_CHECK_NONE, WTD_REVOKE_NONE,
        WTD_STATEACTION_CLOSE, WTD_STATEACTION_VERIFY, WTD_UI_NONE, WTD_UICONTEXT_EXECUTE,
        WTHelperGetProvCertFromChain, WTHelperGetProvSignerFromChain,
        WTHelperProvDataFromStateData, WinVerifyTrust,
    };

    let wide_path = path
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect::<Vec<_>>();
    let mut file_info = WINTRUST_FILE_INFO {
        cbStruct: std::mem::size_of::<WINTRUST_FILE_INFO>() as u32,
        pcwszFilePath: wide_path.as_ptr(),
        hFile: file.as_raw_handle(),
        pgKnownSubject: std::ptr::null_mut(),
    };
    let mut trust_data = WINTRUST_DATA {
        cbStruct: std::mem::size_of::<WINTRUST_DATA>() as u32,
        dwUIChoice: WTD_UI_NONE,
        fdwRevocationChecks: WTD_REVOKE_NONE,
        dwUnionChoice: WTD_CHOICE_FILE,
        Anonymous: windows_sys::Win32::Security::WinTrust::WINTRUST_DATA_0 {
            pFile: &mut file_info,
        },
        dwStateAction: WTD_STATEACTION_VERIFY,
        dwProvFlags: WTD_REVOCATION_CHECK_NONE | WTD_CACHE_ONLY_URL_RETRIEVAL,
        dwUIContext: WTD_UICONTEXT_EXECUTE,
        ..WINTRUST_DATA::default()
    };
    let mut action = WINTRUST_ACTION_GENERIC_VERIFY_V2;
    // SAFETY: `trust_data` points to a stable WINTRUST_FILE_INFO whose path and
    // verified read-only file handle remain live through verification and
    // state cleanup. The generic Authenticode policy GUID is mutable only
    // because WinVerifyTrust's ABI predates const-correctness.
    let status = unsafe {
        WinVerifyTrust(
            std::ptr::null_mut(),
            &mut action,
            (&mut trust_data as *mut WINTRUST_DATA).cast::<c_void>(),
        )
    };
    let signer = if status == 0 {
        // SAFETY: Successful stateful WinVerifyTrust verification owns the
        // provider chain until WTD_STATEACTION_CLOSE below. Every pointer and
        // encoded length is checked before constructing the certificate view.
        unsafe {
            let provider = WTHelperProvDataFromStateData(trust_data.hWVTStateData);
            if provider.is_null() {
                Err(ReleaseIdentityError::new(
                    "WinVerifyTrust did not return provider data",
                ))
            } else {
                let signer = WTHelperGetProvSignerFromChain(provider, 0, 0, 0);
                if signer.is_null() {
                    Err(ReleaseIdentityError::new(
                        "WinVerifyTrust did not return a primary signer",
                    ))
                } else {
                    let provider_certificate = WTHelperGetProvCertFromChain(signer, 0);
                    if provider_certificate.is_null()
                        || (*provider_certificate).pCert.is_null()
                        || (*(*provider_certificate).pCert).pbCertEncoded.is_null()
                        || (*(*provider_certificate).pCert).cbCertEncoded == 0
                    {
                        Err(ReleaseIdentityError::new(
                            "WinVerifyTrust did not return a signer certificate",
                        ))
                    } else {
                        let certificate = &*(*provider_certificate).pCert;
                        let encoded = std::slice::from_raw_parts(
                            certificate.pbCertEncoded,
                            certificate.cbCertEncoded as usize,
                        );
                        Ok(format!("{:x}", Sha256::digest(encoded)))
                    }
                }
            }
        }
    } else {
        Err(ReleaseIdentityError::new(format!(
            "WinVerifyTrust rejected {} with status 0x{:08x}",
            path.file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("release executable"),
            status as u32
        )))
    };
    trust_data.dwStateAction = WTD_STATEACTION_CLOSE;
    // SAFETY: This closes only the state allocated by the matching
    // WinVerifyTrust call above; all backing structures are still alive.
    unsafe {
        WinVerifyTrust(
            std::ptr::null_mut(),
            &mut action,
            (&mut trust_data as *mut WINTRUST_DATA).cast::<c_void>(),
        );
    }
    signer
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

#[cfg(test)]
fn sha256_file(path: &Path) -> Result<String, ReleaseIdentityError> {
    let mut file = fs::File::open(path).map_err(|error| {
        ReleaseIdentityError::new(format!("release executable could not be opened: {error}"))
    })?;
    sha256_open_file(&mut file)
}

fn sha256_open_file(file: &mut fs::File) -> Result<String, ReleaseIdentityError> {
    file.seek(SeekFrom::Start(0)).map_err(|error| {
        ReleaseIdentityError::new(format!("release executable could not be rewound: {error}"))
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
    file.seek(SeekFrom::Start(0)).map_err(|error| {
        ReleaseIdentityError::new(format!("release executable could not be rewound: {error}"))
    })?;
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
            authenticode_signer_sha256: None,
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
        let mut main_file =
            open_locked_executable(&main, "main fixture").expect("main fixture should open");
        let mut helper_file =
            open_locked_executable(&helper, "helper fixture").expect("helper fixture should open");
        let verified = verify_release_manifest_with(
            &manifest_path,
            &mut main_file,
            &mut helper_file,
            "0.9.0-preview.1",
            "preview",
            |payload, signature| {
                public_key.verify(payload, signature, true).map_err(|_| {
                    ReleaseIdentityError::new("fixture release identity signature was rejected")
                })
            },
        )
        .expect("matching executable bytes and metadata should be accepted");
        assert!(verified.authenticode_signer_sha256.is_none());

        drop((main_file, helper_file));
        fs::write(&helper, b"tampered helper bytes").expect("helper fixture should be tampered");
        let mut main_file =
            open_locked_executable(&main, "main fixture").expect("main fixture should reopen");
        let mut helper_file = open_locked_executable(&helper, "helper fixture")
            .expect("helper fixture should reopen");
        assert!(
            verify_release_manifest_with(
                &manifest_path,
                &mut main_file,
                &mut helper_file,
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
    fn signed_manifest_requires_the_same_release_approved_authenticode_signer() {
        let temporary = TempDirectory::new();
        let main = temporary.0.join("formation-lap.exe");
        let helper = temporary.0.join("formation-lap-elevated-helper.exe");
        let manifest_path = temporary.0.join(RELEASE_IDENTITY_FILE);
        fs::write(&main, b"signed main release bytes").expect("main fixture should write");
        fs::write(&helper, b"signed helper release bytes").expect("helper fixture should write");
        let approved_signer = "a".repeat(64);
        let manifest = ReleaseIdentityManifest {
            schema_version: RELEASE_IDENTITY_SCHEMA_VERSION,
            main_executable_sha256: sha256_file(&main).expect("main fixture should hash"),
            helper_sha256: sha256_file(&helper).expect("helper fixture should hash"),
            version: "1.0.0-beta.1".to_owned(),
            protocol_version: ELEVATED_HELPER_PROTOCOL_VERSION,
            release_channel: "beta".to_owned(),
            authenticode_signer_sha256: Some(approved_signer.clone()),
            signature: RELEASE_IDENTITY_SIGNATURE.to_owned(),
        };
        assert!(
            manifest
                .signing_payload()
                .contains(&format!("authenticodeSignerSha256={approved_signer}\n"))
        );
        fs::write(
            &manifest_path,
            serde_json::to_vec(&manifest).expect("manifest should serialize"),
        )
        .expect("manifest fixture should write");

        let open_pair = || {
            (
                open_locked_executable(&main, "main fixture").expect("main fixture should open"),
                open_locked_executable(&helper, "helper fixture")
                    .expect("helper fixture should open"),
            )
        };
        let (mut main_file, mut helper_file) = open_pair();
        verify_signed_release_identity_with(
            &manifest_path,
            &main,
            &mut main_file,
            &helper,
            &mut helper_file,
            "1.0.0-beta.1",
            "beta",
            |_payload, _signature| Ok(()),
            |_path, _file| Ok(approved_signer.clone()),
        )
        .expect("equal trusted signers approved by the manifest should pass");
        drop((main_file, helper_file));

        let (mut main_file, mut helper_file) = open_pair();
        let error = verify_signed_release_identity_with(
            &manifest_path,
            &main,
            &mut main_file,
            &helper,
            &mut helper_file,
            "1.0.0-beta.1",
            "beta",
            |_payload, _signature| Ok(()),
            |path, _file| {
                if file_name_matches(path, "formation-lap.exe") {
                    Ok(approved_signer.clone())
                } else {
                    Ok("b".repeat(64))
                }
            },
        )
        .expect_err("different trusted signer certificates must be rejected");
        assert!(error.to_string().contains("different Authenticode signers"));
        drop((main_file, helper_file));

        let (mut main_file, mut helper_file) = open_pair();
        let error = verify_signed_release_identity_with(
            &manifest_path,
            &main,
            &mut main_file,
            &helper,
            &mut helper_file,
            "1.0.0-beta.1",
            "beta",
            |_payload, _signature| Ok(()),
            |_path, _file| Ok("b".repeat(64)),
        )
        .expect_err("an equal but unapproved trusted signer must be rejected");
        assert!(error.to_string().contains("not approved"));
        drop((main_file, helper_file));

        let (mut main_file, mut helper_file) = open_pair();
        let error = verify_signed_release_identity_with(
            &manifest_path,
            &main,
            &mut main_file,
            &helper,
            &mut helper_file,
            "1.0.0-beta.1",
            "beta",
            |_payload, _signature| Ok(()),
            |_path, _file| Err(ReleaseIdentityError::new("WinVerifyTrust rejected fixture")),
        )
        .expect_err("an untrusted signature must be rejected before equality");
        assert!(error.to_string().contains("WinVerifyTrust rejected"));
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

    #[cfg(windows)]
    #[test]
    fn verified_pair_handles_prevent_sibling_replacement_through_launch() {
        let temporary = TempDirectory::new();
        let main = temporary.0.join("formation-lap.exe");
        let helper = temporary.0.join("formation-lap-elevated-helper.exe");
        fs::write(&main, b"main").expect("main fixture should write");
        fs::write(&helper, b"helper").expect("helper fixture should write");

        let verified = verify_runtime_release_identity(&main, &helper)
            .expect("debug identity pair should lock");
        assert!(
            fs::OpenOptions::new()
                .write(true)
                .truncate(true)
                .open(&helper)
                .is_err(),
            "the helper must not be replaceable between verification and UAC launch"
        );
        drop(verified);
        fs::OpenOptions::new()
            .write(true)
            .truncate(true)
            .open(&helper)
            .expect("dropping the verified pair should release the test lock");
    }
}
