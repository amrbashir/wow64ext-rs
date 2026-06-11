use std::path::PathBuf;

fn remove_dllmain(source: &str) -> String {
	let mut result = String::new();
	let mut in_dllmain = false;
	let mut brace_count = 0;
	let mut dllmain_start_pattern = false;

	for line in source.lines() {
		let trimmed = line.trim();

		if !in_dllmain {
			if trimmed.starts_with("BOOL WINAPI DllMain(") {
				in_dllmain = true;
				brace_count = 0;
				dllmain_start_pattern = true;
				continue;
			}
			result.push_str(line);
			result.push('\n');
		} else {
			for c in trimmed.chars() {
				if c == '{' {
					brace_count += 1;
					if dllmain_start_pattern && brace_count == 1 {
						dllmain_start_pattern = false;
					}
				} else if c == '}' {
					brace_count -= 1;
					if brace_count == 0 {
						in_dllmain = false;
						break;
					}
				}
			}
		}
	}

	result
}

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

	let wow64ext_src = manifest_dir.join("rewolf-wow64ext/src");
	println!("cargo:rerun-if-changed={}", wow64ext_src.display());

	let wow64ext_cpp = wow64ext_src.join("wow64ext.cpp");
	let wow64ext_cpp = std::fs::read_to_string(&wow64ext_cpp).expect("Failed to read wow64ext.cpp");

	let wow64ext_cpp_patched = out_dir.join("wow64ext.cpp");
	std::fs::write(&wow64ext_cpp_patched, remove_dllmain(&wow64ext_cpp))
		.expect("Failed to write patched wow64ext.cpp");

	let wrapper_src = manifest_dir.join("wrapper.cpp");
	std::fs::copy(&wrapper_src, out_dir.join("wrapper.cpp")).expect("Failed to copy wrapper.cpp");
	println!("cargo:rerun-if-changed={}", wrapper_src.display());

	cc::Build::new()
		.cpp(true)
		.file(out_dir.join("wrapper.cpp"))
		.include(&out_dir)
		.include(&manifest_dir.join("rewolf-wow64ext/src"))
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
		.allowlist_function("Wow64ExtInitialize")
		.merge_extern_blocks(true)
		.generate()
		.expect("Unable to generate bindings")
		.write_to_file(out_dir.join("bindings.rs"))
		.expect("Couldn't write bindings!");
}
