# ComfyUI Setup for Folio Example Data Generation

This guide shows how to set up local Stable Diffusion image generation using ComfyUI for generating Folio example data.

## Why Local Generation?

**Benefits:**
- **Free**: No API costs after initial setup
- **Private**: Images generated locally, no cloud services
- **Fast on M3 Max**: 30-60 seconds per image with 36GB RAM
- **Unlimited**: Generate as many images as needed

**Trade-offs:**
- **Initial setup**: ~2-4 hours to download models and configure
- **Disk space**: ~10-20GB for models
- **Quality**: Slightly different aesthetic than DALL-E 3

## Prerequisites

- **Hardware**: M3 Max with 36GB RAM (or similar Apple Silicon)
- **Disk space**: 20GB free (for ComfyUI + models)
- **Time**: 2-4 hours for initial setup

## Step 1: Install ComfyUI

### Option A: Using Git (Recommended)

```bash
# Clone ComfyUI
cd ~/opt # or wherever you want to install
git clone https://github.com/comfyanonymous/ComfyUI.git
cd ComfyUI

# Install dependencies
pip3 install -r requirements.txt

# (Optional) Install PyTorch with MPS (Metal Performance Shaders) for Apple Silicon
pip3 install --pre torch torchvision torchaudio --extra-index-url https://download.pytorch.org/whl/nightly/cpu
```

### Option B: Download Release

1. Download latest release from https://github.com/comfyanonymous/ComfyUI/releases
2. Extract to `~/Projects/ComfyUI`
3. Run `pip3 install -r requirements.txt`

## Step 2: Download Stable Diffusion Models

ComfyUI needs at least one checkpoint model. Folio is setup for **Flux.1-dev** by default for photorealistic images.

Flux.1-dev requires a free Hugging Face account and access token.

**First, get a Hugging Face token:**
1. Create account at https://huggingface.co/join (if you don't have one)
2. Go to https://huggingface.co/settings/tokens
3. Click "New token" → Name it "comfyui" → Select "Read" role → Create
4. Copy the token (starts with `hf_...`)

**Then download the model:**

```bash
cd ~/opt/ComfyUI/models/checkpoints/

# Method 1: Using wget with token
# Replace YOUR_HF_TOKEN with your actual token
wget --header="Authorization: Bearer YOUR_HF_TOKEN" \
  https://huggingface.co/Comfy-Org/flux1-dev-fp8/resolve/main/flux1-dev-fp8.safetensors

# Method 2: Using huggingface-cli (recommended)
# Install if needed: pip3 install huggingface_hub
huggingface-cli login  # Enter your token when prompted
huggingface-cli download Comfy-Org/flux1-dev flux1-dev-fp8.safetensors \
  --local-dir . --local-dir-use-symlinks False
```

**Size**: ~11GB (may take a few minutes depending on connection)

## Step 3: Start ComfyUI Server

```bash
cd ~/opt/ComfyUI
python main.py --listen 127.0.0.1 --port 8188
```

**Expected output: (something like...)**
```
Checkpoint files will always be loaded safely.
Total VRAM 36864 MB, total RAM 36864 MB
pytorch version: 2.10.0.dev20251116
Mac Version (15, 6)
Set vram state to: SHARED
Device: mps
Using sub quadratic optimization for attention, if you have memory or speed issues try using: --use-split-cross-attention
Python version: 3.12.9 (main, Feb 17 2025, 18:51:23) [Clang 15.0.0 (clang-1500.3.9.4)]
ComfyUI version: 0.3.68
****** User settings have been changed to be stored on the server instead of browser storage. ******
****** For multi-user setups add the --multi-user CLI argument to enable multiple user profiles. ******
ComfyUI frontend version: 1.28.8
[Prompt Server] web root: /Users/username/.pyenv/versions/3.12.9/lib/python3.12/site-packages/comfyui_frontend_package/static

Import times for custom nodes:
   0.0 seconds: /Users/username/opt/ComfyUI/custom_nodes/websocket_image_save.py

Context impl SQLiteImpl.
Will assume non-transactional DDL.
No target revision found.
Starting server

To see the GUI go to: http://127.0.0.1:8188
```

**Verification:**
- Open http://127.0.0.1:8188 in browser
- You should see the ComfyUI web interface
- Leave this terminal running

## Step 4: Configure Folio to Use ComfyUI

Add the following to `.env`:

```bash
# Image generation backend: "openai" or "local"
IMAGE_BACKEND=local

# ComfyUI server URL (only used when IMAGE_BACKEND=local)
# Default: http://localhost:8188
LOCAL_SD_URL=http://localhost:8188

# OpenAI API key (only needed when IMAGE_BACKEND=openai)
# OPENAI_API_KEY=sk-your-key-here
```

**To switch between backends:**
- **Local ComfyUI**: Set `IMAGE_BACKEND=local`
- **OpenAI DALL-E 3**: Set `IMAGE_BACKEND=openai` and uncomment `OPENAI_API_KEY`

**Note**: `.env` is already in `.gitignore` so your API keys won't be committed to git.

## Step 5: Generate Example Data

With ComfyUI running in one terminal, open another terminal:

```bash
cd ~/path/to/folio # wherever it is

# Generate 10 test photos
cargo run --bin generate-examples -- --count 10 --force

# Generate full 100-photo dataset
cargo run --bin generate-examples -- --count 100 --force
```

**Expected output:**
```
Using local Stable Diffusion at http://localhost:8188
Generating 10 photos...
Note: Time depends on your local setup

Creating directory structure...
Generating Batch 1: 5 photos (14:00-16:30)
  [1/5] Generating DSC_0001.JPG...
  [2/5] Generating DSC_0002.JPG...
  ...

Generating Batch 2: 5 photos (18:00-21:00)
  [1/5] Generating DSC_0007.JPG...
  ...

Creating videos from photo sequences...
  Creating DSC_0051.MOV...
  Creating DSC_0102.MOV...

✓ Example data generated successfully!
```

## Performance Expectations

**M3 Max (36GB RAM):**
- **Image generation**: 30-60 seconds per image
- **10 photos**: ~5-10 minutes
- **100 photos**: ~50-100 minutes (~1.5 hours)
- **Video creation**: ~30 seconds for 2 videos

**Total time for full dataset**: ~2 hours

## Troubleshooting

### ComfyUI won't start

**Error**: `ModuleNotFoundError: No module named 'torch'`
**Fix**: Install PyTorch: `pip3 install torch torchvision`

**Error**: `CUDA not available`
**Fix**: This is normal on Mac. PyTorch will use MPS (Metal) automatically.

### Connection refused

**Error**: `Failed to connect to local SD at http://localhost:8188`
**Fix**:
1. Verify ComfyUI is running: `curl http://localhost:8188`
2. Check ComfyUI terminal for errors
3. Restart ComfyUI server

### Model not found

**Error**: `flux1-dev-fp8.safetensors not found`
**Fix**:
1. Check model downloaded: `ls ~/Projects/ComfyUI/models/checkpoints/`
2. Verify filename matches exactly (case-sensitive)
3. Download model again if corrupted

### Images are low quality / artifacts

**Fix**:
1. Increase steps in workflow (edit `generate-examples.rs`, change `steps: 20` to `steps: 30`)
2. Try different model (Flux.1-dev vs SDXL)
3. Adjust CFG scale (default: 7.0)

### Generation is slow (>2 minutes per image)

**Fix**:
1. Close other applications to free RAM
2. Use FP8 quantized model (smaller, faster)
3. Reduce steps to 15-20
4. Check Activity Monitor for memory pressure

## Advanced: Customizing the Workflow

The ComfyUI workflow is defined in `crates/folio-examples/src/bin/generate-examples.rs` in the `create_comfyui_workflow()` function.

**Key parameters to adjust:**
```rust
"seed": rand::random::<u32>() as i64,  // Random seed per image
"steps": 20,                           // More steps = higher quality, slower
"cfg": 7.0,                            // Classifier-free guidance (6-8 typical)
"sampler_name": "euler",               // Sampler algorithm
"scheduler": "normal",                 // Scheduler type
"width": 1024,                         // Canvas width
"height": 1024,                        // Canvas height
```

**To modify:**
1. Edit `crates/folio-examples/src/bin/generate-examples.rs`
2. Find `create_comfyui_workflow()` function
3. Adjust parameters as needed
4. Rebuild: `cargo build --bin generate-examples`

## Next Steps

After generating example data:
1. [Test ingestion workflow](../user-stories/001-backlog-ingestion.md)
2. Verify data structure: `ls -R example-data/`
3. Check data size: `du -sh example-data/`
4. Test with folio: `cargo run --bin folio -- ingest --source example-data/sd-card-thanksgiving/DCIM --dest example-data/archive`
