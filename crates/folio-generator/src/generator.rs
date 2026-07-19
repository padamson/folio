//! The [`Generator`] and its photo-shaped configuration.

use anyhow::{Context, Result};
use camino::Utf8Path;
use chrono::{DateTime, Utc};
use image::{GenericImageView, ImageReader};

use crate::metadata::stamp_photo_metadata;
use crate::source::ImageSource;

/// Camera identity stamped into generated media's metadata.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CameraProfile {
    /// EXIF `Make`, e.g. `"NIKON CORPORATION"`.
    pub make: String,
    /// EXIF `Model`, e.g. `"NIKON D800"`.
    pub model: String,
}

impl CameraProfile {
    /// Build a camera profile from a make and model.
    pub fn new(make: impl Into<String>, model: impl Into<String>) -> Self {
        Self {
            make: make.into(),
            model: model.into(),
        }
    }
}

/// Target photo dimensions plus the camera identity to stamp.
#[derive(Debug, Clone)]
pub struct PhotoSpec {
    /// Output width in pixels.
    pub width: u32,
    /// Output height in pixels.
    pub height: u32,
    /// Camera identity written into EXIF.
    pub camera: CameraProfile,
    /// Optional text appended to every prompt (e.g. a realism modifier like
    /// `"photorealistic, DSLR quality"`). `None` sends the prompt unchanged.
    pub prompt_suffix: Option<String>,
}

impl PhotoSpec {
    /// Apply the configured prompt suffix, if any.
    pub(crate) fn decorate_prompt(&self, prompt: &str) -> String {
        match &self.prompt_suffix {
            Some(suffix) if !suffix.is_empty() => format!("{}, {}", prompt, suffix),
            _ => prompt.to_string(),
        }
    }
}

/// Generates photos by pairing an [`ImageSource`] with a [`PhotoSpec`].
pub struct Generator {
    source: Box<dyn ImageSource>,
    spec: PhotoSpec,
}

impl Generator {
    /// Create a generator from an image source and a photo spec.
    pub fn new(source: Box<dyn ImageSource>, spec: PhotoSpec) -> Self {
        Self { source, spec }
    }

    /// The configured photo spec.
    pub fn spec(&self) -> &PhotoSpec {
        &self.spec
    }

    /// Generate one photo: request an image for `prompt`, resize it to the
    /// spec's dimensions, save it as JPEG at `output`, and stamp EXIF capture
    /// metadata (including `timestamp`).
    ///
    /// Fails if the source errors, the file can't be written, or `exiftool` is
    /// unavailable for stamping.
    pub async fn generate_photo(
        &self,
        prompt: &str,
        output: &Utf8Path,
        timestamp: DateTime<Utc>,
    ) -> Result<()> {
        let decorated = self.spec.decorate_prompt(prompt);
        let image = self
            .source
            .generate(&decorated)
            .await
            .with_context(|| format!("image source failed for prompt {:?}", decorated))?;

        let resized = image.resize_exact(
            self.spec.width,
            self.spec.height,
            image::imageops::FilterType::Lanczos3,
        );

        if let Some(parent) = output.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("failed to create {}", parent))?;
        }
        resized
            .save(output)
            .with_context(|| format!("failed to write {}", output))?;

        stamp_photo_metadata(output, &self.spec.camera, timestamp)?;
        Ok(())
    }
}

/// Whether `path` is already a valid JPEG of exactly `width` x `height`.
///
/// Used to make generation runs resumable — an already-generated photo can be
/// skipped rather than regenerated (which costs an API call or a GPU minute).
///
/// # Example
///
/// ```
/// use camino::Utf8Path;
/// use folio_generator::is_valid_image;
///
/// assert!(!is_valid_image(Utf8Path::new("/nonexistent.jpg"), 800, 600));
/// ```
pub fn is_valid_image(path: &Utf8Path, width: u32, height: u32) -> bool {
    if !path.exists() {
        return false;
    }
    let reader = match ImageReader::open(path) {
        Ok(r) => r,
        Err(_) => return false,
    };
    if reader.format() != Some(image::ImageFormat::Jpeg) {
        return false;
    }
    match reader.decode() {
        Ok(img) => img.dimensions() == (width, height),
        Err(_) => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decorate_prompt_appends_suffix_when_present() {
        let spec = PhotoSpec {
            width: 10,
            height: 10,
            camera: CameraProfile::new("m", "d"),
            prompt_suffix: Some("photorealistic".to_string()),
        };
        assert_eq!(spec.decorate_prompt("a cat"), "a cat, photorealistic");
    }

    #[test]
    fn decorate_prompt_leaves_prompt_unchanged_without_suffix() {
        let mut spec = PhotoSpec {
            width: 10,
            height: 10,
            camera: CameraProfile::new("m", "d"),
            prompt_suffix: None,
        };
        assert_eq!(spec.decorate_prompt("a cat"), "a cat");
        // An empty suffix is also a no-op, not a trailing ", ".
        spec.prompt_suffix = Some(String::new());
        assert_eq!(spec.decorate_prompt("a cat"), "a cat");
    }

    #[test]
    fn camera_profile_new_accepts_str_and_string() {
        let a = CameraProfile::new("NIKON", "D800");
        let b = CameraProfile::new("NIKON".to_string(), "D800".to_string());
        assert_eq!(a, b);
        assert_eq!(a.make, "NIKON");
        assert_eq!(a.model, "D800");
    }

    #[test]
    fn is_valid_image_rejects_missing_file() {
        assert!(!is_valid_image(
            Utf8Path::new("/no/such/file.jpg"),
            800,
            600
        ));
    }

    #[test]
    fn is_valid_image_checks_dimensions() {
        let dir = tempfile::tempdir().unwrap();
        let path = camino::Utf8PathBuf::from_path_buf(dir.path().join("p.jpg")).unwrap();
        image::RgbImage::from_pixel(40, 30, image::Rgb([1, 2, 3]))
            .save(&path)
            .unwrap();
        assert!(is_valid_image(&path, 40, 30));
        assert!(!is_valid_image(&path, 800, 600));
    }
}
