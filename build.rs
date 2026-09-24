use std::env;
use std::fs;
use std::path::PathBuf;

fn main() {
    println!("cargo:rerun-if-changed=assets/WebView2Loader.dll");

    // Copy WebView2Loader.dll to the target directory and dist alongside the executable
    if let Ok(profile) = env::var("PROFILE") {
        if let Ok(manifest) = env::var("CARGO_MANIFEST_DIR") {
            let manifest_dir = PathBuf::from(manifest);
            let src_dll = manifest_dir.join("assets").join("WebView2Loader.dll");

            if src_dll.exists() {
                // Copy to target/<profile>/
                let target_dir = manifest_dir.join("target").join(&profile);
                let _ = fs::create_dir_all(&target_dir);
                let _ = fs::copy(&src_dll, target_dir.join("WebView2Loader.dll"));

                // Copy to dist/
                let dist_dir = manifest_dir.join("dist");
                let _ = fs::create_dir_all(&dist_dir);
                let _ = fs::copy(&src_dll, dist_dir.join("WebView2Loader.dll"));
            }
        }
    }
}
