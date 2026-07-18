//! Generate AI-powered example data using AI image generation
//!
//! Creates realistic example photos and videos for testing Folio ingestion workflows.
//!
//! Supports two backends:
//! - OpenAI DALL-E 3 (default): Requires OPENAI_API_KEY environment variable
//! - Local Stable Diffusion: Requires local ComfyUI or compatible API server
//!
//! Configure via environment variables in .env file:
//! - IMAGE_BACKEND=openai (default) or IMAGE_BACKEND=local
//! - OPENAI_API_KEY=sk-... (for OpenAI backend)
//! - LOCAL_SD_URL=http://localhost:8188 (for local backend, default)

use anyhow::{anyhow, Context, Result};
use async_openai::{
    config::OpenAIConfig,
    types::{CreateImageRequestArgs, ImageModel, ImageQuality, ImageSize},
    Client,
};
use camino::Utf8Path;
use chrono::{Duration, TimeZone, Utc};
use clap::Parser;
use dotenvy::dotenv;
use image::ImageReader;
use std::process::Command;

/// Image generation backend
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ImageBackend {
    OpenAI,
    Local,
}

impl ImageBackend {
    /// Detect backend from environment variable
    fn from_env() -> Result<Self> {
        match std::env::var("IMAGE_BACKEND").as_deref() {
            Ok("local") => Ok(Self::Local),
            Ok("openai") | Err(_) => Ok(Self::OpenAI),
            Ok(other) => Err(anyhow!(
                "Invalid IMAGE_BACKEND: {}. Use 'openai' or 'local'",
                other
            )),
        }
    }
}

/// Configuration for image generation
struct ImageConfig {
    backend: ImageBackend,
    openai_client: Option<Client<OpenAIConfig>>,
    local_url: Option<String>,
}

impl ImageConfig {
    /// Create configuration from environment
    fn from_env() -> Result<Self> {
        let backend = ImageBackend::from_env()?;

        let (openai_client, local_url) = match backend {
            ImageBackend::OpenAI => {
                // Verify API key is set
                std::env::var("OPENAI_API_KEY")
                    .context("OPENAI_API_KEY required for OpenAI backend. Set it in .env file")?;
                (Some(Client::with_config(OpenAIConfig::default())), None)
            }
            ImageBackend::Local => {
                let url = std::env::var("LOCAL_SD_URL")
                    .unwrap_or_else(|_| "http://localhost:8188".to_string());
                println!("Using local Stable Diffusion at {}", url);
                (None, Some(url))
            }
        };

        Ok(Self {
            backend,
            openai_client,
            local_url,
        })
    }
}

#[derive(Parser)]
#[command(name = "generate-examples")]
#[command(about = "Generate AI-powered example data for testing Folio workflows")]
struct Args {
    /// Force regeneration even if example-data directory exists
    #[arg(long)]
    force: bool,

    /// Number of photos to generate (default: 100)
    #[arg(long, default_value = "100")]
    count: usize,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Load .env file
    dotenv().ok();

    let args = Args::parse();

    // Check if example-data directory exists
    let example_data_dir = folio_examples::example_data_path()?;
    if example_data_dir.exists() && !args.force {
        println!(
            "Example data directory already exists at: {}",
            example_data_dir
        );
        println!("Use --force to regenerate");
        return Ok(());
    }

    // Initialize image generation configuration
    let config = ImageConfig::from_env()?;

    // Show backend being used
    match config.backend {
        ImageBackend::OpenAI => {
            println!("Using OpenAI DALL-E 3 for image generation");
            println!("Generating {} AI photos...", args.count);
            println!("This will take approximately {} minutes", args.count / 2);
        }
        ImageBackend::Local => {
            println!(
                "Using local Stable Diffusion at {}",
                config.local_url.as_ref().unwrap()
            );
            println!("Generating {} photos...", args.count);
            println!("Note: Time depends on your local setup");
        }
    }
    println!();

    // Create directory structure
    create_directory_structure(&example_data_dir)?;

    // Generate photos
    let photos_dir = example_data_dir
        .join("sd-card-thanksgiving")
        .join("DCIM")
        .join("100NIKON");

    generate_photos(&photos_dir, args.count, &config).await?;

    // Create videos from photos
    let video_count = create_videos(&photos_dir)?;

    // Create deduplication test data
    create_dedup_data(&example_data_dir, &photos_dir)?;

    println!();
    println!("✓ Example data generated successfully!");
    println!("  Location: {}", example_data_dir);
    println!("  Photos: {} JPEGs", args.count);
    if video_count > 0 {
        println!("  Videos: {} MOV files", video_count);
    } else {
        println!("  Videos: 0 (need 60+ photos for video generation)");
    }
    println!();
    println!("To test ingestion:");
    println!(
        "  cargo run --bin folio -- ingest --source {}/DCIM --archive {}",
        example_data_dir.join("sd-card-thanksgiving"),
        example_data_dir.join("archive")
    );

    Ok(())
}

/// Create the example data directory structure
fn create_directory_structure(root: &Utf8Path) -> Result<()> {
    println!("Creating directory structure...");

    // Main structure
    std::fs::create_dir_all(root.join("sd-card-thanksgiving/DCIM/100NIKON"))?;
    std::fs::create_dir_all(root.join("archive"))?;
    std::fs::create_dir_all(root.join("archive-with-existing/2024/11/04"))?;

    Ok(())
}

/// Generate AI photos using configured backend
async fn generate_photos(output_dir: &Utf8Path, count: usize, config: &ImageConfig) -> Result<()> {
    // Calculate batch splits (50% each)
    let batch1_count = count / 2;
    let batch2_count = count - batch1_count;

    // Track timing for progress estimation
    let mut total_images = 0;
    let mut total_duration_secs = 0.0;

    // Batch 1: 14:00-16:30 (Thanksgiving dinner prep)
    println!("Generating Batch 1: {} photos (14:00-16:30)", batch1_count);
    let batch1_start = Utc.with_ymd_and_hms(2024, 11, 28, 14, 0, 0).unwrap();
    let batch1_prompts = [
        "Family cooking Thanksgiving dinner in kitchen, warm lighting, candid moment",
        "Thanksgiving turkey in oven, home kitchen, natural light",
        "Family members preparing food together, cozy kitchen scene",
        "Thanksgiving table being set with plates and decorations",
        "Children helping with Thanksgiving preparations, happy scene",
    ];

    for i in 0..batch1_count {
        let filename = format!("DSC_{:04}.JPG", i + 1);
        let output_path = output_dir.join(&filename);

        // Calculate timestamp with some variation
        let minutes = (i as i64 * 150) / batch1_count as i64; // Spread over 150 minutes
        let timestamp = batch1_start + Duration::minutes(minutes);

        // Rotate through prompts
        let prompt = &batch1_prompts[i % batch1_prompts.len()];

        // Show progress with estimate
        let progress_info = if total_images > 0 {
            let avg_secs = total_duration_secs / total_images as f64;
            let remaining = count - total_images;
            let est_mins = (avg_secs * remaining as f64) / 60.0;
            format!(
                " (avg {:.1}min/img, ~{:.0}min remaining)",
                avg_secs / 60.0,
                est_mins
            )
        } else {
            String::new()
        };

        // Check if photo already exists and is valid
        if is_valid_photo(&output_path) {
            println!(
                "  [{}/{}] {} (already exists, skipping){}",
                total_images + 1,
                count,
                filename,
                progress_info
            );
            total_images += 1;
            continue;
        }

        println!(
            "  [{}/{}] Generating {}...{}",
            total_images + 1,
            count,
            filename,
            progress_info
        );

        let start_time = std::time::Instant::now();
        generate_single_photo(config, prompt, &output_path, timestamp).await?;
        let duration = start_time.elapsed().as_secs_f64();

        total_images += 1;
        total_duration_secs += duration;
    }

    // Batch 2: 19:30-22:30 (Thanksgiving dinner and dessert). Batch 1's last
    // photo lands at ~16:27, so starting here keeps the gap above the CLI's
    // default 2-hour --gap-threshold and the dataset demonstrates two batches.
    println!();
    println!("Generating Batch 2: {} photos (19:30-22:30)", batch2_count);
    let batch2_start = Utc.with_ymd_and_hms(2024, 11, 28, 19, 30, 0).unwrap();
    let batch2_prompts = [
        "Family gathered around Thanksgiving dinner table, warm ambiance",
        "Thanksgiving feast with turkey and side dishes, table setting",
        "Family enjoying meal together, happy celebration",
        "Thanksgiving desserts and pies on table, festive scene",
        "Family relaxing after dinner, cozy living room",
    ];

    for i in 0..batch2_count {
        let filename = format!("DSC_{:04}.JPG", batch1_count + i + 2); // +2 to leave room for videos
        let output_path = output_dir.join(&filename);

        // Calculate timestamp
        let minutes = (i as i64 * 180) / batch2_count as i64; // Spread over 180 minutes
        let timestamp = batch2_start + Duration::minutes(minutes);

        // Rotate through prompts
        let prompt = &batch2_prompts[i % batch2_prompts.len()];

        // Show progress with estimate
        let progress_info = if total_images > 0 {
            let avg_secs = total_duration_secs / total_images as f64;
            let remaining = count - total_images;
            let est_mins = (avg_secs * remaining as f64) / 60.0;
            format!(
                " (avg {:.1}min/img, ~{:.0}min remaining)",
                avg_secs / 60.0,
                est_mins
            )
        } else {
            String::new()
        };

        // Check if photo already exists and is valid
        if is_valid_photo(&output_path) {
            println!(
                "  [{}/{}] {} (already exists, skipping){}",
                total_images + 1,
                count,
                filename,
                progress_info
            );
            total_images += 1;
            continue;
        }

        println!(
            "  [{}/{}] Generating {}...{}",
            total_images + 1,
            count,
            filename,
            progress_info
        );

        let start_time = std::time::Instant::now();
        generate_single_photo(config, prompt, &output_path, timestamp).await?;
        let duration = start_time.elapsed().as_secs_f64();

        total_images += 1;
        total_duration_secs += duration;
    }

    Ok(())
}

/// Generate a single photo using configured backend
async fn generate_single_photo(
    config: &ImageConfig,
    prompt: &str,
    output_path: &Utf8Path,
    timestamp: chrono::DateTime<Utc>,
) -> Result<()> {
    match config.backend {
        ImageBackend::OpenAI => {
            generate_with_openai(
                config.openai_client.as_ref().unwrap(),
                prompt,
                output_path,
                timestamp,
            )
            .await
        }
        ImageBackend::Local => {
            generate_with_local_sd(
                config.local_url.as_ref().unwrap(),
                prompt,
                output_path,
                timestamp,
            )
            .await
        }
    }
}

/// Generate image with OpenAI DALL-E 3
async fn generate_with_openai(
    client: &Client<OpenAIConfig>,
    prompt: &str,
    output_path: &Utf8Path,
    timestamp: chrono::DateTime<Utc>,
) -> Result<()> {
    // Create image request
    let request = CreateImageRequestArgs::default()
        .model(ImageModel::DallE3)
        .prompt(format!(
            "{}, photorealistic, DSLR quality, 4:3 aspect ratio",
            prompt
        ))
        .size(ImageSize::S1024x1024) // We'll resize to 800x600
        .quality(ImageQuality::Standard)
        .n(1)
        .build()?;

    // Generate image
    let response = client.images().create(request).await?;

    // Get the image data from response (Image is an enum with Url or B64Json variants)
    let image_data = &*response.data[0]; // Dereference Arc to get Image enum

    let image_bytes = match image_data {
        async_openai::types::Image::Url { url, .. } => {
            // Download from URL (async)
            reqwest::get(url).await?.bytes().await?.to_vec()
        }
        async_openai::types::Image::B64Json { b64_json, .. } => {
            // Decode base64
            use base64::Engine;
            base64::engine::general_purpose::STANDARD.decode(b64_json.as_str())?
        }
    };

    // Load and resize image to 800x600
    let img = ImageReader::new(std::io::Cursor::new(&image_bytes))
        .with_guessed_format()?
        .decode()?;

    let resized = img.resize_exact(800, 600, image::imageops::FilterType::Lanczos3);

    // Save as JPEG
    resized.save(output_path)?;

    // Add EXIF metadata
    add_exif_metadata(output_path, timestamp)?;

    Ok(())
}

/// Add EXIF metadata to a JPEG file
fn add_exif_metadata(jpeg_path: &Utf8Path, timestamp: chrono::DateTime<Utc>) -> Result<()> {
    // Note: kamadak-exif 0.6 primarily reads EXIF, writing is limited
    // For full EXIF writing, we'd use exiftool via Command
    // For now, we'll use exiftool if available, otherwise skip

    // Try to use exiftool if available
    let exiftool_available = Command::new("exiftool").arg("-ver").output().is_ok();

    if !exiftool_available {
        eprintln!("Warning: exiftool not found, skipping EXIF metadata");
        eprintln!("Install exiftool for proper metadata: brew install exiftool");
        return Ok(());
    }

    // Format timestamp for EXIF
    let exif_timestamp = timestamp.format("%Y:%m:%d %H:%M:%S").to_string();

    // Write EXIF metadata using exiftool
    let status = Command::new("exiftool")
        .arg("-overwrite_original")
        .arg("-Make=NIKON CORPORATION")
        .arg("-Model=NIKON D800")
        .arg(format!("-DateTimeOriginal={}", exif_timestamp))
        .arg(format!("-CreateDate={}", exif_timestamp))
        .arg(format!("-ModifyDate={}", exif_timestamp))
        .arg("-ImageWidth=800")
        .arg("-ImageHeight=600")
        .arg("-XResolution=300")
        .arg("-YResolution=300")
        .arg("-FNumber=5.6")
        .arg("-ExposureTime=1/125")
        .arg("-ISO=400")
        .arg("-FocalLength=50.0")
        .arg(jpeg_path.as_str())
        .output()?;

    if !status.status.success() {
        eprintln!(
            "Warning: exiftool failed: {}",
            String::from_utf8_lossy(&status.stderr)
        );
    }

    Ok(())
}

/// Create videos from photo sequences using ffmpeg
fn create_videos(photos_dir: &Utf8Path) -> Result<usize> {
    println!();
    println!("Creating videos from photo sequences...");

    // Check if ffmpeg is available
    let ffmpeg_available = Command::new("ffmpeg").arg("-version").output().is_ok();

    if !ffmpeg_available {
        eprintln!("Warning: ffmpeg not found, skipping video creation");
        eprintln!("Install ffmpeg: brew install ffmpeg");
        return Ok(0);
    }

    // Count total photos
    let total_photos = std::fs::read_dir(photos_dir)?
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.path()
                .extension()
                .and_then(|ext| ext.to_str())
                .map(|ext| ext.eq_ignore_ascii_case("jpg"))
                .unwrap_or(false)
        })
        .count();

    // Only create videos if we have at least 60 photos (30 for each video)
    if total_photos < 60 {
        eprintln!(
            "  Skipping video creation (need at least 60 photos, have {})",
            total_photos
        );
        return Ok(0);
    }

    // Video 1: DSC_0051.MOV (from first 30 photos)
    println!("  Creating DSC_0051.MOV...");
    create_video_from_images(photos_dir, 1, 30, &photos_dir.join("DSC_0051.MOV"))?;

    // Video 2: DSC_0102.MOV (from last 30 photos)
    println!("  Creating DSC_0102.MOV...");
    create_video_from_images(
        photos_dir,
        total_photos.saturating_sub(29),
        total_photos,
        &photos_dir.join("DSC_0102.MOV"),
    )?;

    Ok(2)
}

/// Create a video from a sequence of images
fn create_video_from_images(
    photos_dir: &Utf8Path,
    start_index: usize,
    end_index: usize,
    output_path: &Utf8Path,
) -> Result<()> {
    // Create a temporary file list for ffmpeg
    let temp_dir = tempfile::tempdir()?;
    let file_list_path = temp_dir.path().join("file_list.txt");

    let mut file_list = String::new();
    for i in start_index..=end_index {
        let photo_path = photos_dir.join(format!("DSC_{:04}.JPG", i));
        if photo_path.exists() {
            file_list.push_str(&format!("file '{}'\n", photo_path));
            file_list.push_str("duration 0.166667\n"); // 6 fps = 1/6 second per frame
        }
    }

    std::fs::write(&file_list_path, file_list)?;

    // Create video with ffmpeg
    let status = Command::new("ffmpeg")
        .arg("-f")
        .arg("concat")
        .arg("-safe")
        .arg("0")
        .arg("-i")
        .arg(&file_list_path)
        .arg("-vf")
        .arg("fps=6")
        .arg("-c:v")
        .arg("libx264")
        .arg("-pix_fmt")
        .arg("yuv420p")
        .arg("-y") // Overwrite output file
        .arg(output_path.as_str())
        .output()?;

    if !status.status.success() {
        anyhow::bail!("ffmpeg failed: {}", String::from_utf8_lossy(&status.stderr));
    }

    Ok(())
}

/// Create deduplication test data
fn create_dedup_data(root: &Utf8Path, photos_dir: &Utf8Path) -> Result<()> {
    println!();
    println!("Creating deduplication test data...");

    let dedup_dir = root.join("archive-with-existing/2024/11/04");

    // Copy first photo as a duplicate
    let source = photos_dir.join("DSC_0001.JPG");
    let dest = dedup_dir.join("20241104-140215-old-batch.jpg");

    if source.exists() {
        std::fs::copy(&source, &dest)?;
        println!("  Created duplicate: {}", dest.file_name().unwrap());

        // Create a dummy XMP sidecar
        let xmp_content = r#"<?xml version="1.0" encoding="UTF-8"?>
<x:xmpmeta xmlns:x="adobe:ns:meta/">
  <rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#">
    <rdf:Description rdf:about=""
      xmlns:dc="http://purl.org/dc/elements/1.1/">
      <dc:description>Test duplicate for deduplication</dc:description>
    </rdf:Description>
  </rdf:RDF>
</x:xmpmeta>"#;

        std::fs::write(dest.with_extension("jpg.xmp"), xmp_content)?;
    }

    Ok(())
}

/// Check if a photo already exists and is valid (not corrupted, correct dimensions)
///
/// This allows resuming generation runs - skipping photos that already exist.
///
/// # Validation
///
/// A photo is considered valid if:
/// - File exists at the path
/// - File can be opened and decoded as a valid image
/// - Dimensions are exactly 800x600 (as specified in our generation workflow)
/// - Format is JPEG
///
/// # Returns
///
/// `true` if the photo exists and is valid, `false` otherwise
fn is_valid_photo(path: &Utf8Path) -> bool {
    // Check if file exists
    if !path.exists() {
        return false;
    }

    // Try to open and decode image to verify format and dimensions
    let reader = match ImageReader::open(path) {
        Ok(r) => r,
        Err(_) => return false,
    };

    // Verify it's a JPEG
    let format = match reader.format() {
        Some(f) => f,
        None => return false,
    };

    if format != image::ImageFormat::Jpeg {
        return false;
    }

    // Decode to check dimensions
    let img = match reader.decode() {
        Ok(i) => i,
        Err(_) => return false,
    };

    // Verify exact dimensions (800x600)
    if img.width() != 800 || img.height() != 600 {
        return false;
    }

    true
}

// ============================================================================
// Local Stable Diffusion (ComfyUI) Support
// ============================================================================

/// Generate image using local Stable Diffusion (ComfyUI API)
///
/// Communicates with a local ComfyUI server to generate images using Stable Diffusion.
/// Supports Flux.1-dev or SDXL models depending on what's available in ComfyUI.
///
/// # Process
///
/// 1. Creates a ComfyUI workflow JSON with text-to-image nodes
/// 2. Submits workflow to ComfyUI server via HTTP POST to `/prompt`
/// 3. Polls `/history/{prompt_id}` until image generation completes (max 40 minutes)
/// 4. Downloads generated image from `/view` endpoint
/// 5. Resizes image to 800x600 (DSLR photo dimensions)
/// 6. Adds EXIF metadata (camera model, timestamp, etc.)
///
/// # Arguments
///
/// * `api_url` - ComfyUI server URL (e.g., "http://localhost:8188")
/// * `prompt` - Text description for image generation
/// * `output_path` - Where to save the final JPEG image
/// * `timestamp` - Photo timestamp for EXIF metadata
///
/// # Errors
///
/// Returns error if:
/// - ComfyUI server is not running or unreachable
/// - Image generation times out (> 40 minutes)
/// - Image download or processing fails
/// - File write fails
async fn generate_with_local_sd(
    api_url: &str,
    prompt: &str,
    output_path: &Utf8Path,
    timestamp: chrono::DateTime<Utc>,
) -> Result<()> {
    // 1. Create ComfyUI workflow
    let workflow = create_comfyui_workflow(prompt);

    // 2. Submit workflow to ComfyUI
    let client = reqwest::Client::new();
    let response = client
        .post(format!("{}/prompt", api_url))
        .json(&serde_json::json!({
            "prompt": workflow
        }))
        .send()
        .await
        .context(format!(
            "Failed to connect to local SD at {}. Is ComfyUI running?",
            api_url
        ))?;

    if !response.status().is_success() {
        anyhow::bail!("ComfyUI prompt submission failed: {}", response.status());
    }

    let prompt_response: serde_json::Value = response.json().await?;
    let prompt_id = prompt_response["prompt_id"]
        .as_str()
        .context("Missing prompt_id in ComfyUI response")?;

    // 3. Poll for completion
    let output_filename = wait_for_completion(&client, api_url, prompt_id).await?;

    // 4. Download generated image
    let image_url = format!("{}/view?filename={}", api_url, output_filename);
    let image_bytes = client.get(&image_url).send().await?.bytes().await?;

    // 5. Resize to 800x600 and save
    let img = ImageReader::new(std::io::Cursor::new(&image_bytes))
        .with_guessed_format()?
        .decode()?;

    let resized = img.resize_exact(800, 600, image::imageops::FilterType::Lanczos3);
    resized.save(output_path)?;

    // 6. Add EXIF metadata
    add_exif_metadata(output_path, timestamp)?;

    Ok(())
}

/// Wait for ComfyUI to finish generating the image
///
/// Polls the ComfyUI `/history/{prompt_id}` endpoint every second until
/// the image generation completes or times out.
///
/// # Timeouts
///
/// - **Overall timeout**: 2400 seconds (40 minutes) per image
/// - **Activity timeout**: 180 seconds (3 minutes) without any response from ComfyUI
///
/// # Returns
///
/// The filename of the generated image in ComfyUI's output directory.
///
/// # Errors
///
/// Returns error if:
/// - Image generation times out (> 40 minutes)
/// - No response from ComfyUI for > 3 minutes (stalled)
async fn wait_for_completion(
    client: &reqwest::Client,
    api_url: &str,
    prompt_id: &str,
) -> Result<String> {
    use std::io::Write;

    eprintln!("    Waiting for image generation (check ComfyUI terminal for progress)");
    std::io::stderr().flush().ok();

    let mut last_activity = std::time::Instant::now();
    const ACTIVITY_TIMEOUT_SECS: u64 = 180; // 3 minutes of no response = stalled

    // Poll history endpoint until complete
    for i in 0..2400 {
        // Wait up to 2400 seconds (40 minutes) - Flux can take 20-30 min on M3 Max
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

        // Show progress dots every 30 seconds to avoid spam
        if i > 0 && i % 30 == 0 {
            eprint!(".");
            std::io::stderr().flush().ok();
        }

        // Check activity timeout
        if last_activity.elapsed().as_secs() > ACTIVITY_TIMEOUT_SECS {
            eprintln!();
            anyhow::bail!(
                "ComfyUI appears stalled (no response for {} minutes). Check ComfyUI terminal for errors.",
                ACTIVITY_TIMEOUT_SECS / 60
            );
        }

        let history_url = format!("{}/history/{}", api_url, prompt_id);
        let response = client.get(&history_url).send().await;

        match response {
            Ok(resp) if resp.status().is_success() => {
                // Got a successful response - reset activity timer
                last_activity = std::time::Instant::now();

                if let Ok(history) = resp.json::<serde_json::Value>().await {
                    // Check if completed
                    if let Some(outputs) = history[prompt_id]["outputs"].as_object() {
                        // Find SaveImage node output
                        for (_, output) in outputs {
                            if let Some(images) = output["images"].as_array() {
                                if let Some(first_image) = images.first() {
                                    if let Some(filename) = first_image["filename"].as_str() {
                                        eprintln!(" done ({:.1} minutes)", (i + 1) as f64 / 60.0);
                                        return Ok(filename.to_string());
                                    }
                                }
                            }
                        }
                    }
                }
            }
            Ok(_) => {
                // Non-success status code, but we got a response
                last_activity = std::time::Instant::now();
            }
            Err(_) => {
                // Network error or connection issue - keep waiting but don't reset timer
                continue;
            }
        }
    }

    eprintln!();
    anyhow::bail!("Timeout waiting for image generation (> 40 minutes)")
}

/// Create ComfyUI workflow JSON for text-to-image generation
///
/// Generates a ComfyUI workflow with nodes for:
/// - CheckpointLoader (Flux.1-dev or SDXL model)
/// - CLIPTextEncode (positive and negative prompts)
/// - EmptyLatentImage (1024x1024 canvas)
/// - KSampler (20 steps, Euler sampler)
/// - VAEDecode
/// - SaveImage
///
/// # Arguments
///
/// * `prompt` - User-provided text description (enhanced with DSLR quality modifiers)
///
/// # Note
///
/// The workflow assumes Flux.1-dev model is available. If not, ComfyUI may fall back
/// to another checkpoint or fail. Users should ensure their ComfyUI has a compatible
/// model loaded.
fn create_comfyui_workflow(prompt: &str) -> serde_json::Value {
    // Flux.1-dev workflow structure
    // Based on: https://raw.githubusercontent.com/Comfy-Org/example_workflows/main/flux/text-to-image/flux_dev_fp8.json
    //
    // Key differences from SDXL:
    // - FluxGuidance node (replaces traditional CFG)
    // - EmptySD3LatentImage (instead of EmptyLatentImage)
    // - CFG=1.0 (Flux ignores negative prompts)
    // - Scheduler="simple" (not "normal")
    serde_json::json!({
        "1": {
            "inputs": {
                "ckpt_name": "flux1-dev-fp8.safetensors"
            },
            "class_type": "CheckpointLoaderSimple"
        },
        "2": {
            "inputs": {
                "text": format!("{}, photorealistic, DSLR quality, 4:3 aspect ratio", prompt),
                "clip": ["1", 1]
            },
            "class_type": "CLIPTextEncode"
        },
        "3": {
            "inputs": {
                "text": "",  // Flux ignores negative prompts
                "clip": ["1", 1]
            },
            "class_type": "CLIPTextEncode"
        },
        "35": {
            "inputs": {
                "guidance": 3.5,  // Flux-specific guidance parameter
                "conditioning": ["2", 0]
            },
            "class_type": "FluxGuidance"
        },
        "4": {
            "inputs": {
                "width": 1024,
                "height": 1024,
                "batch_size": 1
            },
            "class_type": "EmptySD3LatentImage"  // SD3/Flux use this instead of EmptyLatentImage
        },
        "5": {
            "inputs": {
                "seed": rand::random::<u32>() as i64,
                "steps": 20,
                "cfg": 1.0,  // Flux uses guidance via FluxGuidance node, not CFG
                "sampler_name": "euler",
                "scheduler": "simple",  // Flux uses "simple" scheduler
                "denoise": 1.0,
                "model": ["1", 0],
                "positive": ["35", 0],  // From FluxGuidance, not raw CLIPTextEncode
                "negative": ["3", 0],
                "latent_image": ["4", 0]
            },
            "class_type": "KSampler"
        },
        "6": {
            "inputs": {
                "samples": ["5", 0],
                "vae": ["1", 2]
            },
            "class_type": "VAEDecode"
        },
        "7": {
            "inputs": {
                "filename_prefix": "folio_example",
                "images": ["6", 0]
            },
            "class_type": "SaveImage"
        }
    })
}
