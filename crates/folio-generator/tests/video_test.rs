//! Integration: stitching frames produces a real video stamped with capture
//! metadata (the fix for videos that previously carried no timestamp).
//!
//! Requires `ffmpeg` (and `exiftool` for the metadata assertion); skips cleanly
//! when either is missing.

use camino::{Utf8Path, Utf8PathBuf};
use chrono::{TimeZone, Utc};
use folio_generator::testing::SolidColorSource;
use folio_generator::{
    exiftool_available, ffmpeg_available, stitch_video, CameraProfile, Generator, PhotoSpec,
};

async fn make_frames(dir: &Utf8Path, n: usize) -> Vec<Utf8PathBuf> {
    let spec = PhotoSpec {
        width: 320,
        height: 240,
        camera: CameraProfile::new("ACME", "ACME Cam"),
        prompt_suffix: None,
    };
    let generator = Generator::new(Box::new(SolidColorSource::teal()), spec);
    let ts = Utc.with_ymd_and_hms(2024, 11, 28, 14, 0, 0).unwrap();
    let mut frames = Vec::new();
    for i in 0..n {
        let path = dir.join(format!("frame_{:03}.jpg", i));
        generator.generate_photo("frame", &path, ts).await.unwrap();
        frames.push(path);
    }
    frames
}

#[tokio::test]
async fn stitched_video_exists_and_is_stamped() {
    if !ffmpeg_available() || !exiftool_available() {
        eprintln!("skipping: ffmpeg and/or exiftool not installed");
        return;
    }

    let dir = tempfile::tempdir().unwrap();
    let dir = Utf8Path::from_path(dir.path()).unwrap();
    let frames = make_frames(dir, 6).await;
    let frame_refs: Vec<&Utf8Path> = frames.iter().map(|p| p.as_path()).collect();

    let output = dir.join("clip.mov");
    let camera = CameraProfile::new("NIKON CORPORATION", "NIKON D800");
    let timestamp = Utc.with_ymd_and_hms(2024, 11, 28, 20, 30, 0).unwrap();
    stitch_video(&frame_refs, &output, 6.0, &camera, timestamp).unwrap();

    // A non-empty video file exists.
    assert!(output.exists());
    assert!(std::fs::metadata(output.as_std_path()).unwrap().len() > 0);

    // exiftool can read the QuickTime creation date we stamped.
    let out = std::process::Command::new("exiftool")
        .arg("-QuickTime:CreateDate")
        .arg("-s3")
        .arg(output.as_str())
        .output()
        .unwrap();
    let create_date = String::from_utf8_lossy(&out.stdout);
    assert!(
        create_date.contains("2024:11:28 20:30:00"),
        "unexpected QuickTime CreateDate: {:?}",
        create_date
    );

    // Frame cadence is honored: 6 frames at 6 fps is ~1 second, not the ~6s or
    // ~36s a wrong per-frame duration (1 % fps or 1 * fps) would yield.
    if let Some(seconds) = probe_duration_secs(&output) {
        assert!(
            seconds < 3.0,
            "6 frames at 6 fps should run ~1s, got {seconds}s"
        );
    }
}

/// Read a media file's duration in seconds via `ffprobe`, if available.
fn probe_duration_secs(path: &Utf8Path) -> Option<f64> {
    let out = std::process::Command::new("ffprobe")
        .args(["-v", "error", "-show_entries", "format=duration"])
        .args(["-of", "default=nw=1:nk=1"])
        .arg(path.as_str())
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    String::from_utf8_lossy(&out.stdout)
        .trim()
        .parse::<f64>()
        .ok()
}

#[tokio::test]
async fn stitch_rejects_missing_frame() {
    if !ffmpeg_available() {
        eprintln!("skipping: ffmpeg not installed");
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let dir = Utf8Path::from_path(dir.path()).unwrap();
    let missing = dir.join("nope.jpg");
    let camera = CameraProfile::new("m", "d");
    let err = stitch_video(
        &[missing.as_path()],
        &dir.join("out.mov"),
        6.0,
        &camera,
        Utc::now(),
    )
    .unwrap_err()
    .to_string();
    assert!(err.contains("does not exist"), "unexpected error: {}", err);
}
