use std::{env, path::PathBuf};

fn main() {
	// Tell Cargo that if the given file changes, to rerun this build script.
	println!("cargo::rerun-if-changed=lib/modplay.c");
	// Use the `cc` crate to build a C file and statically link it.
	cc::Build::new()
		.file("lib/modplay.c")
		.flag("-Os")
		.compile("modplay");

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
