//! Formation Lap's narrow native host interface.

mod commands;
mod contracts;

pub use commands::get_app_snapshot;
pub use contracts::AppSnapshot;
use tauri::Url;

fn navigation_is_allowed(url: &Url) -> bool {
    match (url.scheme(), url.host_str()) {
        ("tauri", _) => true,
        ("http" | "https", Some("tauri.localhost")) => true,
        ("http", Some("localhost" | "127.0.0.1")) if cfg!(debug_assertions) => true,
        _ => false,
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let navigation_guard = tauri::plugin::Builder::<tauri::Wry>::new("navigation-guard")
        .on_navigation(|_webview, url| navigation_is_allowed(url))
        .build();

    tauri::Builder::default()
        .plugin(navigation_guard)
        .invoke_handler(tauri::generate_handler![commands::get_app_snapshot])
        .run(tauri::generate_context!())
        .expect("Formation Lap failed to start");
}

#[cfg(test)]
mod tests {
    use super::navigation_is_allowed;
    use tauri::Url;

    #[test]
    fn remote_navigation_is_denied() {
        let remote = Url::parse("https://example.com").expect("valid test URL");

        assert!(!navigation_is_allowed(&remote));
    }

    #[test]
    fn bundled_windows_origin_is_allowed() {
        let bundled = Url::parse("http://tauri.localhost").expect("valid test URL");

        assert!(navigation_is_allowed(&bundled));
    }
}
