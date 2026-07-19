//! End-to-end: a prompt list + a timeline produce a directory of photos (and a
//! video) with correct, increasing capture timestamps — driven entirely through
//! the public API with the offline source, no model required.
//!
//! This is the folio-agnostic exercise ADR-0005 calls for: prove the engine
//! stands on its own. Requires `exiftool` (and `ffmpeg` for the video leg);
//! skips cleanly otherwise.

use camino::{Utf8Path, Utf8PathBuf};
use chrono::{DateTime, Duration, TimeZone, Utc};
use folio_generator::testing::SolidColorSource;
use folio_generator::timeline::spread;
use folio_generator::{
    exiftool_available, ffmpeg_available, stitch_video, CameraProfile, Generator, PhotoSpec,
};

fn read_capture_time(path: &Utf8Path) -> Option<DateTime<Utc>> {
    let file = std::fs::File::open(path).ok()?;
    let mut reader = std::io::BufReader::new(&file);
    let exif = exif::Reader::new().read_from_container(&mut reader).ok()?;
    // kamadak-exif humanizes DateTime display with hyphens in the date.
    let raw = exif
        .get_field(exif::Tag::DateTimeOriginal, exif::In::PRIMARY)?
        .display_value()
        .to_string();
    let naive = chrono::NaiveDateTime::parse_from_str(&raw, "%Y-%m-%d %H:%M:%S").ok()?;
    Some(naive.and_utc())
}

#[tokio::test]
async fn generates_a_timestamped_collection() {
    if !exiftool_available() {
        eprintln!("skipping: exiftool not installed");
        return;
    }

    let dir = tempfile::tempdir().unwrap();
    let dir = Utf8PathBuf::from_path_buf(dir.path().to_path_buf()).unwrap();

    let spec = PhotoSpec {
        width: 400,
        height: 300,
        camera: CameraProfile::new("ACME", "ACME Cam 1"),
        prompt_suffix: None,
    };
    let generator = Generator::new(Box::new(SolidColorSource::teal()), spec);

    // A prompt list spread across a two-hour window.
    let prompts = ["a", "b", "c", "d", "e"];
    let start = Utc.with_ymd_and_hms(2024, 11, 28, 14, 0, 0).unwrap();
    let stamps = spread(start, start + Duration::hours(2), prompts.len());

    let mut photos: Vec<Utf8PathBuf> = Vec::new();
    for (i, (prompt, ts)) in prompts.iter().zip(&stamps).enumerate() {
        let path = dir.join(format!("IMG_{:03}.jpg", i));
        generator.generate_photo(prompt, &path, *ts).await.unwrap();
        photos.push(path);
    }

    // Every photo exists at the right size with the right, increasing capture
    // time.
    let mut previous: Option<DateTime<Utc>> = None;
    for (path, expected) in photos.iter().zip(&stamps) {
        assert!(folio_generator::is_valid_image(path, 400, 300));
        let got = read_capture_time(path).expect("capture time present");
        assert_eq!(got, *expected);
        if let Some(prev) = previous {
            assert!(got > prev, "timestamps must increase");
        }
        previous = Some(got);
    }

    // The video leg, when ffmpeg is available.
    if ffmpeg_available() {
        let refs: Vec<&Utf8Path> = photos.iter().map(|p| p.as_path()).collect();
        let video = dir.join("clip.mov");
        stitch_video(
            &refs,
            &video,
            6.0,
            &CameraProfile::new("ACME", "ACME Cam 1"),
            *stamps.last().unwrap(),
        )
        .unwrap();
        assert!(video.exists());
    }
}
