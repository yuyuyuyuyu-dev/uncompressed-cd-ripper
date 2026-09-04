use std::path::Path;

fn main() {
    let bindings = Path::new(env!("CARGO_MANIFEST_DIR")).join("../src/bindings.ts");

    uncompressed_cd_ripper_lib::builder()
        .export(specta_typescript::Typescript::default(), bindings)
        .expect("the bindings are written next to the frontend sources");
}
