# Integration Tests

A test suite that executes generated bindings to ensure soundness.

## Creating A New Test

Each integration test is a directory that contains (at least) two files:

- `native.rs`
- `test.rs`

From there, we create two crates in a temporary directory. The `native.rs` file
is used to build a `cdylib` which we'll generate bindings for, and `test.rs`
will be used to compile an executable that tests those generated bindings.
