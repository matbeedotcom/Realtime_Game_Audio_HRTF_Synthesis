fn main() {
    // Copy the GUI manifest to the output directory so Windows auto-discovers it.
    let manifest = std::path::Path::new("hrtf-synth-gui.exe.manifest");
    if manifest.exists() {
        if let Ok(out_dir) = std::env::var("OUT_DIR") {
            let mut dir = std::path::PathBuf::from(&out_dir);
            while dir.pop() {
                if dir.join("hrtf-synth-gui.exe").exists() {
                    let dest = dir.join("hrtf-synth-gui.exe.manifest");
                    let _ = std::fs::copy(manifest, &dest);
                    break;
                }
            }
        }
    }

    // Tell the compiler where to find the pre-built APO DLL for embedding.
    // Build order: cargo build --release -p hrtf-apo && cargo build --release -p hrtf-synth --features gui
    let workspace_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    let dll_release = workspace_root.join("target").join("release").join("hrtf_apo.dll");
    let dll_debug = workspace_root.join("target").join("debug").join("hrtf_apo.dll");

    let dll_path = if dll_release.exists() {
        dll_release.clone()
    } else if dll_debug.exists() {
        dll_debug.clone()
    } else {
        // Create a dummy so compilation doesn't fail before the DLL is built
        let out_dir = std::env::var("OUT_DIR").unwrap();
        let dummy = std::path::PathBuf::from(&out_dir).join("hrtf_apo_dummy.dll");
        std::fs::write(&dummy, b"").unwrap();
        dummy
    };

    println!("cargo:rustc-env=HRTF_APO_DLL_PATH={}", dll_path.display());
    println!("cargo:rerun-if-changed={}", dll_release.display());
    println!("cargo:rerun-if-changed={}", dll_debug.display());

    // Embed the model file path
    let model_path = workspace_root.join("models").join("hrtf_model.bin");
    println!("cargo:rustc-env=HRTF_MODEL_PATH={}", model_path.display());
    println!("cargo:rerun-if-changed={}", model_path.display());
}
