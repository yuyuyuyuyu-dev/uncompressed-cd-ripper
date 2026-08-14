use super::*;
// The documented alternative writes the bindings while the app runs in
// debug mode, which would leave CI unable to regenerate them without
// starting a window. Writing them from a test case means `cargo test`
// regenerates them, and the job asserting the working tree stayed clean
// is what turns a stale file into a failure.
#[test]
fn should_generate_typescript_bindings_matching_the_commands() {
    builder()
        .export(
            specta_typescript::Typescript::default(),
            "../src/bindings.ts",
        )
        .expect("the bindings are written next to the frontend sources");
}
