//! The [`ImageSource`] trait and its real backends.

use anyhow::{anyhow, bail, Context, Result};
use async_trait::async_trait;
use image::{DynamicImage, ImageReader};
use std::io::Cursor;
use std::time::{Duration, Instant};

/// A source of generated images. Backends implement this so a [`Generator`]
/// (and tests, via [`crate::testing::SolidColorSource`]) can be backend-agnostic.
///
/// [`Generator`]: crate::Generator
#[async_trait]
pub trait ImageSource: Send + Sync {
    /// Produce a raw image for `prompt`. The [`Generator`] resizes and stamps
    /// it, so implementations return the model's output as-is.
    ///
    /// [`Generator`]: crate::Generator
    async fn generate(&self, prompt: &str) -> Result<DynamicImage>;
}

/// Decode raw image bytes into a [`DynamicImage`], guessing the format.
fn decode_image(bytes: &[u8]) -> Result<DynamicImage> {
    ImageReader::new(Cursor::new(bytes))
        .with_guessed_format()
        .context("failed to guess image format")?
        .decode()
        .context("failed to decode image")
}

/// Remote backend: OpenAI DALL-E 3.
///
/// Requires an API key; construct with [`OpenAiSource::from_env`] to read
/// `OPENAI_API_KEY`, or [`OpenAiSource::new`] with an explicit client.
pub struct OpenAiSource {
    client: async_openai::Client<async_openai::config::OpenAIConfig>,
}

impl OpenAiSource {
    /// Build from an explicit async-openai client.
    pub fn new(client: async_openai::Client<async_openai::config::OpenAIConfig>) -> Self {
        Self { client }
    }

    /// Build from the `OPENAI_API_KEY` environment variable.
    ///
    /// Errors if the variable is unset, rather than deferring to an opaque
    /// auth failure at request time.
    pub fn from_env() -> Result<Self> {
        std::env::var("OPENAI_API_KEY")
            .context("OPENAI_API_KEY is required for the OpenAI backend")?;
        Ok(Self::new(async_openai::Client::with_config(
            async_openai::config::OpenAIConfig::default(),
        )))
    }
}

#[async_trait]
impl ImageSource for OpenAiSource {
    async fn generate(&self, prompt: &str) -> Result<DynamicImage> {
        use async_openai::types::{
            CreateImageRequestArgs, Image, ImageModel, ImageQuality, ImageSize,
        };

        let request = CreateImageRequestArgs::default()
            .model(ImageModel::DallE3)
            .prompt(prompt)
            .size(ImageSize::S1024x1024)
            .quality(ImageQuality::Standard)
            .n(1)
            .build()?;

        let response = self.client.images().create(request).await?;
        let image = response
            .data
            .first()
            .ok_or_else(|| anyhow!("OpenAI returned no image data"))?;

        let bytes = match &**image {
            Image::Url { url, .. } => reqwest::get(url).await?.bytes().await?.to_vec(),
            Image::B64Json { b64_json, .. } => {
                use base64::Engine;
                base64::engine::general_purpose::STANDARD.decode(b64_json.as_str())?
            }
        };
        decode_image(&bytes)
    }
}

/// Local backend: ComfyUI (Stable Diffusion / Flux) over HTTP.
pub struct ComfyUiSource {
    client: reqwest::Client,
    api_url: String,
}

impl ComfyUiSource {
    /// Point at a running ComfyUI server (e.g. `http://localhost:8188`).
    pub fn new(api_url: impl Into<String>) -> Self {
        Self {
            client: reqwest::Client::new(),
            api_url: api_url.into(),
        }
    }

    /// The configured ComfyUI base URL.
    pub fn api_url(&self) -> &str {
        &self.api_url
    }

    /// Poll ComfyUI's history endpoint until the prompt produces an image.
    ///
    /// Two limits guard the wait: an overall cap, and a stall timeout that
    /// fires when the server stops answering entirely. ComfyUI's `/history`
    /// stays empty until a job *finishes*, so this cannot tell a slow-but-live
    /// generation from a hung one — the stall guard catches a dead or
    /// unreachable server, not a wedged-but-responding one.
    async fn wait_for_image(&self, prompt_id: &str) -> Result<String> {
        const OVERALL_LIMIT: Duration = Duration::from_secs(2400); // 40 min
        const STALL_LIMIT: Duration = Duration::from_secs(180); // 3 min unreachable
        const POLL: Duration = Duration::from_secs(1);

        let started = Instant::now();
        let mut last_response = Instant::now();

        loop {
            if started.elapsed() > OVERALL_LIMIT {
                bail!("timed out waiting for ComfyUI image (> 40 minutes)");
            }
            if last_response.elapsed() > STALL_LIMIT {
                bail!(
                    "ComfyUI stopped responding for {} minutes — check the ComfyUI terminal",
                    STALL_LIMIT.as_secs() / 60
                );
            }
            tokio::time::sleep(POLL).await;

            let url = format!("{}/history/{}", self.api_url, prompt_id);
            let resp = match self.client.get(&url).send().await {
                Ok(r) => r,
                Err(_) => continue, // unreachable; the stall/overall limits apply
            };
            last_response = Instant::now(); // the server answered — it is alive
            if !resp.status().is_success() {
                continue;
            }
            let history: serde_json::Value = match resp.json().await {
                Ok(v) => v,
                Err(_) => continue,
            };
            if let Some(name) = find_output_filename(&history, prompt_id) {
                return Ok(name);
            }
        }
    }
}

/// Extract a finished job's output image filename from a ComfyUI `/history`
/// response. Returns `None` while the job is still running (history not yet
/// populated) or if the response has no image output.
fn find_output_filename(history: &serde_json::Value, prompt_id: &str) -> Option<String> {
    let outputs = history[prompt_id]["outputs"].as_object()?;
    for output in outputs.values() {
        if let Some(name) = output["images"]
            .as_array()
            .and_then(|imgs| imgs.first())
            .and_then(|img| img["filename"].as_str())
        {
            return Some(name.to_string());
        }
    }
    None
}

#[async_trait]
impl ImageSource for ComfyUiSource {
    async fn generate(&self, prompt: &str) -> Result<DynamicImage> {
        let workflow = comfyui_workflow(prompt, 1024, 1024, rand::random::<u32>());

        let resp = self
            .client
            .post(format!("{}/prompt", self.api_url))
            .json(&serde_json::json!({ "prompt": workflow }))
            .send()
            .await
            .with_context(|| {
                format!(
                    "failed to reach ComfyUI at {} — is it running?",
                    self.api_url
                )
            })?;
        if !resp.status().is_success() {
            bail!("ComfyUI rejected the prompt: {}", resp.status());
        }

        let submitted: serde_json::Value = resp.json().await?;
        let prompt_id = submitted["prompt_id"]
            .as_str()
            .context("ComfyUI response had no prompt_id")?;

        let filename = self.wait_for_image(prompt_id).await?;
        let view_url = format!("{}/view?filename={}", self.api_url, filename);
        let bytes = self.client.get(&view_url).send().await?.bytes().await?;
        decode_image(&bytes)
    }
}

/// Build a ComfyUI text-to-image workflow (Flux.1-dev shape) for `prompt`.
///
/// The `filename_prefix` is generic (`"generated"`) so the workflow carries no
/// caller-specific branding.
///
/// # Example
///
/// ```
/// use folio_generator::comfyui_workflow;
///
/// let wf = comfyui_workflow("a red barn", 512, 512, 42);
/// // The positive-prompt node carries the prompt text.
/// assert!(wf["2"]["inputs"]["text"].as_str().unwrap().contains("a red barn"));
/// // Latent canvas honors the requested dimensions.
/// assert_eq!(wf["4"]["inputs"]["width"], 512);
/// assert_eq!(wf["4"]["inputs"]["height"], 512);
/// // No caller-specific branding leaks into the output filename.
/// assert_eq!(wf["7"]["inputs"]["filename_prefix"], "generated");
/// ```
pub fn comfyui_workflow(prompt: &str, width: u32, height: u32, seed: u32) -> serde_json::Value {
    serde_json::json!({
        "1": {
            "inputs": { "ckpt_name": "flux1-dev-fp8.safetensors" },
            "class_type": "CheckpointLoaderSimple"
        },
        "2": {
            "inputs": { "text": prompt, "clip": ["1", 1] },
            "class_type": "CLIPTextEncode"
        },
        "3": {
            "inputs": { "text": "", "clip": ["1", 1] },
            "class_type": "CLIPTextEncode"
        },
        "35": {
            "inputs": { "guidance": 3.5, "conditioning": ["2", 0] },
            "class_type": "FluxGuidance"
        },
        "4": {
            "inputs": { "width": width, "height": height, "batch_size": 1 },
            "class_type": "EmptySD3LatentImage"
        },
        "5": {
            "inputs": {
                "seed": seed as i64,
                "steps": 20,
                "cfg": 1.0,
                "sampler_name": "euler",
                "scheduler": "simple",
                "denoise": 1.0,
                "model": ["1", 0],
                "positive": ["35", 0],
                "negative": ["3", 0],
                "latent_image": ["4", 0]
            },
            "class_type": "KSampler"
        },
        "6": {
            "inputs": { "samples": ["5", 0], "vae": ["1", 2] },
            "class_type": "VAEDecode"
        },
        "7": {
            "inputs": { "filename_prefix": "generated", "images": ["6", 0] },
            "class_type": "SaveImage"
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn workflow_embeds_prompt_and_dimensions() {
        let wf = comfyui_workflow("a red barn", 640, 480, 7);
        assert!(wf["2"]["inputs"]["text"]
            .as_str()
            .unwrap()
            .contains("a red barn"));
        assert_eq!(wf["4"]["inputs"]["width"], 640);
        assert_eq!(wf["4"]["inputs"]["height"], 480);
        assert_eq!(wf["5"]["inputs"]["seed"], 7);
    }

    #[test]
    fn workflow_uses_generic_filename_prefix() {
        let wf = comfyui_workflow("x", 512, 512, 1);
        assert_eq!(wf["7"]["inputs"]["filename_prefix"], "generated");
        // Guard against re-introducing caller-specific branding.
        assert!(!wf.to_string().to_lowercase().contains("folio"));
    }

    #[test]
    fn find_output_filename_none_while_running() {
        // Empty history (job not started/finished).
        assert_eq!(find_output_filename(&serde_json::json!({}), "abc"), None);
        // Outputs present but no image yet.
        let running = serde_json::json!({ "abc": { "outputs": {} } });
        assert_eq!(find_output_filename(&running, "abc"), None);
    }

    #[test]
    fn find_output_filename_extracts_completed_filename() {
        let done = serde_json::json!({
            "abc": { "outputs": { "7": { "images": [ { "filename": "generated_00001_.png" } ] } } }
        });
        assert_eq!(
            find_output_filename(&done, "abc").as_deref(),
            Some("generated_00001_.png")
        );
        // A different prompt id does not match.
        assert_eq!(find_output_filename(&done, "other"), None);
    }

    #[test]
    fn decode_image_roundtrips_and_rejects_garbage() {
        use image::GenericImageView;
        let img = DynamicImage::ImageRgb8(image::RgbImage::from_pixel(8, 8, image::Rgb([1, 2, 3])));
        let mut bytes = Vec::new();
        img.write_to(&mut Cursor::new(&mut bytes), image::ImageFormat::Jpeg)
            .unwrap();
        assert_eq!(decode_image(&bytes).unwrap().dimensions(), (8, 8));
        assert!(decode_image(b"not an image").is_err());
    }

    #[test]
    fn openai_from_env_requires_key() {
        // Save/restore so the test is order-independent.
        let prior = std::env::var("OPENAI_API_KEY").ok();
        std::env::remove_var("OPENAI_API_KEY");
        assert!(OpenAiSource::from_env().is_err());
        if let Some(v) = prior {
            std::env::set_var("OPENAI_API_KEY", v);
        }
    }
}
