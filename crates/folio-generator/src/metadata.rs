//! Metadata stamping via `exiftool`.
//!
//! Photos get EXIF capture metadata; videos get the QuickTime equivalent. Both
//! also get their filesystem modification time set to the capture time via
//! `-FileModifyDate`, so downstream tools that fall back to mtime (as folio's
//! ingestion does for video, which carries no EXIF) still see the right time.

use anyhow::{bail, Context, Result};
use camino::Utf8Path;
use chrono::{DateTime, Utc};
use std::process::Command;

use crate::generator::CameraProfile;

/// Whether `exiftool` is on the PATH.
pub fn exiftool_available() -> bool {
    Command::new("exiftool")
        .arg("-ver")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// exiftool datetime format: `YYYY:MM:DD HH:MM:SS`.
fn exif_datetime(timestamp: DateTime<Utc>) -> String {
    timestamp.format("%Y:%m:%d %H:%M:%S").to_string()
}

/// Run exiftool with the given tag arguments against `path`, failing loudly.
fn run_exiftool(path: &Utf8Path, args: &[String]) -> Result<()> {
    if !exiftool_available() {
        bail!(
            "exiftool is required to stamp metadata but was not found on PATH \
             (install it, e.g. `brew install exiftool`)"
        );
    }
    let output = Command::new("exiftool")
        .arg("-overwrite_original")
        .args(args)
        .arg(path.as_str())
        .output()
        .context("failed to run exiftool")?;
    if !output.status.success() {
        bail!(
            "exiftool failed for {}: {}",
            path,
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(())
}

/// Stamp a photo with EXIF capture metadata: camera make/model, the capture
/// timestamp (as `DateTimeOriginal`/`CreateDate`/`ModifyDate`), and the file
/// mtime.
///
/// Errors if `exiftool` is not available — a photo with no capture time is a
/// silent trap, so this refuses rather than skipping.
pub fn stamp_photo_metadata(
    path: &Utf8Path,
    camera: &CameraProfile,
    timestamp: DateTime<Utc>,
) -> Result<()> {
    let ts = exif_datetime(timestamp);
    let args = vec![
        format!("-Make={}", camera.make),
        format!("-Model={}", camera.model),
        format!("-DateTimeOriginal={}", ts),
        format!("-CreateDate={}", ts),
        format!("-ModifyDate={}", ts),
        format!("-FileModifyDate={}", ts),
    ];
    run_exiftool(path, &args)
}

/// Stamp a video with QuickTime capture metadata and set its file mtime.
///
/// Videos carry no EXIF, and folio's ingestion reads video capture time from
/// the filesystem mtime; `-FileModifyDate` is what makes that correct, while
/// the QuickTime `CreateDate`/make/model keep the container honest for tools
/// that read it.
pub fn stamp_video_metadata(
    path: &Utf8Path,
    camera: &CameraProfile,
    timestamp: DateTime<Utc>,
) -> Result<()> {
    let ts = exif_datetime(timestamp);
    let args = vec![
        format!("-QuickTime:Make={}", camera.make),
        format!("-QuickTime:Model={}", camera.model),
        format!("-QuickTime:CreateDate={}", ts),
        format!("-QuickTime:ModifyDate={}", ts),
        format!("-FileModifyDate={}", ts),
    ];
    run_exiftool(path, &args)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exif_datetime_formats_colons() {
        let ts = chrono::TimeZone::with_ymd_and_hms(&Utc, 2024, 11, 28, 14, 5, 9).unwrap();
        assert_eq!(exif_datetime(ts), "2024:11:28 14:05:09");
    }
}
