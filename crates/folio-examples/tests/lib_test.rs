//! Integration tests for folio-examples library

use assert_fs::prelude::*;
use camino::Utf8Path;
use folio_examples::{validate_structure, EXAMPLE_DATA_DIR};

#[test]
fn test_validate_structure_valid() {
    let temp = assert_fs::TempDir::new().unwrap();

    // Create valid structure
    temp.child("sd-card-thanksgiving/DCIM/100NIKON")
        .create_dir_all()
        .unwrap();
    temp.child("archive").create_dir_all().unwrap();
    temp.child("archive-with-existing")
        .create_dir_all()
        .unwrap();

    let path = Utf8Path::from_path(temp.path()).unwrap();
    let result = validate_structure(path);

    assert!(result.is_ok());
}

#[test]
fn test_validate_structure_missing_sd_card() {
    let temp = assert_fs::TempDir::new().unwrap();

    // Missing sd-card-thanksgiving
    temp.child("archive").create_dir_all().unwrap();
    temp.child("archive-with-existing")
        .create_dir_all()
        .unwrap();

    let path = Utf8Path::from_path(temp.path()).unwrap();
    let result = validate_structure(path);

    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("sd-card-thanksgiving"));
}

#[test]
fn test_validate_structure_missing_dcim() {
    let temp = assert_fs::TempDir::new().unwrap();

    // Missing DCIM directory
    temp.child("sd-card-thanksgiving").create_dir_all().unwrap();
    temp.child("archive").create_dir_all().unwrap();
    temp.child("archive-with-existing")
        .create_dir_all()
        .unwrap();

    let path = Utf8Path::from_path(temp.path()).unwrap();
    let result = validate_structure(path);

    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("DCIM"));
}

#[test]
fn test_validate_structure_missing_archive() {
    let temp = assert_fs::TempDir::new().unwrap();

    // Missing archive
    temp.child("sd-card-thanksgiving/DCIM/100NIKON")
        .create_dir_all()
        .unwrap();
    temp.child("archive-with-existing")
        .create_dir_all()
        .unwrap();

    let path = Utf8Path::from_path(temp.path()).unwrap();
    let result = validate_structure(path);

    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("archive"));
}

#[test]
fn test_example_data_dir_constant() {
    assert_eq!(EXAMPLE_DATA_DIR, "example-data");
}

#[test]
fn test_from_folio_version() {
    let config = folio_examples::from_folio_version().unwrap();

    // Should read the workspace version (which this crate inherits, so the
    // assertion survives version bumps)
    assert_eq!(config.version, env!("CARGO_PKG_VERSION"));

    // Should construct download URL
    assert!(config.download_url.contains(env!("CARGO_PKG_VERSION")));
    assert!(config.download_url.contains("folio-example-data"));
}

#[test]
fn test_example_data_path() {
    let path = folio_examples::example_data_path().unwrap();

    // Should end with "example-data"
    assert!(path.as_str().ends_with("example-data"));

    // Should be absolute path
    assert!(path.is_absolute());
}
