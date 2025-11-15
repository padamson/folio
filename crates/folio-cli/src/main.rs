use anyhow::{Context, Result};
use chrono::{Duration, Utc};
use clap::{Parser, Subcommand};
use folio_core::{
    generate_filename, group_by_temporal_proximity, scan_directory, validate_batch_name,
    TemporalBatch,
};
use std::collections::HashMap;
use std::fs;
use std::io::{self, Write};
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

        /// Batch name for all files (skips interactive prompts)
        #[arg(long)]
        batch_name: Option<String>,

        /// Time gap in hours to separate batches (default: 2.0)
        #[arg(long, default_value = "2.0")]
        gap_threshold: f64,
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

/// Prompt user for batch name with validation
fn prompt_for_batch_name(
    batch_num: usize,
    total_batches: usize,
    batch: &TemporalBatch,
) -> Result<String> {
    loop {
        // Display batch information
        println!("\n--- Batch {} of {} ---", batch_num, total_batches);
        println!(
            "  Date range: {} to {}",
            batch.start_time.format("%Y-%m-%d %H:%M:%S"),
            batch.end_time.format("%Y-%m-%d %H:%M:%S")
        );

        let photo_count = batch
            .items
            .iter()
            .filter(|i| i.media_type.is_photo())
            .count();
        let video_count = batch
            .items
            .iter()
            .filter(|i| i.media_type.is_video())
            .count();
        println!(
            "  Files: {} ({} photos, {} videos)",
            batch.items.len(),
            photo_count,
            video_count
        );

        // Show first 3 filenames as samples
        println!("  Samples:");
        for (i, item) in batch.items.iter().take(3).enumerate() {
            if let Some(filename) = item.path.file_name() {
                println!("    {}. {}", i + 1, filename.to_string_lossy());
            }
        }
        if batch.items.len() > 3 {
            println!("    ... and {} more", batch.items.len() - 3);
        }

        // Prompt for batch name
        print!("\nEnter batch name: ");
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let batch_name = input.trim().to_string();

        // Validate batch name
        match validate_batch_name(&batch_name) {
            Ok(_) => return Ok(batch_name),
            Err(e) => {
                eprintln!("❌ Invalid batch name: {}", e);
                eprintln!("   Please use only alphanumeric characters, hyphens, and underscores.");
                // Loop to re-prompt
            }
        }
    }
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
            batch_name,
            gap_threshold,
        } => {
            // Validate batch name if provided
            if let Some(ref name) = batch_name {
                validate_batch_name(name).context("Invalid batch name")?;
            }

            let source_path = PathBuf::from(&source);
            let dest_path = PathBuf::from(&dest);

            if dry_run {
                println!("Dry run mode - no files will be copied\n");
            }

            // Scan source directory
            println!("Scanning source: {}", source);
            let source_items =
                scan_directory(&source_path).context("Failed to scan source directory")?;

            if source_items.is_empty() {
                println!("No media files found in source directory");
                return Ok(());
            }

            // Count by type
            let photo_count = source_items
                .iter()
                .filter(|i| i.media_type.is_photo())
                .count();
            let video_count = source_items
                .iter()
                .filter(|i| i.media_type.is_video())
                .count();

            let plural = if source_items.len() == 1 {
                "file"
            } else {
                "files"
            };
            println!(
                "Found {} media {} ({} photos, {} videos)",
                source_items.len(),
                plural,
                photo_count,
                video_count
            );

            // Decide on batching strategy based on --batch-name flag
            let batches_with_names: Vec<(TemporalBatch, String)> = if let Some(ref single_name) =
                batch_name
            {
                // User provided single batch name - treat all files as one batch
                // Temporal batching is disabled when --batch-name is provided
                println!("Using single batch name for all files (temporal batching disabled)");

                // Create single batch with all items
                let all_batch = TemporalBatch {
                    start_time: source_items
                        .iter()
                        .filter_map(|i| i.timestamp)
                        .min()
                        .unwrap_or_else(Utc::now),
                    end_time: source_items
                        .iter()
                        .filter_map(|i| i.timestamp)
                        .max()
                        .unwrap_or_else(Utc::now),
                    items: source_items.clone(),
                };
                vec![(all_batch, single_name.clone())]
            } else {
                // No batch name provided - detect temporal batches for interactive naming
                let gap_threshold_duration = Duration::seconds((gap_threshold * 3600.0) as i64);
                let batches = group_by_temporal_proximity(&source_items, gap_threshold_duration);

                if batches.is_empty() {
                    vec![]
                } else {
                    let batch_plural = if batches.len() == 1 {
                        "batch"
                    } else {
                        "batches"
                    };
                    println!(
                        "Detected {} temporal {} (gap threshold: {:.1} hours)",
                        batches.len(),
                        batch_plural,
                        gap_threshold
                    );

                    if dry_run {
                        // In dry-run mode, skip interactive prompts, use placeholder names
                        batches
                            .into_iter()
                            .enumerate()
                            .map(|(i, batch)| (batch, format!("batch-{}", i + 1)))
                            .collect()
                    } else {
                        // Interactive prompts for batch naming
                        let total_batches = batches.len();
                        let mut batches_with_names = Vec::new();
                        for (i, batch) in batches.into_iter().enumerate() {
                            let name = prompt_for_batch_name(i + 1, total_batches, &batch)?;
                            batches_with_names.push((batch, name));
                        }
                        batches_with_names
                    }
                }
            };

            if !dry_run {
                // Create destination directory if it doesn't exist
                fs::create_dir_all(&dest_path).context("Failed to create destination directory")?;

                // Scan destination to check for duplicates
                let dest_items = scan_directory(&dest_path).unwrap_or_default();
                let dest_hashes: HashMap<_, _> = dest_items
                    .iter()
                    .map(|item| (item.hash, &item.path))
                    .collect();

                // Copy files from each batch
                let mut copied = 0;
                let mut skipped = 0;

                for (batch, batch_name) in &batches_with_names {
                    for item in &batch.items {
                        // Generate destination filename with batch name
                        let timestamp = item.timestamp.unwrap_or_else(|| {
                            // Fallback to modified date if no timestamp
                            std::fs::metadata(&item.path)
                                .and_then(|m| m.modified())
                                .map(|t| t.into())
                                .unwrap_or_else(|_| Utc::now())
                        });
                        let extension = item
                            .path
                            .extension()
                            .and_then(|e| e.to_str())
                            .unwrap_or("jpg");
                        let dest_filename = generate_filename(timestamp, batch_name, extension);

                        // Create date-based folder structure
                        let dest_folder = dest_path.join(&item.folder_path);
                        fs::create_dir_all(&dest_folder)
                            .context("Failed to create date-based folder")?;

                        let dest_file = dest_folder.join(&dest_filename);

                        // Check if already exists in destination
                        if dest_hashes.contains_key(&item.hash) {
                            skipped += 1;
                            continue;
                        }

                        // Copy file
                        fs::copy(&item.path, &dest_file)
                            .context(format!("Failed to copy {:?}", dest_filename))?;
                        copied += 1;
                    }
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
