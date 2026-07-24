fn main() {
    println!("cargo:rerun-if-env-changed=FORMATION_LAP_UPDATE_PUBLIC_KEY");
    let windows = tauri_build::WindowsAttributes::new()
        .app_manifest(include_str!("windows-app-manifest.xml"));
    let attributes = tauri_build::Attributes::new().windows_attributes(windows);
    tauri_build::try_build(attributes).expect("failed to run the Tauri build script");
}
