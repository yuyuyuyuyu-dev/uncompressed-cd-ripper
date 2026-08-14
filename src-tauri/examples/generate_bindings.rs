use std::path::Path;

// Writes the TypeScript the frontend imports. Anything that reads it runs this
// first, which pnpm arranges through the pre scripts in package.json.
//
// An example rather than a test case, because it asserts nothing. It exists to
// produce a file, and examples are Cargo's facility for running a small
// program out of a crate.
fn main() {
    // Resolved against the crate rather than the working directory. Cargo is
    // invoked from src-tauri, because that is where the toolchain file rustup
    // reads lives, and the file belongs beside the frontend sources.
    let bindings = Path::new(env!("CARGO_MANIFEST_DIR")).join("../src/bindings.ts");

    uncompressed_cd_ripper_lib::builder()
        .export(specta_typescript::Typescript::default(), bindings)
        .expect("the bindings are written next to the frontend sources");
}
