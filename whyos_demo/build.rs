use std::fs::File;
use std::io::Write;
use std::path::PathBuf;
use std::env;

fn main() {
    let out_dir = PathBuf::from(env::var_os("OUT_DIR").unwrap());
    println!("cargo:rustc-link-search={}", out_dir.display()); // tell linker to look for scripts in OUT_DIR

    // copy memory.x file into OUT_DIR
    let memory_x = include_bytes!("memory.x"); // bake memory.x bytes into build binary
    let mut f = File::create(out_dir.join("memory.x")).unwrap();
    f.write_all(memory_x).unwrap();

    // if build.rs or memory.x is change, run build script again
    println!("cargo:rerun-if-changed=memory.x");
    println!("cargo:rerun-if-changed=build.rs");
}
