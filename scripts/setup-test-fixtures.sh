#!/bin/bash
# Setup minimal test fixtures for Folio unit tests
# These fixtures are safe to commit to git (small, synthetic, no real family photos)

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
FIXTURES_DIR="$PROJECT_ROOT/test-data/fixtures"

echo "Setting up test fixtures in $FIXTURES_DIR..."

# Create directory
mkdir -p "$FIXTURES_DIR"

# Check for required tools
# Try 'magick' first (ImageMagick 7+), then fall back to 'convert' (ImageMagick 6)
if command -v magick >/dev/null 2>&1; then
    CONVERT_CMD="magick"
elif command -v convert >/dev/null 2>&1; then
    CONVERT_CMD="convert"
else
    echo "⚠️  ImageMagick not found. Install with: brew install imagemagick"
    echo "Skipping fixture generation. Tests may fail."
    exit 1
fi

command -v exiftool >/dev/null 2>&1 || {
    echo "⚠️  exiftool not found. Install with: brew install exiftool"
    echo "Skipping EXIF generation. Some tests may fail."
    exit 1
}

# 1. Minimal 1x1 JPEG (white pixel) - ~600 bytes
echo "Creating minimal.jpg (1x1 white pixel)..."
$CONVERT_CMD -size 1x1 xc:white "$FIXTURES_DIR/minimal.jpg"

# 2. Small JPEG with EXIF data - for timestamp/metadata tests
echo "Creating sample-with-exif.jpg (100x100 with D800 EXIF)..."
$CONVERT_CMD -size 100x100 xc:blue "$FIXTURES_DIR/sample-with-exif.jpg"
exiftool -overwrite_original \
    -DateTimeOriginal="2024:11:04 14:02:15" \
    -Make="Nikon" \
    -Model="D800" \
    -LensModel="AF-S NIKKOR 24-70mm f/2.8G ED" \
    -FNumber=5.6 \
    -ExposureTime="1/200" \
    -ISO=400 \
    -GPSLatitude=40.7128 \
    -GPSLongitude=-74.0060 \
    "$FIXTURES_DIR/sample-with-exif.jpg"

# 3. JPEG without EXIF - for fallback timestamp tests
echo "Creating no-exif.jpg (50x50, no EXIF)..."
$CONVERT_CMD -size 50x50 xc:red "$FIXTURES_DIR/no-exif.jpg"

# 4. Another JPEG with different timestamp - for temporal batching tests
echo "Creating sample-different-time.jpg (100x100, 4 hours later)..."
$CONVERT_CMD -size 100x100 xc:green "$FIXTURES_DIR/sample-different-time.jpg"
exiftool -overwrite_original \
    -DateTimeOriginal="2024:11:04 18:15:30" \
    -Make="Nikon" \
    -Model="D800" \
    "$FIXTURES_DIR/sample-different-time.jpg"

# 5. Corrupted JPEG - for error handling tests
echo "Creating corrupted.jpg (truncated JPEG)..."
head -c 100 "$FIXTURES_DIR/minimal.jpg" > "$FIXTURES_DIR/corrupted.jpg"

# 6. Non-media file - for filtering tests
echo "Creating test.txt (should be ignored by scanner)..."
echo "This is not a media file" > "$FIXTURES_DIR/test.txt"

# Video fixtures - only if ffmpeg is available
if command -v ffmpeg >/dev/null 2>&1; then
    echo "Creating video fixtures with ffmpeg..."

    # 7. Minimal 1-second MOV
    echo "Creating minimal.mov (1 second, 320x240)..."
    ffmpeg -f lavfi -i color=c=blue:s=320x240:d=1 \
           -f lavfi -i sine=frequency=1000:duration=1 \
           -y "$FIXTURES_DIR/minimal.mov" 2>/dev/null

    # 8. Another MOV with different characteristics
    echo "Creating minimal2.mov (1 second, different color)..."
    ffmpeg -f lavfi -i color=c=green:s=320x240:d=1 \
           -f lavfi -i sine=frequency=1500:duration=1 \
           -y "$FIXTURES_DIR/minimal2.mov" 2>/dev/null

    # 9. MP4 video
    echo "Creating minimal.mp4 (1 second)..."
    ffmpeg -f lavfi -i color=c=red:s=320x240:d=1 \
           -f lavfi -i sine=frequency=800:duration=1 \
           -y "$FIXTURES_DIR/minimal.mp4" 2>/dev/null
else
    echo "⚠️  ffmpeg not found. Skipping video fixture generation."
    echo "Install with: brew install ffmpeg"
    echo "Video tests may fail without these fixtures."
fi

echo ""
echo "✓ Test fixtures created successfully!"
echo ""
echo "Fixtures in $FIXTURES_DIR:"
ls -lh "$FIXTURES_DIR"
echo ""
echo "Total size:"
du -sh "$FIXTURES_DIR"
echo ""
echo "These fixtures are safe to commit to git."
echo "Run 'cargo test' to verify tests pass."
