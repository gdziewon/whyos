use std::env;
use std::fs;
use std::path::PathBuf;

fn main() {
    let target_arch = env::var("CARGO_CFG_TARGET_ARCH").unwrap_or_default();
    let out_dir = PathBuf::from(env::var_os("OUT_DIR").unwrap());

    println!("cargo:rustc-link-search={}", out_dir.display());

    let soc = "rp235x"; // we support only rp235x for now

    let memory_x_path =
        PathBuf::from("src")
        .join("arch")
        .join(soc)
        .join(target_arch)
        .join("memory.x");

    fs::copy(&memory_x_path, out_dir.join("memory.x")).unwrap_or_else(|_| {
        panic!("WhyOS: Can't find linker script at {}", memory_x_path.display())
    });

    println!("cargo:rerun-if-changed={}", memory_x_path.display());
    println!("cargo:rerun-if-changed=build.rs");
}