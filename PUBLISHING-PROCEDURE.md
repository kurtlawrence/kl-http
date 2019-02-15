# Publishing Procedure

1. `cargo my-readme` - updates the readmes on the `lib.rs` and `main.rs`
2. `cargo update` - updates the compatible versions
3. `cargo outdated` - check outdated versions, update to latest major if possible
4. `cargo test` (use the no threading version)
5. `cargo bench` - see if there are any regressions
6. Increment version number