fn main() {
    tauri_build::build();
    if std::env::var("CARGO_CFG_TARGET_ENV").as_deref() == Ok("gnu") {
        if let Ok(out_dir) = std::env::var("OUT_DIR") {
            let lib_res = std::path::Path::new(&out_dir).join("libresource.a");
            if lib_res.exists() {
                println!("cargo:rustc-link-arg=-Wl,--whole-archive");
                println!("cargo:rustc-link-arg={}", lib_res.display());
                println!("cargo:rustc-link-arg=-Wl,--no-whole-archive");
            }
        }
    }
}
