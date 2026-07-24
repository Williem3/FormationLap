use crate::{ApplicationUpdateSnapshot, CatalogUpdateProvider, UpdateCheckPlan, UpdateStatus};
use serde::Deserialize;
use std::{
    io::Read,
    process::{Command, Stdio},
    time::{Duration, Instant},
};

const MAX_PROVIDER_RESPONSE_BYTES: u64 = 1_048_576;
const PROVIDER_TIMEOUT: Duration = Duration::from_secs(15);

pub(crate) struct DirectUpdateProviderRuntime {
    client: reqwest::blocking::Client,
}

impl DirectUpdateProviderRuntime {
    pub(crate) fn new() -> Result<Self, String> {
        let client = reqwest::blocking::Client::builder()
            .https_only(true)
            .redirect(reqwest::redirect::Policy::none())
            .timeout(PROVIDER_TIMEOUT)
            .user_agent(concat!("Formation-Lap/", env!("CARGO_PKG_VERSION")))
            .build()
            .map_err(|error| format!("update HTTP client could not be created: {error}"))?;
        Ok(Self { client })
    }
}

impl UpdateProviderRuntime for DirectUpdateProviderRuntime {
    fn get_https(&self, url: &str, allowed_hosts: &[&str]) -> Result<String, String> {
        let mut current =
            reqwest::Url::parse(url).map_err(|_| "provider URL is invalid".to_owned())?;
        for _ in 0..=3 {
            validate_direct_url(&current, allowed_hosts)?;
            let response = self
                .client
                .get(current.clone())
                .header("Accept", "application/vnd.github+json, text/html")
                .header("X-GitHub-Api-Version", "2022-11-28")
                .send()
                .map_err(|error| format!("provider request failed: {error}"))?;
            if response.status().is_redirection() {
                let location = response
                    .headers()
                    .get(reqwest::header::LOCATION)
                    .and_then(|value| value.to_str().ok())
                    .ok_or_else(|| {
                        "provider redirect did not include a valid location".to_owned()
                    })?;
                current = current
                    .join(location)
                    .map_err(|_| "provider redirect URL is invalid".to_owned())?;
                continue;
            }
            if !response.status().is_success() {
                return Err(format!(
                    "provider returned HTTP status {}",
                    response.status()
                ));
            }
            let mut bytes = Vec::new();
            response
                .take(MAX_PROVIDER_RESPONSE_BYTES + 1)
                .read_to_end(&mut bytes)
                .map_err(|error| format!("provider response could not be read: {error}"))?;
            if bytes.len() as u64 > MAX_PROVIDER_RESPONSE_BYTES {
                return Err("provider response exceeded the safe size limit".to_owned());
            }
            return String::from_utf8(bytes)
                .map_err(|_| "provider response was not UTF-8 text".to_owned());
        }
        Err("provider returned too many redirects".to_owned())
    }

    fn winget_list(&self, package_id: &str) -> Result<String, String> {
        winget_list(package_id)
    }

    fn file_version(&self, executable_path: &str) -> Option<String> {
        executable_file_version(executable_path)
    }
}

fn validate_direct_url(url: &reqwest::Url, allowed_hosts: &[&str]) -> Result<(), String> {
    let trusted = url.scheme() == "https"
        && url.username().is_empty()
        && url.password().is_none()
        && allowed_hosts.iter().any(|host| {
            url.host_str()
                .is_some_and(|value| value.eq_ignore_ascii_case(host))
        });
    trusted
        .then_some(())
        .ok_or_else(|| "provider URL left its curated HTTPS origin".to_owned())
}

#[cfg(windows)]
fn winget_list(package_id: &str) -> Result<String, String> {
    use std::os::windows::process::CommandExt;
    use windows_sys::Win32::System::Threading::CREATE_NO_WINDOW;

    if package_id.is_empty()
        || !package_id
            .chars()
            .all(is_safe_provider_identifier_character)
    {
        return Err("Winget package ID is invalid".to_owned());
    }
    let mut child = Command::new("winget.exe")
        .args([
            "list",
            "--id",
            package_id,
            "--exact",
            "--source",
            "winget",
            "--disable-interactivity",
        ])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .creation_flags(CREATE_NO_WINDOW)
        .spawn()
        .map_err(|error| format!("Winget could not start: {error}"))?;
    let deadline = Instant::now() + PROVIDER_TIMEOUT;
    loop {
        match child
            .try_wait()
            .map_err(|error| format!("Winget status could not be read: {error}"))?
        {
            Some(status) if status.success() => {
                let output = child
                    .wait_with_output()
                    .map_err(|error| format!("Winget output could not be read: {error}"))?;
                return String::from_utf8(output.stdout)
                    .map_err(|_| "Winget output was not UTF-8 text".to_owned());
            }
            Some(_) => return Err("Winget did not return reliable package metadata".to_owned()),
            None if Instant::now() >= deadline => {
                let _ = child.kill();
                let _ = child.wait();
                return Err("Winget timed out".to_owned());
            }
            None => std::thread::sleep(Duration::from_millis(50)),
        }
    }
}

fn is_safe_provider_identifier_character(character: char) -> bool {
    character.is_ascii_alphanumeric() || matches!(character, '-' | '_' | '.')
}

#[cfg(not(windows))]
fn winget_list(_package_id: &str) -> Result<String, String> {
    Err("Winget is available only on Windows".to_owned())
}

#[cfg(windows)]
fn executable_file_version(executable_path: &str) -> Option<String> {
    use std::{ffi::c_void, os::windows::ffi::OsStrExt, path::Path};
    use windows_sys::Win32::Storage::FileSystem::{
        GetFileVersionInfoSizeW, GetFileVersionInfoW, VS_FIXEDFILEINFO, VerQueryValueW,
    };

    let canonical = Path::new(executable_path).canonicalize().ok()?;
    if !canonical
        .extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("exe"))
    {
        return None;
    }
    let wide = canonical
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect::<Vec<_>>();
    let mut unused = 0;
    // SAFETY: `wide` is a stable, null-terminated UTF-16 path and `unused`
    // remains valid for the duration of this Win32 query.
    let size = unsafe { GetFileVersionInfoSizeW(wide.as_ptr(), &mut unused) };
    if size == 0 {
        return None;
    }
    let mut data = vec![0_u8; size as usize];
    // SAFETY: `data` has exactly the size returned by Windows for this path.
    if unsafe { GetFileVersionInfoW(wide.as_ptr(), 0, size, data.as_mut_ptr().cast::<c_void>()) }
        == 0
    {
        return None;
    }
    let root = ['\\' as u16, 0];
    let mut value = std::ptr::null_mut::<c_void>();
    let mut value_size = 0;
    // SAFETY: Windows owns the queried view inside `data`; the returned pointer
    // is checked for null and sized as a fixed-version structure before use.
    if unsafe {
        VerQueryValueW(
            data.as_ptr().cast::<c_void>(),
            root.as_ptr(),
            &mut value,
            &mut value_size,
        )
    } == 0
        || value.is_null()
        || value_size < std::mem::size_of::<VS_FIXEDFILEINFO>() as u32
    {
        return None;
    }
    // SAFETY: The preceding Win32 size check establishes a valid fixed-version
    // structure for the lifetime of `data`.
    let info = unsafe { &*value.cast::<VS_FIXEDFILEINFO>() };
    let major = info.dwFileVersionMS >> 16;
    let minor = info.dwFileVersionMS & 0xffff;
    let build = info.dwFileVersionLS >> 16;
    let revision = info.dwFileVersionLS & 0xffff;
    Some(format!("{major}.{minor}.{build}.{revision}"))
}

#[cfg(not(windows))]
fn executable_file_version(_executable_path: &str) -> Option<String> {
    None
}

pub(crate) trait UpdateProviderRuntime {
    fn get_https(&self, url: &str, allowed_hosts: &[&str]) -> Result<String, String>;
    fn winget_list(&self, package_id: &str) -> Result<String, String>;
    fn file_version(&self, executable_path: &str) -> Option<String>;
}

pub(crate) struct UpdateProviderRunner<R> {
    runtime: R,
}

impl<R> UpdateProviderRunner<R>
where
    R: UpdateProviderRuntime,
{
    pub(crate) fn new(runtime: R) -> Self {
        Self { runtime }
    }

    pub(crate) fn check(&self, plan: &UpdateCheckPlan) -> Vec<ApplicationUpdateSnapshot> {
        plan.applications
            .iter()
            .map(|target| {
                let advice = match &target.provider {
                    Some(CatalogUpdateProvider::GitHubReleases { repository }) => {
                        let url =
                            format!("https://api.github.com/repos/{repository}/releases/latest");
                        match self.runtime.get_https(&url, &["api.github.com"]) {
                            Ok(response) => parse_github_release(
                                target
                                    .executable_path
                                    .as_deref()
                                    .and_then(|path| self.runtime.file_version(path))
                                    .as_deref(),
                                &response,
                            ),
                            Err(_) => ProviderAdvice::unknown(
                                "The trusted GitHub release provider could not be reached.",
                            ),
                        }
                    }
                    Some(CatalogUpdateProvider::Winget { package_id }) => {
                        match self.runtime.winget_list(package_id) {
                            Ok(response) => parse_winget_list(&response, package_id),
                            Err(_) => ProviderAdvice::unknown(
                                "Winget could not return reliable update information.",
                            ),
                        }
                    }
                    Some(CatalogUpdateProvider::OfficialPage { url }) => {
                        let allowed_host = tauri::Url::parse(url)
                            .ok()
                            .and_then(|url| url.host_str().map(str::to_owned));
                        match allowed_host
                            .as_deref()
                            .and_then(|host| self.runtime.get_https(url, &[host]).ok())
                        {
                            Some(response) => parse_simhub_page(
                                target
                                    .executable_path
                                    .as_deref()
                                    .and_then(|path| self.runtime.file_version(path))
                                    .as_deref(),
                                &response,
                                url,
                            ),
                            None => ProviderAdvice::unknown(
                                "The official update page could not be reached safely.",
                            ),
                        }
                    }
                    None => ProviderAdvice::unknown("No trusted update provider is configured."),
                };
                ApplicationUpdateSnapshot {
                    application_id: target.application_id.clone(),
                    name: target.name.clone(),
                    status: advice.status,
                    information_url: advice.information_url,
                }
            })
            .collect()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ProviderAdvice {
    pub(crate) status: UpdateStatus,
    pub(crate) information_url: Option<String>,
}

impl ProviderAdvice {
    fn unknown(reason: impl Into<String>) -> Self {
        Self {
            status: UpdateStatus::Unknown {
                reason: reason.into(),
            },
            information_url: None,
        }
    }
}

#[derive(Deserialize)]
struct GitHubRelease {
    tag_name: String,
    html_url: String,
}

pub(crate) fn parse_github_release(
    current_version: Option<&str>,
    response: &str,
) -> ProviderAdvice {
    let Some(current_version) = current_version.and_then(normalize_version) else {
        return ProviderAdvice::unknown("The installed version could not be read safely.");
    };
    let Ok(release) = serde_json::from_str::<GitHubRelease>(response) else {
        return ProviderAdvice::unknown(
            "The trusted release provider returned an invalid response.",
        );
    };
    let Some(latest_version) = normalize_version(&release.tag_name) else {
        return ProviderAdvice::unknown(
            "The trusted release provider returned an ambiguous version.",
        );
    };
    if !is_trusted_information_url(&release.html_url, &["github.com"]) {
        return ProviderAdvice::unknown(
            "The trusted release provider returned an invalid release link.",
        );
    }
    let Some(ordering) = compare_versions(&current_version, &latest_version) else {
        return ProviderAdvice::unknown(
            "The installed and available versions could not be compared.",
        );
    };
    let status = if ordering.is_lt() {
        UpdateStatus::UpdateAvailable {
            current_version,
            latest_version,
        }
    } else {
        UpdateStatus::Current { current_version }
    };
    ProviderAdvice {
        status,
        information_url: Some(release.html_url),
    }
}

pub(crate) fn parse_winget_list(response: &str, package_id: &str) -> ProviderAdvice {
    let mut lines = response.lines();
    let Some(header) = lines.find(|line| {
        line.contains("Name")
            && line.contains("Id")
            && line.contains("Version")
            && line.contains("Source")
    }) else {
        return ProviderAdvice::unknown("Winget output was localized or ambiguous.");
    };
    let has_available = header.contains("Available");
    let matches = lines
        .filter_map(|line| {
            let columns = line.split_whitespace().collect::<Vec<_>>();
            let id_index = columns.iter().position(|column| *column == package_id)?;
            let current = columns.get(id_index + 1)?.to_string();
            let next = columns.get(id_index + 2).copied();
            let source = columns.last().copied();
            (source == Some("winget")).then_some((current, next.map(str::to_owned)))
        })
        .collect::<Vec<_>>();
    let [(current_version, available)] = matches.as_slice() else {
        return ProviderAdvice::unknown("Winget did not return one exact package match.");
    };
    if normalize_version(current_version).is_none() {
        return ProviderAdvice::unknown("Winget returned an ambiguous installed version.");
    }
    let status = if has_available {
        let Some(latest_version) = available
            .as_deref()
            .filter(|value| *value != "winget")
            .and_then(normalize_version)
        else {
            return ProviderAdvice::unknown("Winget did not provide a reliable available version.");
        };
        UpdateStatus::UpdateAvailable {
            current_version: current_version.clone(),
            latest_version,
        }
    } else {
        UpdateStatus::Current {
            current_version: current_version.clone(),
        }
    };
    ProviderAdvice {
        status,
        information_url: None,
    }
}

pub(crate) fn parse_simhub_page(
    current_version: Option<&str>,
    response: &str,
    information_url: &str,
) -> ProviderAdvice {
    let Some(current_version) = current_version.and_then(normalize_version) else {
        return ProviderAdvice::unknown("The installed SimHub version could not be read safely.");
    };
    let headings = html_headings(response);
    let download_versions = headings
        .iter()
        .filter_map(|heading| {
            heading
                .strip_prefix("Download SimHub v")
                .and_then(|value| normalize_version(value.trim()))
        })
        .collect::<Vec<_>>();
    let [latest_version] = download_versions.as_slice() else {
        return ProviderAdvice::unknown("The official SimHub page changed or was ambiguous.");
    };
    let changelog_matches = headings
        .iter()
        .filter_map(|heading| normalize_version(heading))
        .filter(|version| version == latest_version)
        .count();
    if changelog_matches != 1 {
        return ProviderAdvice::unknown(
            "The official SimHub download and changelog versions did not agree.",
        );
    }
    let Some(ordering) = compare_versions(&current_version, latest_version) else {
        return ProviderAdvice::unknown("The SimHub versions could not be compared.");
    };
    let status = if ordering.is_lt() {
        UpdateStatus::UpdateAvailable {
            current_version,
            latest_version: latest_version.clone(),
        }
    } else {
        UpdateStatus::Current { current_version }
    };
    ProviderAdvice {
        status,
        information_url: Some(information_url.to_owned()),
    }
}

fn normalize_version(value: &str) -> Option<String> {
    let value = value.trim().strip_prefix('v').unwrap_or(value.trim());
    let value = value.trim();
    let segments = value
        .split('.')
        .map(str::parse::<u64>)
        .collect::<Result<Vec<_>, _>>()
        .ok()?;
    (segments.len() >= 2 && segments.iter().any(|segment| *segment != 0)).then(|| value.to_owned())
}

fn compare_versions(current: &str, latest: &str) -> Option<std::cmp::Ordering> {
    let mut current = current
        .split('.')
        .map(str::parse::<u64>)
        .collect::<Result<Vec<_>, _>>()
        .ok()?;
    let mut latest = latest
        .split('.')
        .map(str::parse::<u64>)
        .collect::<Result<Vec<_>, _>>()
        .ok()?;
    let length = current.len().max(latest.len());
    current.resize(length, 0);
    latest.resize(length, 0);
    Some(current.cmp(&latest))
}

fn html_headings(document: &str) -> Vec<String> {
    let lowercase = document.to_ascii_lowercase();
    let mut headings = Vec::new();
    let mut cursor = 0;
    while let Some(relative_start) = lowercase[cursor..].find("<h") {
        let start = cursor + relative_start;
        let Some(level) = lowercase.as_bytes().get(start + 2).copied() else {
            break;
        };
        if !(b'1'..=b'6').contains(&level) {
            cursor = start + 2;
            continue;
        }
        let Some(open_end_relative) = lowercase[start..].find('>') else {
            break;
        };
        let content_start = start + open_end_relative + 1;
        let close = format!("</h{}>", level as char);
        let Some(content_end_relative) = lowercase[content_start..].find(&close) else {
            cursor = content_start;
            continue;
        };
        let content_end = content_start + content_end_relative;
        headings.push(collapse_html_text(&document[content_start..content_end]));
        cursor = content_end + close.len();
    }
    headings
}

fn collapse_html_text(value: &str) -> String {
    let mut text = String::new();
    let mut in_tag = false;
    for character in value.chars() {
        match character {
            '<' => in_tag = true,
            '>' => {
                in_tag = false;
                text.push(' ');
            }
            _ if !in_tag => text.push(character),
            _ => {}
        }
    }
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn is_trusted_information_url(value: &str, allowed_hosts: &[&str]) -> bool {
    tauri::Url::parse(value).is_ok_and(|url| {
        url.scheme() == "https"
            && url.username().is_empty()
            && url.password().is_none()
            && allowed_hosts.iter().any(|host| {
                url.host_str()
                    .is_some_and(|value| value.eq_ignore_ascii_case(host))
            })
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        ApplicationUpdateTarget, CatalogUpdateProvider, UpdateChannel, UpdateCheckPlan,
        UpdateCheckTrigger, UpdateStatus,
    };
    use std::{cell::RefCell, collections::BTreeMap};

    struct FakeRuntime {
        responses: BTreeMap<String, String>,
        requests: RefCell<Vec<String>>,
        versions: BTreeMap<String, String>,
        winget: BTreeMap<String, String>,
    }

    impl UpdateProviderRuntime for FakeRuntime {
        fn get_https(&self, url: &str, _allowed_hosts: &[&str]) -> Result<String, String> {
            self.requests.borrow_mut().push(url.to_owned());
            self.responses
                .get(url)
                .cloned()
                .ok_or_else(|| "missing fake HTTP response".to_owned())
        }

        fn winget_list(&self, package_id: &str) -> Result<String, String> {
            self.requests
                .borrow_mut()
                .push(format!("winget:{package_id}"));
            self.winget
                .get(package_id)
                .cloned()
                .ok_or_else(|| "missing fake Winget response".to_owned())
        }

        fn file_version(&self, executable_path: &str) -> Option<String> {
            self.versions.get(executable_path).cloned()
        }
    }

    #[test]
    fn github_release_contract_compares_only_unambiguous_versions() {
        let available = parse_github_release(
            Some("1.4.0"),
            r#"{"tag_name":"v1.5.0","html_url":"https://github.com/coasting-nc/LMUFFB/releases/tag/v1.5.0"}"#,
        );
        assert_eq!(
            available.status,
            UpdateStatus::UpdateAvailable {
                current_version: "1.4.0".to_owned(),
                latest_version: "1.5.0".to_owned(),
            }
        );
        assert_eq!(
            available.information_url.as_deref(),
            Some("https://github.com/coasting-nc/LMUFFB/releases/tag/v1.5.0")
        );

        assert!(matches!(
            parse_github_release(
                None,
                r#"{"tag_name":"v1.5.0","html_url":"https://github.com/coasting-nc/LMUFFB/releases/tag/v1.5.0"}"#,
            )
            .status,
            UpdateStatus::Unknown { .. }
        ));
    }

    #[test]
    fn winget_contract_uses_exact_installed_and_available_columns() {
        let advice = parse_winget_list(
            "Name             Id                      Version Available Source\n\
             -----------------------------------------------------------------\n\
             Trading Paints   Rhinode.TradingPaints  2.0.36  2.0.37    winget\n",
            "Rhinode.TradingPaints",
        );
        assert_eq!(
            advice.status,
            UpdateStatus::UpdateAvailable {
                current_version: "2.0.36".to_owned(),
                latest_version: "2.0.37".to_owned(),
            }
        );

        assert!(matches!(
            parse_winget_list(
                "Nom Version Disponible Source\nTrading Paints 2.0.36 2.0.37 winget\n",
                "Rhinode.TradingPaints",
            )
            .status,
            UpdateStatus::Unknown { .. }
        ));
    }

    #[test]
    fn simhub_contract_requires_matching_download_and_changelog_versions() {
        let current = parse_simhub_page(
            Some("9.5.0"),
            "<h1>Download SimHub v9.6.1</h1><h2>9.6.1</h2><p>Latest changes</p>",
            "https://www.simhubdash.com/download-2/",
        );
        assert_eq!(
            current.status,
            UpdateStatus::UpdateAvailable {
                current_version: "9.5.0".to_owned(),
                latest_version: "9.6.1".to_owned(),
            }
        );

        assert!(matches!(
            parse_simhub_page(
                Some("9.5.0"),
                "<h1>Download SimHub v9.6.1</h1><h2>9.6.0</h2>",
                "https://www.simhubdash.com/download-2/",
            )
            .status,
            UpdateStatus::Unknown { .. }
        ));
    }

    #[test]
    fn provider_runner_queries_only_curated_origins_and_returns_unknown_fail_closed() {
        let github_url = "https://api.github.com/repos/coasting-nc/LMUFFB/releases/latest";
        let simhub_url = "https://www.simhubdash.com/download-2/";
        let runtime = FakeRuntime {
            responses: BTreeMap::from([
                (
                    github_url.to_owned(),
                    r#"{"tag_name":"v1.5.0","html_url":"https://github.com/coasting-nc/LMUFFB/releases/tag/v1.5.0"}"#.to_owned(),
                ),
                (
                    simhub_url.to_owned(),
                    "<h1>Download SimHub v9.6.1</h1><h2>9.6.1</h2>".to_owned(),
                ),
            ]),
            requests: RefCell::new(Vec::new()),
            versions: BTreeMap::from([
                ("C:\\Apps\\LMUFFB.exe".to_owned(), "1.4.0".to_owned()),
                ("C:\\Apps\\SimHub.exe".to_owned(), "9.5.0".to_owned()),
            ]),
            winget: BTreeMap::from([(
                "Rhinode.TradingPaints".to_owned(),
                "Name Id Version Source\nTrading Paints Rhinode.TradingPaints 2.0.37 winget\n"
                    .to_owned(),
            )]),
        };
        let plan = UpdateCheckPlan {
            request_id: "request".to_owned(),
            channel: UpdateChannel::Stable,
            trigger: UpdateCheckTrigger::Manual,
            applications: vec![
                ApplicationUpdateTarget {
                    application_id: "lmuffb-profile-id".to_owned(),
                    name: "LMUFFB".to_owned(),
                    executable_path: Some("C:\\Apps\\LMUFFB.exe".to_owned()),
                    provider: Some(CatalogUpdateProvider::GitHubReleases {
                        repository: "coasting-nc/LMUFFB".to_owned(),
                    }),
                },
                ApplicationUpdateTarget {
                    application_id: "trading-paints-profile-id".to_owned(),
                    name: "Trading Paints".to_owned(),
                    executable_path: None,
                    provider: Some(CatalogUpdateProvider::Winget {
                        package_id: "Rhinode.TradingPaints".to_owned(),
                    }),
                },
                ApplicationUpdateTarget {
                    application_id: "simhub-profile-id".to_owned(),
                    name: "SimHub".to_owned(),
                    executable_path: Some("C:\\Apps\\SimHub.exe".to_owned()),
                    provider: Some(CatalogUpdateProvider::OfficialPage {
                        url: simhub_url.to_owned(),
                    }),
                },
                ApplicationUpdateTarget {
                    application_id: "custom-profile-id".to_owned(),
                    name: "Custom tool".to_owned(),
                    executable_path: None,
                    provider: None,
                },
            ],
        };

        let advice = UpdateProviderRunner::new(runtime).check(&plan);

        assert_eq!(
            advice
                .iter()
                .map(|application| application.status.clone())
                .collect::<Vec<_>>(),
            vec![
                UpdateStatus::UpdateAvailable {
                    current_version: "1.4.0".to_owned(),
                    latest_version: "1.5.0".to_owned(),
                },
                UpdateStatus::Current {
                    current_version: "2.0.37".to_owned(),
                },
                UpdateStatus::UpdateAvailable {
                    current_version: "9.5.0".to_owned(),
                    latest_version: "9.6.1".to_owned(),
                },
                UpdateStatus::Unknown {
                    reason: "No trusted update provider is configured.".to_owned(),
                },
            ]
        );
        assert_eq!(
            advice
                .iter()
                .map(|application| application.application_id.as_str())
                .collect::<Vec<_>>(),
            vec![
                "lmuffb-profile-id",
                "trading-paints-profile-id",
                "simhub-profile-id",
                "custom-profile-id",
            ]
        );
    }
}
