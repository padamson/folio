//! Generate collections of AI photos and videos from prompts.
//!
//! `folio-generator` turns text prompts into realistic synthetic media — photos
//! with EXIF capture timestamps and videos with matching metadata — using either
//! a local model (ComfyUI / Stable Diffusion) or a remote one (OpenAI DALL-E).
//! It is deliberately application-agnostic: the caller supplies the prompts,
//! timeline, output paths, and camera profile. Folio uses it to build test
//! fixtures, but nothing here knows about folio.
//!
//! # Design
//!
//! The image backend is the [`ImageSource`] trait, so callers (and tests) can
//! plug in any source. Two real backends ship — [`OpenAiSource`] and
//! [`ComfyUiSource`] — and [`testing::SolidColorSource`] provides a fast,
//! offline fake for exercising the full pipeline without a model.
//!
//! A [`Generator`] pairs an [`ImageSource`] with a [`PhotoSpec`] (target
//! dimensions + camera profile) and produces individual photos; free functions
//! handle video synthesis ([`stitch_video`]), metadata stamping, timeline
//! spreading, and resume checks.
//!
//! # Example
//!
//! ```no_run
//! use camino::Utf8Path;
//! use chrono::Utc;
//! use folio_generator::{CameraProfile, Generator, PhotoSpec};
//! use folio_generator::testing::SolidColorSource;
//!
//! # tokio_test::block_on(async {
//! let spec = PhotoSpec {
//!     width: 800,
//!     height: 600,
//!     camera: CameraProfile::new("ACME", "ACME Cam 1"),
//!     prompt_suffix: None,
//! };
//! let generator = Generator::new(Box::new(SolidColorSource::teal()), spec);
//! generator
//!     .generate_photo("a sunset", Utf8Path::new("/tmp/shot.jpg"), Utc::now())
//!     .await?;
//! # Ok::<(), anyhow::Error>(())
//! # });
//! ```

mod generator;
mod metadata;
mod source;
pub mod testing;
pub mod timeline;
mod video;

pub use generator::{is_valid_image, CameraProfile, Generator, PhotoSpec};
pub use metadata::{exiftool_available, stamp_photo_metadata, stamp_video_metadata};
pub use source::{comfyui_workflow, ComfyUiSource, ImageSource, OpenAiSource};
pub use video::{ffmpeg_available, stitch_video};
