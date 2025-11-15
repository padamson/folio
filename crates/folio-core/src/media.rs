use anyhow::{Context, Result};
use blake3::Hash as Blake3Hash;
use chrono::{DateTime, Datelike, Duration, Timelike, Utc};
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PhotoFormat {
    Jpeg,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VideoFormat {
    Mov,
    Mp4,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MediaType {
    Photo(PhotoFormat),
    Video(VideoFormat),
}

impl MediaType {
    pub fn is_photo(&self) -> bool {
        matches!(self, MediaType::Photo(_))
    }

    pub fn is_video(&self) -> bool {
        matches!(self, MediaType::Video(_))
    }
}

#[derive(Debug, Clone)]
pub struct MediaItem {
    pub path: PathBuf,
    pub hash: Blake3Hash,
    pub size: u64,
    pub media_type: MediaType,
    pub timestamp: Option<DateTime<Utc>>,
    pub folder_path: PathBuf,
}

/// Represents a temporal batch of media items
/// Items in a batch are grouped by time proximity (e.g., same event)
#[derive(Debug, Clone)]
pub struct TemporalBatch {
    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,
    pub items: Vec<MediaItem>,
}

/// Detect media type from file extension
pub fn detect_media_type(path: &Path) -> Option<MediaType> {
    let ext = path.extension()?.to_str()?.to_lowercase();

    match ext.as_str() {
        "jpg" | "jpeg" => Some(MediaType::Photo(PhotoFormat::Jpeg)),
        "mov" => Some(MediaType::Video(VideoFormat::Mov)),
        "mp4" => Some(MediaType::Video(VideoFormat::Mp4)),
        _ => None,
    }
}

/// Extract capture timestamp from a media file
/// Returns None if no timestamp metadata is available
pub fn get_capture_timestamp(path: &Path, media_type: &MediaType) -> Result<Option<DateTime<Utc>>> {
    match media_type {
        MediaType::Photo(_) => {
            // Try to extract EXIF DateTimeOriginal
            let file =
                std::fs::File::open(path).context("Failed to open file for EXIF extraction")?;
            let mut bufreader = std::io::BufReader::new(file);

            let exifreader = exif::Reader::new();
            let exif = exifreader.read_from_container(&mut bufreader);

            if let Ok(exif) = exif {
                if let Some(field) = exif.get_field(exif::Tag::DateTimeOriginal, exif::In::PRIMARY)
                {
                    if let exif::Value::Ascii(ref vec) = field.value {
                        if let Some(datetime_bytes) = vec.first() {
                            // EXIF format: "YYYY:MM:DD HH:MM:SS"
                            let datetime_str = String::from_utf8_lossy(datetime_bytes);
                            // Parse EXIF datetime format
                            if let Ok(dt) = chrono::NaiveDateTime::parse_from_str(
                                datetime_str.trim(),
                                "%Y:%m:%d %H:%M:%S",
                            ) {
                                return Ok(Some(DateTime::<Utc>::from_naive_utc_and_offset(
                                    dt, Utc,
                                )));
                            }
                        }
                    }
                }
            }
            Ok(None)
        }
        MediaType::Video(_) => {
            // For videos, we'll use file creation/modification date as fallback
            // TODO: Extract video metadata in future enhancement
            Ok(None)
        }
    }
}

/// Get file modification timestamp as fallback
pub fn get_file_modified_date(path: &Path) -> Result<DateTime<Utc>> {
    let metadata = std::fs::metadata(path).context("Failed to read file metadata")?;
    let modified = metadata
        .modified()
        .context("Failed to get file modification time")?;
    Ok(modified.into())
}

/// Generate folder path from timestamp (YYYY/MM/DD)
pub fn generate_folder_path(timestamp: DateTime<Utc>) -> PathBuf {
    PathBuf::from(format!(
        "{:04}/{:02}/{:02}",
        timestamp.year(),
        timestamp.month(),
        timestamp.day()
    ))
}

/// Generate standardized filename from timestamp and batch name
/// Format: YYYYMMDD-HHMMSS-{batch-name}.{ext}
/// Example: 20241104-140215-thanksgiving-arrival.jpg
pub fn generate_filename(
    timestamp: DateTime<Utc>,
    batch_name: &str,
    original_extension: &str,
) -> String {
    format!(
        "{:04}{:02}{:02}-{:02}{:02}{:02}-{}.{}",
        timestamp.year(),
        timestamp.month(),
        timestamp.day(),
        timestamp.hour(),
        timestamp.minute(),
        timestamp.second(),
        batch_name,
        original_extension
    )
}

/// Validate batch name format
/// Batch names must be alphanumeric with hyphens and underscores only
/// No spaces or special characters allowed
///
/// # Arguments
/// * `name` - The batch name to validate
///
/// # Returns
/// Ok(()) if valid, Err with descriptive message if invalid
///
/// # Examples
/// ```
/// use folio_core::validate_batch_name;
///
/// assert!(validate_batch_name("vacation-2024").is_ok());
/// assert!(validate_batch_name("family_reunion").is_ok());
/// assert!(validate_batch_name("trip 1").is_err()); // spaces not allowed
/// ```
pub fn validate_batch_name(name: &str) -> Result<()> {
    // Check if empty
    if name.is_empty() {
        return Err(anyhow::anyhow!("Batch name cannot be empty"));
    }

    // Check if contains only alphanumeric, hyphens, and underscores
    let is_valid = name
        .chars()
        .all(|c| c.is_alphanumeric() || c == '-' || c == '_');

    if !is_valid {
        return Err(anyhow::anyhow!(
            "Batch name must contain only alphanumeric characters, hyphens, and underscores (got: '{}')",
            name
        ));
    }

    // Check if contains at least one alphanumeric character
    // (prevents names like "---" or "___")
    let has_alphanumeric = name.chars().any(|c| c.is_alphanumeric());

    if !has_alphanumeric {
        return Err(anyhow::anyhow!(
            "Batch name must contain at least one alphanumeric character (got: '{}')",
            name
        ));
    }

    Ok(())
}

/// Group media items by temporal proximity
/// Items are grouped into batches based on time gaps between consecutive items
/// A new batch starts when the gap between items exceeds the threshold
///
/// # Arguments
/// * `items` - Media items to group (will be sorted by timestamp)
/// * `gap_threshold` - Maximum time gap within a batch (e.g., Duration::hours(2))
///
/// # Returns
/// Vector of temporal batches, each containing items from the same event
///
/// # Example
/// ```
/// use chrono::{Duration, DateTime, Utc};
/// use folio_core::{group_by_temporal_proximity, MediaItem, MediaType, generate_folder_path};
/// use folio_core::media::PhotoFormat;
/// use std::path::PathBuf;
///
/// // Create test items with 1.5-hour gap (within 2-hour threshold)
/// let timestamp1 = DateTime::parse_from_rfc3339("2024-11-04T14:00:00Z")
///     .unwrap()
///     .with_timezone(&Utc);
/// let timestamp2 = DateTime::parse_from_rfc3339("2024-11-04T15:30:00Z")
///     .unwrap()
///     .with_timezone(&Utc);
///
/// let items = vec![
///     MediaItem {
///         path: PathBuf::from("photo1.jpg"),
///         hash: blake3::hash(b"test1"),
///         size: 1000,
///         media_type: MediaType::Photo(PhotoFormat::Jpeg),
///         timestamp: Some(timestamp1),
///         folder_path: generate_folder_path(timestamp1),
///     },
///     MediaItem {
///         path: PathBuf::from("photo2.jpg"),
///         hash: blake3::hash(b"test2"),
///         size: 2000,
///         media_type: MediaType::Photo(PhotoFormat::Jpeg),
///         timestamp: Some(timestamp2),
///         folder_path: generate_folder_path(timestamp2),
///     },
/// ];
///
/// let gap_threshold = Duration::hours(2);
/// let batches = group_by_temporal_proximity(&items, gap_threshold);
///
/// // Assert: Items within 2-hour gap should be in same batch
/// assert_eq!(batches.len(), 1);
/// assert_eq!(batches[0].items.len(), 2);
/// assert_eq!(batches[0].start_time, timestamp1);
/// assert_eq!(batches[0].end_time, timestamp2);
/// ```
pub fn group_by_temporal_proximity(
    items: &[MediaItem],
    gap_threshold: Duration,
) -> Vec<TemporalBatch> {
    if items.is_empty() {
        return Vec::new();
    }

    // Create a sorted copy of items by timestamp
    // Filter out items without timestamps
    let mut sorted_items: Vec<MediaItem> = items
        .iter()
        .filter(|item| item.timestamp.is_some())
        .cloned()
        .collect();

    // Sort by timestamp
    sorted_items.sort_by(|a, b| a.timestamp.cmp(&b.timestamp));

    if sorted_items.is_empty() {
        return Vec::new();
    }

    let mut batches: Vec<TemporalBatch> = Vec::new();
    let mut current_batch_items: Vec<MediaItem> = Vec::new();
    let mut batch_start_time = sorted_items[0].timestamp.unwrap();

    for item in sorted_items {
        let item_timestamp = item.timestamp.unwrap(); // Safe because we filtered

        if current_batch_items.is_empty() {
            // First item in new batch
            current_batch_items.push(item);
        } else {
            // Check gap from previous item
            let prev_timestamp = current_batch_items.last().unwrap().timestamp.unwrap();
            let gap = item_timestamp - prev_timestamp;

            if gap > gap_threshold {
                // Gap exceeded - finalize current batch and start new one
                let batch_end_time = current_batch_items.last().unwrap().timestamp.unwrap();
                batches.push(TemporalBatch {
                    start_time: batch_start_time,
                    end_time: batch_end_time,
                    items: current_batch_items.clone(),
                });

                // Start new batch
                current_batch_items.clear();
                current_batch_items.push(item);
                batch_start_time = item_timestamp;
            } else {
                // Within threshold - add to current batch
                current_batch_items.push(item);
            }
        }
    }

    // Don't forget the last batch
    if !current_batch_items.is_empty() {
        let batch_end_time = current_batch_items.last().unwrap().timestamp.unwrap();
        batches.push(TemporalBatch {
            start_time: batch_start_time,
            end_time: batch_end_time,
            items: current_batch_items,
        });
    }

    batches
}

/// Calculate BLAKE3 hash of a file
pub fn hash_file(path: &Path) -> Result<Blake3Hash> {
    let mut file = File::open(path).context("Failed to open file for hashing")?;
    let mut hasher = blake3::Hasher::new();
    let mut buffer = [0; 8192];

    loop {
        let count = file
            .read(&mut buffer)
            .context("Failed to read file during hashing")?;
        if count == 0 {
            break;
        }
        hasher.update(&buffer[..count]);
    }

    Ok(hasher.finalize())
}

/// Scan directory recursively and return all media items
pub fn scan_directory(path: &Path) -> Result<Vec<MediaItem>> {
    let mut items = Vec::new();

    for entry in WalkDir::new(path).follow_links(false) {
        let entry = entry.context("Failed to read directory entry")?;

        if !entry.file_type().is_file() {
            continue;
        }

        let file_path = entry.path();

        // Check if it's a media file
        let Some(media_type) = detect_media_type(file_path) else {
            continue;
        };

        // Get file size
        let metadata = entry.metadata().context("Failed to read file metadata")?;
        let size = metadata.len();

        // Calculate hash
        let hash = hash_file(file_path)?;

        // Extract timestamp (with fallback to modified date)
        let timestamp = get_capture_timestamp(file_path, &media_type)?
            .or_else(|| get_file_modified_date(file_path).ok());

        // Generate folder path from timestamp
        let folder_path = if let Some(ts) = timestamp {
            generate_folder_path(ts)
        } else {
            PathBuf::from("unknown-date")
        };

        items.push(MediaItem {
            path: file_path.to_path_buf(),
            hash,
            size,
            media_type,
            timestamp,
            folder_path,
        });
    }

    Ok(items)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_media_type_jpeg() {
        let path = PathBuf::from("test.jpg");
        let media_type = detect_media_type(&path);
        assert_eq!(media_type, Some(MediaType::Photo(PhotoFormat::Jpeg)));

        let path = PathBuf::from("test.JPG");
        let media_type = detect_media_type(&path);
        assert_eq!(media_type, Some(MediaType::Photo(PhotoFormat::Jpeg)));

        let path = PathBuf::from("test.jpeg");
        let media_type = detect_media_type(&path);
        assert_eq!(media_type, Some(MediaType::Photo(PhotoFormat::Jpeg)));
    }

    #[test]
    fn test_detect_media_type_video() {
        let path = PathBuf::from("test.mov");
        let media_type = detect_media_type(&path);
        assert_eq!(media_type, Some(MediaType::Video(VideoFormat::Mov)));

        let path = PathBuf::from("test.MOV");
        let media_type = detect_media_type(&path);
        assert_eq!(media_type, Some(MediaType::Video(VideoFormat::Mov)));

        let path = PathBuf::from("test.mp4");
        let media_type = detect_media_type(&path);
        assert_eq!(media_type, Some(MediaType::Video(VideoFormat::Mp4)));
    }

    #[test]
    fn test_detect_media_type_non_media() {
        let path = PathBuf::from("test.txt");
        let media_type = detect_media_type(&path);
        assert_eq!(media_type, None);

        let path = PathBuf::from("test.pdf");
        let media_type = detect_media_type(&path);
        assert_eq!(media_type, None);
    }

    #[test]
    fn test_scan_directory_with_fixtures() {
        let test_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join("test-data/fixtures");

        let items = scan_directory(&test_dir).unwrap();

        // Should find photos and videos but not text files
        assert!(items.len() >= 7, "Expected at least 7 media files");

        let photos: Vec<_> = items.iter().filter(|i| i.media_type.is_photo()).collect();
        let videos: Vec<_> = items.iter().filter(|i| i.media_type.is_video()).collect();

        assert!(photos.len() >= 4, "Expected at least 4 photos");
        assert!(videos.len() >= 3, "Expected at least 3 videos");

        // Verify all items have valid hashes and sizes
        for item in &items {
            assert!(item.size > 0, "File size should be > 0");
        }
    }

    #[test]
    fn test_hash_file() {
        let test_file = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join("test-data/fixtures/minimal.jpg");

        let hash1 = hash_file(&test_file).unwrap();
        let hash2 = hash_file(&test_file).unwrap();

        // Same file should produce same hash
        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_group_by_temporal_proximity_single_batch() {
        // All items within 2-hour gap should be in same batch
        let timestamp1 = DateTime::parse_from_rfc3339("2024-11-04T14:00:00Z")
            .unwrap()
            .with_timezone(&Utc);
        let timestamp2 = DateTime::parse_from_rfc3339("2024-11-04T15:30:00Z")
            .unwrap()
            .with_timezone(&Utc); // 1.5 hours later

        let items = vec![
            MediaItem {
                path: PathBuf::from("photo1.jpg"),
                hash: blake3::hash(b"test1"),
                size: 1000,
                media_type: MediaType::Photo(PhotoFormat::Jpeg),
                timestamp: Some(timestamp1),
                folder_path: generate_folder_path(timestamp1),
            },
            MediaItem {
                path: PathBuf::from("photo2.jpg"),
                hash: blake3::hash(b"test2"),
                size: 2000,
                media_type: MediaType::Photo(PhotoFormat::Jpeg),
                timestamp: Some(timestamp2),
                folder_path: generate_folder_path(timestamp2),
            },
        ];

        let gap_threshold = Duration::hours(2);
        let batches = group_by_temporal_proximity(&items, gap_threshold);

        assert_eq!(batches.len(), 1, "Should have 1 batch");
        assert_eq!(batches[0].items.len(), 2, "Batch should have 2 items");
        assert_eq!(batches[0].start_time, timestamp1);
        assert_eq!(batches[0].end_time, timestamp2);
    }

    #[test]
    fn test_group_by_temporal_proximity_two_batches() {
        // Items with >2-hour gap should be in separate batches
        let timestamp1 = DateTime::parse_from_rfc3339("2024-11-04T14:00:00Z")
            .unwrap()
            .with_timezone(&Utc);
        let timestamp2 = DateTime::parse_from_rfc3339("2024-11-04T18:00:00Z")
            .unwrap()
            .with_timezone(&Utc); // 4 hours later

        let items = vec![
            MediaItem {
                path: PathBuf::from("photo1.jpg"),
                hash: blake3::hash(b"test1"),
                size: 1000,
                media_type: MediaType::Photo(PhotoFormat::Jpeg),
                timestamp: Some(timestamp1),
                folder_path: generate_folder_path(timestamp1),
            },
            MediaItem {
                path: PathBuf::from("photo2.jpg"),
                hash: blake3::hash(b"test2"),
                size: 2000,
                media_type: MediaType::Photo(PhotoFormat::Jpeg),
                timestamp: Some(timestamp2),
                folder_path: generate_folder_path(timestamp2),
            },
        ];

        let gap_threshold = Duration::hours(2);
        let batches = group_by_temporal_proximity(&items, gap_threshold);

        assert_eq!(batches.len(), 2, "Should have 2 batches");
        assert_eq!(batches[0].items.len(), 1, "First batch should have 1 item");
        assert_eq!(batches[1].items.len(), 1, "Second batch should have 1 item");
    }

    #[test]
    fn test_group_by_temporal_proximity_three_batches() {
        // Test with 3 distinct temporal batches
        let timestamp1 = DateTime::parse_from_rfc3339("2024-11-04T09:00:00Z")
            .unwrap()
            .with_timezone(&Utc);
        let timestamp2 = DateTime::parse_from_rfc3339("2024-11-04T09:30:00Z")
            .unwrap()
            .with_timezone(&Utc); // 30 min later (same batch)
        let timestamp3 = DateTime::parse_from_rfc3339("2024-11-04T14:00:00Z")
            .unwrap()
            .with_timezone(&Utc); // 4.5 hours later (new batch)
        let timestamp4 = DateTime::parse_from_rfc3339("2024-11-04T20:00:00Z")
            .unwrap()
            .with_timezone(&Utc); // 6 hours later (new batch)

        let items = vec![
            MediaItem {
                path: PathBuf::from("photo1.jpg"),
                hash: blake3::hash(b"test1"),
                size: 1000,
                media_type: MediaType::Photo(PhotoFormat::Jpeg),
                timestamp: Some(timestamp1),
                folder_path: generate_folder_path(timestamp1),
            },
            MediaItem {
                path: PathBuf::from("photo2.jpg"),
                hash: blake3::hash(b"test2"),
                size: 2000,
                media_type: MediaType::Photo(PhotoFormat::Jpeg),
                timestamp: Some(timestamp2),
                folder_path: generate_folder_path(timestamp2),
            },
            MediaItem {
                path: PathBuf::from("photo3.jpg"),
                hash: blake3::hash(b"test3"),
                size: 3000,
                media_type: MediaType::Photo(PhotoFormat::Jpeg),
                timestamp: Some(timestamp3),
                folder_path: generate_folder_path(timestamp3),
            },
            MediaItem {
                path: PathBuf::from("photo4.jpg"),
                hash: blake3::hash(b"test4"),
                size: 4000,
                media_type: MediaType::Photo(PhotoFormat::Jpeg),
                timestamp: Some(timestamp4),
                folder_path: generate_folder_path(timestamp4),
            },
        ];

        let gap_threshold = Duration::hours(2);
        let batches = group_by_temporal_proximity(&items, gap_threshold);

        assert_eq!(batches.len(), 3, "Should have 3 batches");
        assert_eq!(batches[0].items.len(), 2, "First batch should have 2 items");
        assert_eq!(batches[1].items.len(), 1, "Second batch should have 1 item");
        assert_eq!(batches[2].items.len(), 1, "Third batch should have 1 item");
    }

    #[test]
    fn test_group_by_temporal_proximity_empty() {
        let items: Vec<MediaItem> = vec![];
        let gap_threshold = Duration::hours(2);
        let batches = group_by_temporal_proximity(&items, gap_threshold);

        assert_eq!(batches.len(), 0, "Empty input should produce no batches");
    }

    #[test]
    fn test_validate_batch_name_valid_alphanumeric() {
        assert!(validate_batch_name("vacation").is_ok());
        assert!(validate_batch_name("trip2024").is_ok());
        assert!(validate_batch_name("FAMILY").is_ok());
        assert!(validate_batch_name("Event123").is_ok());
    }

    #[test]
    fn test_validate_batch_name_valid_with_hyphens() {
        assert!(validate_batch_name("vacation-2024").is_ok());
        assert!(validate_batch_name("family-reunion").is_ok());
        assert!(validate_batch_name("summer-trip-day1").is_ok());
    }

    #[test]
    fn test_validate_batch_name_valid_with_underscores() {
        assert!(validate_batch_name("family_reunion").is_ok());
        assert!(validate_batch_name("trip_2024").is_ok());
        assert!(validate_batch_name("vacation_day_1").is_ok());
    }

    #[test]
    fn test_validate_batch_name_valid_mixed() {
        assert!(validate_batch_name("vacation_2024-day1").is_ok());
        assert!(validate_batch_name("Trip-2024_summer").is_ok());
    }

    #[test]
    fn test_validate_batch_name_invalid_spaces() {
        assert!(validate_batch_name("vacation 2024").is_err());
        assert!(validate_batch_name("family reunion").is_err());
        assert!(validate_batch_name("trip day 1").is_err());
    }

    #[test]
    fn test_validate_batch_name_invalid_special_chars() {
        assert!(validate_batch_name("vacation!").is_err());
        assert!(validate_batch_name("trip@home").is_err());
        assert!(validate_batch_name("family#reunion").is_err());
        assert!(validate_batch_name("event$2024").is_err());
        assert!(validate_batch_name("trip%off").is_err());
        assert!(validate_batch_name("day&night").is_err());
        assert!(validate_batch_name("trip*").is_err());
        assert!(validate_batch_name("event(2024)").is_err());
    }

    #[test]
    fn test_validate_batch_name_invalid_empty() {
        assert!(validate_batch_name("").is_err());
    }

    #[test]
    fn test_validate_batch_name_invalid_only_hyphens() {
        assert!(validate_batch_name("---").is_err());
    }

    #[test]
    fn test_validate_batch_name_invalid_only_underscores() {
        assert!(validate_batch_name("___").is_err());
    }
}
