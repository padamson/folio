use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use folio_core::scan_directory;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "folio")]
#[command(about = "Family media archive workflow tools", long_about = None)]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Ingest photos/videos from a source directory
    Ingest {
        /// Source directory (e.g., SD card, phone backup)
        #[arg(short, long)]
        source: String,

        /// Destination directory in archive
        #[arg(short, long)]
        dest: String,

        /// Perform dry run without copying files
        #[arg(long)]
        dry_run: bool,
    },

    /// Find and report duplicate files
    Dedupe {
        /// Archive directory to scan
        #[arg(short, long)]
        archive: String,

        /// Perform dry run without removing files
        #[arg(long)]
        dry_run: bool,
    },

    /// Show version information
    Version,
}

fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    let cli = Cli::parse();

    match cli.command {
        Commands::Ingest {
            source,
            dest,
            dry_run,
        } => {
            let source_path = PathBuf::from(&source);
            let dest_path = PathBuf::from(&dest);

            if dry_run {
                println!("Dry run mode - no files will be copied\n");
            }

            // Scan source directory
            println!("Scanning source: {}", source);
            let source_items = scan_directory(&source_path)
                .context("Failed to scan source directory")?;

            if source_items.is_empty() {
                println!("No media files found in source directory");
                return Ok(());
            }

            // Count by type
            let photo_count = source_items.iter()
                .filter(|i| i.media_type.is_photo())
                .count();
            let video_count = source_items.iter()
                .filter(|i| i.media_type.is_video())
                .count();

            let plural = if source_items.len() == 1 { "file" } else { "files" };
            println!(
                "Found {} media {} ({} photos, {} videos)",
                source_items.len(),
                plural,
                photo_count,
                video_count
            );

            if !dry_run {
                // Create destination directory if it doesn't exist
                fs::create_dir_all(&dest_path)
                    .context("Failed to create destination directory")?;

                // Scan destination to check for duplicates
                let dest_items = scan_directory(&dest_path).unwrap_or_default();
                let dest_hashes: HashMap<_, _> = dest_items
                    .iter()
                    .map(|item| (item.hash, &item.path))
                    .collect();

                // Copy files
                let mut copied = 0;
                let mut skipped = 0;

                for item in &source_items {
                    let filename = item.path.file_name()
                        .context("Failed to get filename")?;
                    let dest_file = dest_path.join(filename);

                    // Check if already exists in destination
                    if dest_hashes.contains_key(&item.hash) {
                        skipped += 1;
                        continue;
                    }

                    // Copy file
                    fs::copy(&item.path, &dest_file)
                        .context(format!("Failed to copy {:?}", filename))?;
                    copied += 1;
                }

                println!("\nCopied {} files", copied);
                if skipped > 0 {
                    println!("Skipped {} duplicate files", skipped);
                }
            }

            Ok(())
        }
        Commands::Dedupe { archive, dry_run } => {
            println!("Finding duplicates in {}", archive);
            if dry_run {
                println!("(Dry run - no files will be removed)");
            }
            // TODO: Implement deduplication logic
            Ok(())
        }
        Commands::Version => {
            println!("folio {}", env!("CARGO_PKG_VERSION"));
            Ok(())
        }
    }
}
