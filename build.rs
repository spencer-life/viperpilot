use std::env;
use std::path::PathBuf;

// 2026-09-30: embed Common Controls v6 only in the opt-in native UI preview
// so standard push buttons can send supported custom-draw notifications.
fn main() {
    const MANIFEST: &str = "assets/viperpilot-ui-preview.manifest";
    println!("cargo:rerun-if-changed={MANIFEST}");

    let is_windows = env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows");
    let preview_enabled = env::var_os("CARGO_FEATURE_UI_PREVIEW").is_some();
    if !is_windows || !preview_enabled {
        return;
    }

    let manifest = PathBuf::from(
        env::var_os("CARGO_MANIFEST_DIR").expect("Cargo sets the manifest directory"),
    )
    .join(MANIFEST)
    .canonicalize()
    .expect("the native UI preview manifest must exist");
    println!("cargo:rustc-link-arg-bin=viperpilot-ui-preview=/MANIFEST:EMBED");
    println!(
        "cargo:rustc-link-arg-bin=viperpilot-ui-preview=/MANIFESTINPUT:{}",
        manifest.display()
    );
}
