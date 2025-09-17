// Integration tests module
// These tests are only run when specific features are enabled
// to avoid burdening regular `cargo test` runs

#[cfg(feature = "s3-tests")]
mod s3_tests;
