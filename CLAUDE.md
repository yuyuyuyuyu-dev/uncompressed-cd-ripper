## Coding Conventions

### Tests as Specification

State every specification explicitly as a test case.
Each test case's name MUST start with "should", and MUST state exactly one specification.

### Feature package

Split directories by feature (`src/features/<feature>/`, `src-tauri/src/<feature>/`).
Also declare `mod <feature>;` in `lib.rs`.

`src/components/ui/` is shared, not a feature: the shadcn/ui CLI owns that path and
rewrites the files in it whenever a component is added or updated.
