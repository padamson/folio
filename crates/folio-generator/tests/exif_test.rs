//! Integration: a generated photo carries the EXIF capture metadata we stamped.
//!
//! Uses the offline [`SolidColorSource`] so no model is needed; requires
//! `exiftool` (skips cleanly without it, so it runs in CI where the tool is
//! installed and no-ops for contributors who lack it).

use camino::Utf8PathBuf;
use chrono::{TimeZone, Utc};
use folio_generator::testing::SolidColorSource;
use folio_generator::{exiftool_available, CameraProfile, Generator, PhotoSpec};

fn read_exif_field(path: &camino::Utf8Path, tag: exif::Tag) -> Option<String> {
    let file = std::fs::File::open(path).ok()?;
    let mut reader = std::io::BufReader::new(&file);
    let exif = exif::Reader::new().read_from_container(&mut reader).ok()?;
    exif.get_field(tag, exif::In::PRIMARY)
        .map(|f| f.display_value().to_string())
}

#[tokio::test]
async fn generated_photo_has_stamped_exif() {
    if !exiftool_available() {
        eprintln!("skipping: exiftool not installed");
        return;
    }

    let dir = tempfile::tempdir().unwrap();
    let output = Utf8PathBuf::from_path_buf(dir.path().join("shot.jpg")).unwrap();

    let spec = PhotoSpec {
        width: 800,
        height: 600,
        camera: CameraProfile::new("NIKON CORPORATION", "NIKON D800"),
        prompt_suffix: Some("photorealistic".to_string()),
    };
    let generator = Generator::new(Box::new(SolidColorSource::teal()), spec);

    let timestamp = Utc.with_ymd_and_hms(2024, 11, 28, 14, 5, 9).unwrap();
    generator
        .generate_photo("a family dinner", &output, timestamp)
        .await
        .unwrap();

    // The file is a real 800x600 JPEG.
    assert!(folio_generator::is_valid_image(&output, 800, 600));

    // Capture time round-trips through EXIF.
    // kamadak-exif humanizes DateTime display with hyphens in the date.
    let dto = read_exif_field(&output, exif::Tag::DateTimeOriginal)
        .expect("DateTimeOriginal should be present");
    assert_eq!(dto, "2024-11-28 14:05:09");

    // Camera identity is stamped.
    let model = read_exif_field(&output, exif::Tag::Model).expect("Model should be present");
    assert!(model.contains("NIKON D800"), "got model: {}", model);
    let make = read_exif_field(&output, exif::Tag::Make).expect("Make should be present");
    assert!(make.contains("NIKON"), "got make: {}", make);
}
