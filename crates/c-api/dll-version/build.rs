fn main() {
    let target = std::env::var("TARGET").unwrap();
    if target.contains("windows") {
        let rc_path = "dll-version/version.rc";
        let include_paths = [
            "C:\\Program Files (x86)\\Windows Kits\\10\\Include\\10.0.26100.0\\um",
            "C:\\Program Files (x86)\\Windows Kits\\10\\Include\\10.0.26100.0\\shared",
            "C:\\Program Files (x86)\\Windows Kits\\10\\Include\\10.0.26100.0\\winrt",
            "C:\\Program Files (x86)\\Windows Kits\\10\\Include\\10.0.26100.0\\ucrt",
        ];
        let include_combined = include_paths.join(";");

        let out_dir = std::env::var("OUT_DIR").unwrap();
        let output = std::process::Command::new("rc.exe")
            .env("INCLUDE", include_combined)
            .args(["/fo", &format!("{}/version.res", out_dir), rc_path])
            .status()
            .expect("Failed to run rc.exe");
        assert!(output.success(), "rc.exe failed");

        // println!("cargo:rustc-link-arg=/DEF:{}", "your.def"); // optional
        println!("cargo:rustc-link-arg={}/version.res", out_dir);
    }
}
