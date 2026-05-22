use std::env;
use std::fs;
use std::path::PathBuf;

fn main() {
    let target_arch = env::var("CARGO_CFG_TARGET_ARCH").unwrap_or_default();
    let out_dir = PathBuf::from(env::var_os("OUT_DIR").unwrap());

    println!("cargo:rustc-link-search={}", out_dir.display());

    let enabled_socs = [("rp235x", env::var_os("CARGO_FEATURE_RP235X").is_some())]
        .into_iter()
        .filter_map(|(soc, enabled)| enabled.then_some(soc))
        .collect::<Vec<_>>();

    let soc = match enabled_socs.as_slice() {
        [soc] => *soc,
        [] => panic!(
            "WhyOS: enable exactly one SoC feature. Currently supported: `rp235x`.\n\
             Example: `cargo build --features rp235x`"
        ),
        _ => panic!(
            "WhyOS: enable exactly one SoC feature, but multiple were set: {}.\n\
             Pick one of: `rp235x`",
            enabled_socs.join(", ")
        ),
    };

    let linker_script_name = match target_arch.as_str() {
        "arm" => "memory-arm.x",
        "riscv32" => "memory-riscv32.x",
        _ => panic!("WhyOS: Unsupported architecture {}", target_arch),
    };

    let memory_x_path = PathBuf::from("src")
        .join("arch")
        .join("soc")
        .join(soc)
        .join(linker_script_name);

    fs::copy(&memory_x_path, out_dir.join("memory.x")).unwrap_or_else(|_| {
        panic!("WhyOS: Can't find linker script at {}", memory_x_path.display())
    });

    println!("cargo:rerun-if-changed={}", memory_x_path.display());
    println!("cargo:rerun-if-changed=build.rs");

    println!("cargo:rustc-env=WHYOS_BUILD_IDENT={}-{}-{}-{}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"), soc, target_arch);
    // Re-run build.rs when git HEAD changes
    println!("cargo:rerun-if-changed=.git/HEAD");
    println!("cargo:rerun-if-changed=.git/index");
}