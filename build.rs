//! Embeds the PovarDock icon and product metadata.

fn main() {
    println!("cargo:rerun-if-changed=assets/povardock.ico");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        let source = "assets/liquid-glass.hlsl";
        println!("cargo:rerun-if-changed={source}");
        println!("cargo:rerun-if-env-changed=GPUI_FXC_PATH");
        let output = std::path::PathBuf::from(std::env::var_os("OUT_DIR").unwrap()).join("liquid-glass.cso");
        let compiler = std::env::var_os("GPUI_FXC_PATH").unwrap_or_else(|| "fxc.exe".into());
        let compiled = std::process::Command::new(compiler)
            .args(["/nologo", "/T", "cs_5_0", "/E", "main", "/O3", "/Fo"])
            .arg(&output)
            .arg(source)
            .output()
            .expect("find fxc.exe on PATH or set GPUI_FXC_PATH");
        assert!(
            compiled.status.success(),
            "Liquid Glass shader: {}",
            String::from_utf8_lossy(&compiled.stderr)
        );

        let mut res = winresource::WindowsResource::new();
        let nightly = std::env::var_os("CARGO_FEATURE_NIGHTLY").is_some();
        // Resource ID 1 is used by the launcher and system tray.
        res.set_icon_with_id("assets/povardock.ico", "1");
        res.set(
            "FileDescription",
            &format!("PovarDock{}", if nightly { " (Nightly)" } else { "" }),
        );
        res.set("ProductName", if nightly { "PovarDock Nightly" } else { "PovarDock" });
        res.set("InternalName", "PovarDock");
        res.set("OriginalFilename", "povardock-x64.exe");
        res.set("LegalCopyright", "Copyright (c) 2026 Sumir Seth. MIT License.");
        for (id, path) in [
            (100, "LICENSE"),
            (101, "assets/LICENSE-GEMOJI"),
            (102, "assets/LICENSE-CLDR"),
            (103, "settings/assets/icons/LICENSE-LUCIDE"),
        ] {
            println!("cargo:rerun-if-changed={path}");
            res.append_rc_content(&format!("{id} RCDATA \"{path}\"\n"));
        }
        res.compile().expect("failed to embed Windows resources");
    }
}
