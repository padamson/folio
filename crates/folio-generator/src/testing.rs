//! Test helpers: an offline [`ImageSource`] for exercising the pipeline without
//! a real model.

use anyhow::Result;
use async_trait::async_trait;
use image::DynamicImage;

use crate::source::ImageSource;

/// An [`ImageSource`] that returns a solid-color image, ignoring the prompt.
///
/// It lets tests (in this crate and in consumers) drive a [`Generator`] end to
/// end — resize, save, EXIF stamping, video stitching — with no network, API
/// key, or GPU. The returned image is small; the generator resizes it to the
/// configured [`PhotoSpec`] dimensions.
///
/// [`Generator`]: crate::Generator
/// [`PhotoSpec`]: crate::PhotoSpec
///
/// # Example
///
/// ```
/// use folio_generator::testing::SolidColorSource;
/// let source = SolidColorSource::teal();
/// ```
pub struct SolidColorSource {
    rgb: [u8; 3],
    size: u32,
}

impl SolidColorSource {
    /// A source producing images of the given RGB color.
    pub fn new(rgb: [u8; 3]) -> Self {
        Self { rgb, size: 64 }
    }

    /// A teal source — a convenient default for tests.
    pub fn teal() -> Self {
        Self::new([0, 128, 128])
    }
}

#[async_trait]
impl ImageSource for SolidColorSource {
    async fn generate(&self, _prompt: &str) -> Result<DynamicImage> {
        let img = image::RgbImage::from_pixel(self.size, self.size, image::Rgb(self.rgb));
        Ok(DynamicImage::ImageRgb8(img))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn solid_source_ignores_prompt_and_returns_color() {
        use image::GenericImageView;
        let source = SolidColorSource::new([10, 20, 30]);
        let img = source.generate("anything at all").await.unwrap();
        assert_eq!(img.dimensions(), (64, 64));
        let px = img.get_pixel(0, 0);
        assert_eq!([px[0], px[1], px[2]], [10, 20, 30]);
    }
}
