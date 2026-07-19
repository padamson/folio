//! Video synthesis from still frames via `ffmpeg`.

use anyhow::{bail, Context, Result};
use camino::Utf8Path;
use chrono::{DateTime, Utc};
use std::process::Command;

use crate::generator::CameraProfile;
use crate::metadata::stamp_video_metadata;

/// Whether `ffmpeg` is on the PATH.
pub fn ffmpeg_available() -> bool {
    Command::new("ffmpeg")
        .arg("-version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Stitch `frames` (in order) into an H.264 video at `output`, then stamp it
/// with capture metadata so it participates in timeline/temporal grouping the
/// same way the photos do.
///
/// `fps` sets both the frame cadence and the encoded frame rate. Fails if
/// `ffmpeg` (or, for stamping, `exiftool`) is unavailable, if no frames are
/// given, or if a frame path is missing.
pub fn stitch_video(
    frames: &[&Utf8Path],
    output: &Utf8Path,
    fps: f64,
    camera: &CameraProfile,
    timestamp: DateTime<Utc>,
) -> Result<()> {
    if frames.is_empty() {
        bail!("cannot stitch a video from zero frames");
    }
    if !ffmpeg_available() {
        bail!("ffmpeg is required to stitch videos but was not found on PATH");
    }
    if fps <= 0.0 {
        bail!("fps must be positive, got {}", fps);
    }

    // ffmpeg concat demuxer: a list of `file`/`duration` pairs.
    let temp = tempfile::tempdir().context("failed to create temp dir")?;
    let list_path = temp.path().join("frames.txt");
    let per_frame = 1.0 / fps;
    let mut list = String::new();
    for frame in frames {
        if !frame.exists() {
            bail!("frame does not exist: {}", frame);
        }
        list.push_str(&format!("file '{}'\n", frame));
        list.push_str(&format!("duration {}\n", per_frame));
    }
    std::fs::write(&list_path, list).context("failed to write ffmpeg frame list")?;

    if let Some(parent) = output.parent() {
        std::fs::create_dir_all(parent).with_context(|| format!("failed to create {}", parent))?;
    }

    let result = Command::new("ffmpeg")
        .args(["-f", "concat", "-safe", "0", "-i"])
        .arg(&list_path)
        .args(["-vf", &format!("fps={}", fps)])
        .args(["-c:v", "libx264", "-pix_fmt", "yuv420p", "-y"])
        .arg(output.as_str())
        .output()
        .context("failed to run ffmpeg")?;
    if !result.status.success() {
        bail!("ffmpeg failed: {}", String::from_utf8_lossy(&result.stderr));
    }

    stamp_video_metadata(output, camera, timestamp)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stitch_rejects_empty_frames() {
        let camera = CameraProfile::new("m", "d");
        let err = stitch_video(&[], Utf8Path::new("/tmp/x.mov"), 6.0, &camera, Utc::now())
            .unwrap_err()
            .to_string();
        assert!(err.contains("zero frames"), "unexpected error: {}", err);
    }
}
