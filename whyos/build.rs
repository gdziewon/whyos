use std::env;
use std::fs;
use std::path::PathBuf;

fn main() {
    let target_arch = env::var("CARGO_CFG_TARGET_ARCH").unwrap_or_default();
    let out_dir = PathBuf::from(env::var_os("OUT_DIR").unwrap());

    println!("cargo:rustc-link-search={}", out_dir.display());

    let soc = if env::var("CARGO_FEATURE_RP235X").is_ok() {
        "rp235x"
    } else {
        "rp235x"// we support only rp235x for now
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

    // try to get git describe info for build identification
    let git_ident = std::process::Command::new("git")
        .args(["describe", "--tags", "--dirty", "--always"])
        .output()
        .ok()
        .and_then(|out| if out.status.success() {
            String::from_utf8(out.stdout).ok()
        } else { None })
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|| "unknown".into());


    println!("cargo:rustc-env=WHYOS_BUILD_IDENT={}-{}-{}-{}", env!("CARGO_PKG_NAME"), soc, target_arch, git_ident);
    // Re-run build.rs when git HEAD changes
    println!("cargo:rerun-if-changed=.git/HEAD");
    println!("cargo:rerun-if-changed=.git/index");
}