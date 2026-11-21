fn main() {
    println!("cargo:rerun-if-changed=../assets/povardock.ico");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        let mut res = winresource::WindowsResource::new();
        let nightly = std::env::var_os("CARGO_FEATURE_NIGHTLY").is_some();
        res.set_icon_with_id("../assets/povardock.ico", "1");
        res.set(
            "FileDescription",
            &format!("PovarDock Settings{}", if nightly { " (Nightly)" } else { "" }),
        );
        res.set("ProductName", if nightly { "PovarDock Nightly" } else { "PovarDock" });
        res.set("LegalCopyright", "Copyright (c) 2026 Sumir Seth. MIT License.");
        res.compile().expect("failed to embed Windows resources");
    }
}
