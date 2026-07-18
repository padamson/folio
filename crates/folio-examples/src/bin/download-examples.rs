//! Download pre-generated example data from GitHub releases

use anyhow::{Context, Result};
use clap::Parser;
use folio_examples::{
    download_tarball, example_data_path, extract_tarball, from_folio_version, validate_structure,
};

#[derive(Parser, Debug)]
#[command(name = "download-examples")]
#[command(about = "Download pre-generated example data from GitHub releases")]
struct Args {
    /// Force re-download even if example data already exists
    #[arg(short, long)]
    force: bool,
}

fn main() -> Result<()> {
    let args = Args::parse();

    // Get example data path
    let example_data_path = example_data_path()?;

    // Check if already exists
    if example_data_path.exists() && !args.force {
        println!("Example data already exists at: {}", example_data_path);
        println!("Use --force to re-download");
        return Ok(());
    }

    // Get version and download URL from workspace Cargo.toml
    let config =
        from_folio_version().context("Failed to read Folio version from workspace Cargo.toml")?;

    println!(
        "Downloading example data for Folio version {}",
        config.version
    );

    // Download tarball
    let tarball = download_tarball(&config.download_url)?;

    // Extract to example-data directory
    extract_tarball(tarball.path(), &example_data_path)?;

    // Validate structure
    print!("Validating structure... ");
    validate_structure(&example_data_path).context("Example data structure validation failed")?;
    println!("done");

    println!(
        "\nExample data successfully downloaded to: {}",
        example_data_path
    );
    println!("\nYou can now run ingestion workflows:");
    println!("  cargo run --bin folio -- ingest \\");
    println!("      --source example-data/sd-card-thanksgiving/DCIM \\");
    println!("      --archive example-data/archive");

    Ok(())
}
