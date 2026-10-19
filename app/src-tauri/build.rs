fn main() {
    // The Windows shell reads this icon directly from the executable.
    // Register it as a build dependency so an icon replacement always
    // rebuilds the executable resource instead of leaving a stale icon.
    println!("cargo:rerun-if-changed=icons/icon.ico");
    println!("cargo:rerun-if-changed=icons/icon-dark.png");
    println!("cargo:rerun-if-changed=icons/icon-light.png");

    let attributes = tauri_build::Attributes::new().windows_attributes(
        tauri_build::WindowsAttributes::new().window_icon_path("icons/icon.ico"),
    );
    tauri_build::try_build(attributes).expect("failed to build Windows application resources");
}
