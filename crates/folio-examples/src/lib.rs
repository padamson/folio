//! Example data management for Folio
//!
//! Provides utilities for generating and downloading AI-generated example media data
//! for testing and demonstration purposes.

use anyhow::{anyhow, Context, Result};
use camino::{Utf8Path, Utf8PathBuf};
use flate2::read::GzDecoder;
use std::env;
use std::fs::File;
use std::io::{self, Write};
use tar::Archive;

/// Directory name for example data at project root
pub const EXAMPLE_DATA_DIR: &str = "example-data";

/// Configuration for example data downloads
#[derive(Debug, Clone)]
pub struct ExampleDataConfig {
    /// Folio version (e.g., "0.1.0")
    pub version: String,
    /// Download URL for example data tarball
    pub download_url: String,
}

/// Returns path to example data directory (workspace root / example-data)
///
/// Finds the workspace root by walking up from the current directory looking for
/// a Cargo.toml with `[workspace]`, then returns the path to the example-data
/// directory within that workspace.
///
/// # Example
///
/// ```
/// use folio_examples::example_data_path;
///
/// let path = example_data_path().unwrap();
/// assert!(path.as_str().ends_with("example-data"));
/// assert!(path.is_absolute());
/// ```
pub fn example_data_path() -> Result<Utf8PathBuf> {
    let workspace_root = find_workspace_root()?;
    Ok(workspace_root.join(EXAMPLE_DATA_DIR))
}

/// Validates example data directory structure
///
/// Ensures the required directories exist:
/// - `sd-card-thanksgiving/DCIM/100NIKON`
/// - `archive`
/// - `archive-with-existing`
///
/// # Example
///
/// ```no_run
/// use camino::Utf8Path;
/// use folio_examples::validate_structure;
///
/// let path = Utf8Path::new("example-data");
/// validate_structure(path).unwrap();
/// ```
pub fn validate_structure(root: &Utf8Path) -> Result<()> {
    let required_dirs = [
        "sd-card-thanksgiving/DCIM/100NIKON",
        "archive",
        "archive-with-existing",
    ];

    for dir in &required_dirs {
        let path = root.join(dir);
        if !path.exists() {
            return Err(anyhow!("Required directory missing: {}", path));
        }
        if !path.is_dir() {
            return Err(anyhow!("Path exists but is not a directory: {}", path));
        }
    }

    Ok(())
}

/// Creates ExampleDataConfig from workspace Cargo.toml version
///
/// Reads the workspace version from the Cargo.toml file at the project root
/// and constructs a download URL for the matching example data release.
///
/// # Example
///
/// ```
/// use folio_examples::from_folio_version;
///
/// let config = from_folio_version().unwrap();
/// // The crate inherits the workspace version, so the two always agree.
/// assert_eq!(config.version, env!("CARGO_PKG_VERSION"));
/// assert!(config.download_url.contains(env!("CARGO_PKG_VERSION")));
/// ```
pub fn from_folio_version() -> Result<ExampleDataConfig> {
    // Find workspace root by looking for Cargo.toml
    let workspace_root = find_workspace_root()?;
    let cargo_toml_path = workspace_root.join("Cargo.toml");

    // Read and parse Cargo.toml
    let cargo_toml_content = std::fs::read_to_string(&cargo_toml_path)
        .with_context(|| format!("Failed to read {}", cargo_toml_path))?;

    let cargo_toml: toml::Value =
        toml::from_str(&cargo_toml_content).context("Failed to parse Cargo.toml")?;

    // Extract version from [workspace.package]
    let version = cargo_toml
        .get("workspace")
        .and_then(|w| w.get("package"))
        .and_then(|p| p.get("version"))
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow!("Could not find workspace.package.version in Cargo.toml"))?
        .to_string();

    // Construct download URL
    let download_url = format!(
        "https://github.com/padamson/folio-example-data/releases/download/v{}/example-data.tar.gz",
        version
    );

    Ok(ExampleDataConfig {
        version,
        download_url,
    })
}

/// Downloads example data from the specified URL
///
/// Downloads the tarball to a temporary file and returns the path.
pub fn download_tarball(url: &str) -> Result<tempfile::NamedTempFile> {
    print!("Downloading example data from {}... ", url);
    io::stdout().flush()?;

    let response =
        reqwest::blocking::get(url).with_context(|| format!("Failed to download from {}", url))?;

    if !response.status().is_success() {
        return Err(anyhow!(
            "Download failed with status: {}",
            response.status()
        ));
    }

    let mut temp_file =
        tempfile::NamedTempFile::new().context("Failed to create temporary file")?;

    let bytes = response.bytes().context("Failed to read response bytes")?;
    temp_file
        .write_all(&bytes)
        .context("Failed to write to temporary file")?;

    println!("done");

    Ok(temp_file)
}

/// Extracts a tarball to the specified directory
pub fn extract_tarball(tarball_path: &std::path::Path, dest: &Utf8Path) -> Result<()> {
    print!("Extracting to {}... ", dest);
    io::stdout().flush()?;

    let file = File::open(tarball_path)
        .with_context(|| format!("Failed to open tarball: {:?}", tarball_path))?;

    let decoder = GzDecoder::new(file);
    let mut archive = Archive::new(decoder);

    // Extract to parent directory of dest, since tarball contains "example-data/" prefix
    let parent = dest
        .parent()
        .ok_or_else(|| anyhow!("Destination has no parent directory"))?;

    archive
        .unpack(parent.as_std_path())
        .context("Failed to extract tarball")?;

    println!("done");

    Ok(())
}

/// Finds the workspace root by walking up from current directory
fn find_workspace_root() -> Result<Utf8PathBuf> {
    let current = env::current_dir().context("Failed to get current directory")?;
    let mut current = Utf8PathBuf::try_from(current)
        .map_err(|e| anyhow!("Current directory is not valid UTF-8: {}", e))?;

    loop {
        let cargo_toml = current.join("Cargo.toml");
        if cargo_toml.exists() {
            // Check if this is a workspace root
            let content = std::fs::read_to_string(&cargo_toml)?;
            if content.contains("[workspace]") {
                return Ok(current);
            }
        }

        // Move to parent directory
        if !current.pop() {
            return Err(anyhow!(
                "Could not find workspace root (no Cargo.toml with [workspace])"
            ));
        }
    }
}
