use std::env;
use std::path::PathBuf;

fn main() {
    let target = std::env::var("TARGET").unwrap();
    if target != "i686-pc-windows-msvc" {
        panic!(
            "wow64ext-rs native compilation is only supported for i686-pc-windows-msvc target.\n\
            The wow64ext library contains x86 inline assembly that requires 32-bit compilation.\n\
            \n\
            To build this crate:\
            1. Install MSVC x86 build tools: `rustup target add i686-pc-windows-msvc`\n\
            2. Build with: `cargo build --target i686-pc-windows-msvc`\n\
            \n\
            Current target: {target}\n\
            Required target: i686-pc-windows-msvc",
        );
    }

    let out_dir = std::env::var("OUT_DIR").unwrap();
    let out_dir = PathBuf::from(&out_dir);

    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let manifest_dir = PathBuf::from(&manifest_dir);

    let wow64_ext_dir = manifest_dir.join("rewolf-wow64ext").join("src");

    cc::Build::new()
        .cpp(true)
        .file(wow64_ext_dir.join("wow64ext.cpp"))
        .include(wow64_ext_dir.clone())
        .compile("wow64ext");

    let wrapper_h = manifest_dir.join("wrapper.h");
    println!("cargo:rerun-if-changed={}", wrapper_h.display());

    bindgen::Builder::default()
        .header(wrapper_h.to_string_lossy())
        .allowlist_item("CONTEXT64_.*")
        .allowlist_item("CONTEXT_AMD64")
        .allowlist_function("X64Call")
        .allowlist_function("GetModuleHandle64")
        .allowlist_function("GetProcAddress64")
        .allowlist_function("VirtualQueryEx64")
        .allowlist_function("VirtualAllocEx64")
        .allowlist_function("VirtualFreeEx64")
        .allowlist_function("VirtualProtectEx64")
        .allowlist_function("ReadProcessMemory64")
        .allowlist_function("WriteProcessMemory64")
        .allowlist_function("GetThreadContext64")
        .allowlist_function("SetThreadContext64")
        .allowlist_function("SetLastErrorFromX64Call")
        .merge_extern_blocks(true)
        .generate()
        .expect("Unable to generate bindings")
        .write_to_file(out_dir.join("bindings.rs"))
        .expect("Couldn't write bindings!");
}
