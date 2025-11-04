use anyhow::{Context, Result};
use blake3::Hash as Blake3Hash;
use chrono::{DateTime, Datelike, Utc};
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
}
