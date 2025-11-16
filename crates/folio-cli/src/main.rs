use anyhow::{Context, Result};
use chrono::{Duration, Utc};
use clap::{Parser, Subcommand};
use folio_core::preview::{
    generate_thumbnail, start_preview_with_browser, BatchPreview, PreviewState, WorkflowStage,
};
use folio_core::{
    generate_filename, group_by_temporal_proximity, scan_directory, validate_batch_name,
    TemporalBatch,
};
use std::collections::HashMap;
use std::fs;
use std::io::{self, Write};
use std::path::PathBuf;

/// Create PreviewState from batches with names for browser preview
///
/// Converts temporal batches with their assigned names into a PreviewState
/// structure suitable for the browser preview dashboard. Generates thumbnails
/// for the first and last photo in each batch, formats time ranges, and
/// filters out videos (only photos are shown in preview).
///
/// # Arguments
///
/// * `batches` - Slice of tuples containing (TemporalBatch, batch_name)
///
/// # Returns
///
/// A `PreviewState` with:
/// - Batch count and list of `BatchPreview` items
/// - Thumbnails for first/last photos (200px max size)
/// - Time ranges formatted as "HH:MM - HH:MM"
/// - Current workflow stage set to `WorkflowStage::Group`
///
/// # Error Handling
///
/// Thumbnail generation failures are handled gracefully - if a thumbnail
/// cannot be generated, it's set to `None` rather than failing the entire
/// preview generation.
fn create_preview_state_from_batches(batches: &[(TemporalBatch, String)]) -> PreviewState {
    let batch_count = batches.len();
    let mut preview_batches = Vec::new();

    for (index, (batch, name)) in batches.iter().enumerate() {
        let file_count = batch.items.len();

        // Get first and last photo paths (only photos, not videos)
        let photos: Vec<_> = batch
            .items
            .iter()
            .filter(|item| item.media_type.is_photo())
            .collect();

        let first_photo = photos
            .first()
            .and_then(|item| item.path.to_str().map(|s| s.to_string()));

        let last_photo = if photos.len() > 1 {
            photos
                .last()
                .and_then(|item| item.path.to_str().map(|s| s.to_string()))
        } else {
            None // Don't show duplicate if only one photo
        };

        // Generate thumbnails (gracefully handle errors)
        let first_thumbnail = first_photo
            .as_ref()
            .and_then(|path| generate_thumbnail(std::path::Path::new(path), 200).ok());

        let last_thumbnail = last_photo
            .as_ref()
            .and_then(|path| generate_thumbnail(std::path::Path::new(path), 200).ok());

        // Format time range (HH:MM - HH:MM)
        let time_range = Some(format!(
            "{} - {}",
            batch.start_time.format("%H:%M"),
            batch.end_time.format("%H:%M")
        ));

        preview_batches.push(BatchPreview {
            index,
            name: Some(name.clone()),
            file_count,
            first_photo,
            last_photo,
            first_thumbnail,
            last_thumbnail,
            time_range,
        });
    }

    PreviewState {
        batch_count,
        current_stage: WorkflowStage::Group,
        batches: preview_batches,
    }
}

/// Create placeholder batch names for initial preview state
///
/// Generates placeholder names like "batch-1", "batch-2", etc. before user provides actual names.
/// This allows the preview server to start before interactive naming, showing the batch structure
/// immediately while the user enters names one by one.
///
/// # Arguments
///
/// * `batches` - Slice of `TemporalBatch` to create placeholders for
///
/// # Returns
///
/// A vector of tuples containing (cloned TemporalBatch, placeholder name)
///
/// # Example
///
/// ```
/// use folio_core::TemporalBatch;
/// use chrono::Utc;
///
/// let batch1 = TemporalBatch {
///     start_time: Utc::now(),
///     end_time: Utc::now(),
///     items: vec![],
/// };
/// let batch2 = batch1.clone();
///
/// let batches = vec![batch1, batch2];
/// let result = create_placeholder_batches(&batches);
///
/// assert_eq!(result.len(), 2);
/// assert_eq!(result[0].1, "batch-1");
/// assert_eq!(result[1].1, "batch-2");
/// ```
fn create_placeholder_batches(batches: &[TemporalBatch]) -> Vec<(TemporalBatch, String)> {
    batches
        .iter()
        .enumerate()
        .map(|(i, batch)| (batch.clone(), format!("batch-{}", i + 1)))
        .collect()
}

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

        /// Skip browser preview (terminal only)
        #[arg(long)]
        headless: bool,
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

#[tokio::main]
async fn main() -> Result<()> {
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
            headless,
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
                        create_placeholder_batches(&batches)
                    } else {
                        // Start preview server BEFORE interactive loop with placeholder names
                        let _preview_server = if !headless && !batches.is_empty() {
                            let placeholder_batches = create_placeholder_batches(&batches);
                            let preview_state =
                                create_preview_state_from_batches(&placeholder_batches);
                            match start_preview_with_browser(preview_state).await {
                                Ok(server) => {
                                    println!("\nBrowser preview: {}", server.url());
                                    Some(server)
                                }
                                Err(e) => {
                                    eprintln!("\nWarning: Failed to start browser preview: {}", e);
                                    eprintln!("Continuing in headless mode...\n");
                                    None
                                }
                            }
                        } else {
                            None
                        };

                        // Interactive loop with real-time updates
                        let total_batches = batches.len();
                        let mut batches_with_names = Vec::new();
                        for (i, batch) in batches.into_iter().enumerate() {
                            let name = prompt_for_batch_name(i + 1, total_batches, &batch)?;

                            // Update browser preview in real-time
                            if let Some(ref server) = _preview_server {
                                if let Err(e) = server.update_batch_name(i, name.clone()).await {
                                    eprintln!("Warning: Failed to update browser preview: {}", e);
                                }
                            }

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

#[cfg(test)]
mod tests {
    use super::*;
    use blake3::Hash as Blake3Hash;
    use chrono::NaiveDateTime;
    use folio_core::media::{MediaItem, MediaType, PhotoFormat, VideoFormat};

    fn dummy_hash() -> Blake3Hash {
        blake3::hash(b"test")
    }

    #[test]
    fn test_create_preview_state_empty() {
        let batches: Vec<(TemporalBatch, String)> = vec![];
        let state = create_preview_state_from_batches(&batches);

        assert_eq!(state.batch_count, 0);
        assert_eq!(state.batches.len(), 0);
        assert_eq!(state.current_stage, WorkflowStage::Group);
    }

    #[test]
    fn test_create_preview_state_single_batch() {
        let start_time = NaiveDateTime::parse_from_str("2024-01-15 14:00:00", "%Y-%m-%d %H:%M:%S")
            .unwrap()
            .and_utc();
        let end_time = NaiveDateTime::parse_from_str("2024-01-15 16:30:00", "%Y-%m-%d %H:%M:%S")
            .unwrap()
            .and_utc();

        let items = vec![MediaItem {
            path: PathBuf::from("test-data/fixtures/sample-with-exif.jpg"),
            media_type: MediaType::Photo(PhotoFormat::Jpeg),
            timestamp: Some(start_time),
            hash: dummy_hash(),
            size: 821,
            folder_path: PathBuf::from("2024/01/15"),
        }];

        let batch = TemporalBatch {
            start_time,
            end_time,
            items,
        };

        let batches = vec![(batch, "test-batch".to_string())];
        let state = create_preview_state_from_batches(&batches);

        assert_eq!(state.batch_count, 1);
        assert_eq!(state.batches.len(), 1);
        assert_eq!(state.batches[0].index, 0);
        assert_eq!(state.batches[0].name, Some("test-batch".to_string()));
        assert_eq!(state.batches[0].file_count, 1);
        assert_eq!(
            state.batches[0].time_range,
            Some("14:00 - 16:30".to_string())
        );
    }

    #[test]
    fn test_create_preview_state_multiple_batches() {
        let start_time1 = NaiveDateTime::parse_from_str("2024-01-15 14:00:00", "%Y-%m-%d %H:%M:%S")
            .unwrap()
            .and_utc();
        let end_time1 = NaiveDateTime::parse_from_str("2024-01-15 16:30:00", "%Y-%m-%d %H:%M:%S")
            .unwrap()
            .and_utc();

        let start_time2 = NaiveDateTime::parse_from_str("2024-01-15 19:00:00", "%Y-%m-%d %H:%M:%S")
            .unwrap()
            .and_utc();
        let end_time2 = NaiveDateTime::parse_from_str("2024-01-15 21:00:00", "%Y-%m-%d %H:%M:%S")
            .unwrap()
            .and_utc();

        let batch1 = TemporalBatch {
            start_time: start_time1,
            end_time: end_time1,
            items: vec![MediaItem {
                path: PathBuf::from("test-data/fixtures/sample-with-exif.jpg"),
                media_type: MediaType::Photo(PhotoFormat::Jpeg),
                timestamp: Some(start_time1),
                hash: dummy_hash(),
                size: 821,
                folder_path: PathBuf::from("2024/01/15"),
            }],
        };

        let batch2 = TemporalBatch {
            start_time: start_time2,
            end_time: end_time2,
            items: vec![MediaItem {
                path: PathBuf::from("test-data/fixtures/sample-different-time.jpg"),
                media_type: MediaType::Photo(PhotoFormat::Jpeg),
                timestamp: Some(start_time2),
                hash: blake3::hash(b"different"),
                size: 625,
                folder_path: PathBuf::from("2024/01/15"),
            }],
        };

        let batches = vec![
            (batch1, "morning".to_string()),
            (batch2, "evening".to_string()),
        ];
        let state = create_preview_state_from_batches(&batches);

        assert_eq!(state.batch_count, 2);
        assert_eq!(state.batches.len(), 2);

        assert_eq!(state.batches[0].index, 0);
        assert_eq!(state.batches[0].name, Some("morning".to_string()));
        assert_eq!(
            state.batches[0].time_range,
            Some("14:00 - 16:30".to_string())
        );

        assert_eq!(state.batches[1].index, 1);
        assert_eq!(state.batches[1].name, Some("evening".to_string()));
        assert_eq!(
            state.batches[1].time_range,
            Some("19:00 - 21:00".to_string())
        );
    }

    #[test]
    fn test_create_preview_state_filters_photos_only() {
        let start_time = NaiveDateTime::parse_from_str("2024-01-15 14:00:00", "%Y-%m-%d %H:%M:%S")
            .unwrap()
            .and_utc();

        let items = vec![
            MediaItem {
                path: PathBuf::from("test-data/fixtures/sample-with-exif.jpg"),
                media_type: MediaType::Photo(PhotoFormat::Jpeg),
                timestamp: Some(start_time),
                hash: dummy_hash(),
                size: 821,
                folder_path: PathBuf::from("2024/01/15"),
            },
            MediaItem {
                path: PathBuf::from("test-data/fixtures/minimal.mp4"),
                media_type: MediaType::Video(VideoFormat::Mp4),
                timestamp: Some(start_time),
                hash: blake3::hash(b"video"),
                size: 12439,
                folder_path: PathBuf::from("2024/01/15"),
            },
        ];

        let batch = TemporalBatch {
            start_time,
            end_time: start_time,
            items,
        };

        let batches = vec![(batch, "mixed".to_string())];
        let state = create_preview_state_from_batches(&batches);

        assert_eq!(state.batch_count, 1);
        assert_eq!(state.batches[0].file_count, 2);
        // Should only have photo path, not video
        assert!(state.batches[0].first_photo.is_some());
        assert!(state.batches[0]
            .first_photo
            .as_ref()
            .unwrap()
            .contains("sample-with-exif.jpg"));
    }

    #[test]
    fn test_create_placeholder_batches() {
        // Create test batches
        let start_time1 = NaiveDateTime::parse_from_str("2024-01-15 14:00:00", "%Y-%m-%d %H:%M:%S")
            .unwrap()
            .and_utc();
        let end_time1 = NaiveDateTime::parse_from_str("2024-01-15 16:30:00", "%Y-%m-%d %H:%M:%S")
            .unwrap()
            .and_utc();

        let start_time2 = NaiveDateTime::parse_from_str("2024-01-15 19:00:00", "%Y-%m-%d %H:%M:%S")
            .unwrap()
            .and_utc();
        let end_time2 = NaiveDateTime::parse_from_str("2024-01-15 21:00:00", "%Y-%m-%d %H:%M:%S")
            .unwrap()
            .and_utc();

        let batch1 = TemporalBatch {
            start_time: start_time1,
            end_time: end_time1,
            items: vec![MediaItem {
                path: PathBuf::from("test-data/fixtures/sample-with-exif.jpg"),
                media_type: MediaType::Photo(PhotoFormat::Jpeg),
                timestamp: Some(start_time1),
                hash: dummy_hash(),
                size: 821,
                folder_path: PathBuf::from("2024/01/15"),
            }],
        };

        let batch2 = TemporalBatch {
            start_time: start_time2,
            end_time: end_time2,
            items: vec![MediaItem {
                path: PathBuf::from("test-data/fixtures/sample-different-time.jpg"),
                media_type: MediaType::Photo(PhotoFormat::Jpeg),
                timestamp: Some(start_time2),
                hash: blake3::hash(b"different"),
                size: 625,
                folder_path: PathBuf::from("2024/01/15"),
            }],
        };

        let batches = vec![batch1, batch2];
        let result = create_placeholder_batches(&batches);

        // Should return 2 batches with placeholder names
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].1, "batch-1");
        assert_eq!(result[1].1, "batch-2");

        // Batches should be cloned (same timestamps, items)
        assert_eq!(result[0].0.start_time, start_time1);
        assert_eq!(result[0].0.end_time, end_time1);
        assert_eq!(result[0].0.items.len(), 1);

        assert_eq!(result[1].0.start_time, start_time2);
        assert_eq!(result[1].0.end_time, end_time2);
        assert_eq!(result[1].0.items.len(), 1);
    }

    #[test]
    fn test_create_placeholder_batches_empty() {
        let batches: Vec<TemporalBatch> = vec![];
        let result = create_placeholder_batches(&batches);

        assert_eq!(result.len(), 0);
    }

    #[test]
    fn test_create_placeholder_batches_single() {
        let start_time = NaiveDateTime::parse_from_str("2024-01-15 14:00:00", "%Y-%m-%d %H:%M:%S")
            .unwrap()
            .and_utc();

        let batch = TemporalBatch {
            start_time,
            end_time: start_time,
            items: vec![MediaItem {
                path: PathBuf::from("test-data/fixtures/sample-with-exif.jpg"),
                media_type: MediaType::Photo(PhotoFormat::Jpeg),
                timestamp: Some(start_time),
                hash: dummy_hash(),
                size: 821,
                folder_path: PathBuf::from("2024/01/15"),
            }],
        };

        let batches = vec![batch];
        let result = create_placeholder_batches(&batches);

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].1, "batch-1");
    }
}
