fn main() {
    tauri_build::build();
    let target_env = std::env::var("CARGO_CFG_TARGET_ENV").unwrap_or_default();
    if target_env == "gnu" {
        if let Ok(out_dir) = std::env::var("OUT_DIR") {
            let lib_res = std::path::Path::new(&out_dir).join("libresource.a");
            if lib_res.exists() {
                println!("cargo:rustc-link-arg=-Wl,--whole-archive");
                println!("cargo:rustc-link-arg={}", lib_res.display());
                println!("cargo:rustc-link-arg=-Wl,--no-whole-archive");
            }
        }
    } else if target_env == "msvc" {
        if let Ok(out_dir) = std::env::var("OUT_DIR") {
            let manifest_path =
                std::path::Path::new(&out_dir).join("common-controls-v6.manifest");
            let manifest_xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<assembly xmlns="urn:schemas-microsoft-com:asm.v1" manifestVersion="1.0">
  <dependency>
    <dependentAssembly>
      <assemblyIdentity
        type="win32"
        name="Microsoft.Windows.Common-Controls"
        version="6.0.0.0"
        processorArchitecture="*"
        publicKeyToken="6595b64144ccf1df"
        language="*"
      />
    </dependentAssembly>
  </dependency>
</assembly>
"#;
            if std::fs::write(&manifest_path, manifest_xml).is_ok() {
                println!("cargo:rustc-link-arg=/MANIFEST:EMBED");
                println!(
                    "cargo:rustc-link-arg=/MANIFESTINPUT:{}",
                    manifest_path.display()
                );
                println!("cargo:rustc-link-arg-bins=/MANIFEST:NO");
            }
        }
    }
}
