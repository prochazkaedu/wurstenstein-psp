use std::{env, path::PathBuf};

fn main() {
	// Tell Cargo that if the given file changes, to rerun this build script.
	println!("cargo::rerun-if-changed=lib/modplay.c");
	// Use the `cc` crate to build a C file and statically link it.
	cc::Build::new()
		.file("lib/modplay.c")
		// .compiler("mipsel-none-elf-gcc")
		// .no_default_flags(true)
		// .pic(false)
		// .flag("-D__STDC_HOSTED__=0")
		// .flag("-march=mips32r2")
		// .flag("-mel")
		// .flag("-mno-abicalls")
		// .flag("-mabi=eabi")
		// .flag("-Os")
		// .compiler("psp-gcc")
		// .no_default_flags(true)        // skip cc-rs defaults like -fPIC, -ffunction-sections, etc.
		// .pic(false)                    // explicitly disable PIC
		// .flag("-D__STDC_HOSTED__=0")
		// .flag("-march=mips32r2")
		// .flag("-mel")
		// .flag("-mabi=32")
		// .flag("-mno-abicalls")
		.flag("-Os")
		.compile("modplay");

	let out_dir = std::env::var("OUT_DIR").unwrap();
	// The object file is usually named like <hash>-modplay.o inside the archive
	// or directly as modplay.o in OUT_DIR depending on cc-rs version.
	// Strip local/debug symbols that may confuse rust-lld.
	let _ = std::process::Command::new("psp-objcopy")
		.arg("--strip-unneeded")
		.arg(format!("{}/libmodplay.a", out_dir))
		.status();

	let bindings = bindgen::Builder::default()
		.use_core()
		// The input header we would like to generate
		// bindings for.
		.header("lib/modplay.h")
		// Tell cargo to invalidate the built crate whenever any of the
		// included header files changed.
		.parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
		// Finish the builder and generate the bindings.
		.generate()
		// Unwrap the Result and panic on failure.
		.expect("Unable to generate bindings");

	// Write the bindings to the $OUT_DIR/bindings.rs file.
	let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
	bindings
		.write_to_file(out_path.join("bindings.rs"))
		.expect("Couldn't write bindings!");
}
