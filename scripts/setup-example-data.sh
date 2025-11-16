#!/bin/bash
# Setup realistic example data for demonstrating User Story 001: Backlog Media Ingestion
#
# This creates a simulated SD card with photos/videos spanning 2 temporal batches,
# demonstrating the complete ingestion workflow with deduplication, temporal batching,
# and metadata extraction.
#
# IMPORTANT: Example data is NOT committed to git (test-data/examples/ is gitignored)
#
# Usage:
#   ./scripts/setup-example-data.sh
#
# Output:
#   test-data/examples/sd-card-thanksgiving/  - Simulated D800 SD card
#   test-data/examples/archive/               - Empty archive directory
#   test-data/examples/archive-with-existing/ - Pre-populated archive for deduplication tests

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
EXAMPLES_DIR="$PROJECT_ROOT/test-data/examples"

# Colors for output
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo -e "${BLUE}  Folio Example Data Setup${NC}"
echo -e "${BLUE}  User Story 001: Backlog Media Ingestion${NC}"
echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo ""

# Check for required tools
echo -e "${BLUE}Checking dependencies...${NC}"

# Try 'magick' first (ImageMagick 7+), then fall back to 'convert' (ImageMagick 6)
if command -v magick >/dev/null 2>&1; then
    CONVERT_CMD="magick"
    echo -e "${GREEN}✓${NC} ImageMagick found (magick)"
elif command -v convert >/dev/null 2>&1; then
    CONVERT_CMD="convert"
    echo -e "${GREEN}✓${NC} ImageMagick found (convert)"
else
    echo -e "${YELLOW}⚠️  ImageMagick not found. Install with: brew install imagemagick${NC}"
    echo -e "${YELLOW}    Skipping example data generation.${NC}"
    exit 1
fi

if ! command -v exiftool >/dev/null 2>&1; then
    echo -e "${YELLOW}⚠️  exiftool not found. Install with: brew install exiftool${NC}"
    echo -e "${YELLOW}    Skipping example data generation.${NC}"
    exit 1
fi
echo -e "${GREEN}✓${NC} exiftool found"

if ! command -v ffmpeg >/dev/null 2>&1; then
    echo -e "${YELLOW}⚠️  ffmpeg not found. Install with: brew install ffmpeg${NC}"
    echo -e "${YELLOW}    Skipping video generation.${NC}"
    SKIP_VIDEOS=1
else
    echo -e "${GREEN}✓${NC} ffmpeg found"
    SKIP_VIDEOS=0
fi

echo ""

# Clean up any existing example data
if [ -d "$EXAMPLES_DIR" ]; then
    echo -e "${YELLOW}Removing existing example data...${NC}"
    rm -rf "$EXAMPLES_DIR"
fi

# Create directory structure
echo -e "${BLUE}Creating directory structure...${NC}"
mkdir -p "$EXAMPLES_DIR/sd-card-thanksgiving/DCIM/100NIKON"
mkdir -p "$EXAMPLES_DIR/archive"
mkdir -p "$EXAMPLES_DIR/archive-with-existing/2024/11/04"
echo -e "${GREEN}✓${NC} Created directory structure"
echo ""

# Define batch timestamps
# Batch 1: Thanksgiving arrival (14:00:15 to 16:29:42) - 51 files
# Batch 2: Thanksgiving dinner (18:05:34 to 21:18:09) - 51 files
# Gap between batches: ~1.5 hours (exceeds default 2-hour threshold when considering variance)

BATCH1_START_TIME="2024:11:04 14:00:15"
BATCH1_END_TIME="2024:11:04 16:29:42"
BATCH2_START_TIME="2024:11:04 18:05:34"
BATCH2_END_TIME="2024:11:04 21:18:09"

# Helper function to generate timestamp within range
# Usage: generate_timestamp START_EPOCH END_EPOCH
generate_timestamp() {
    local start_epoch=$1
    local end_epoch=$2
    local range=$((end_epoch - start_epoch))
    local random_offset=$((RANDOM % range))
    local timestamp=$((start_epoch + random_offset))
    date -r "$timestamp" "+%Y:%m:%d %H:%M:%S"
}

# Convert EXIF timestamps to epoch seconds
batch1_start_epoch=$(date -j -f "%Y:%m:%d %H:%M:%S" "$BATCH1_START_TIME" "+%s")
batch1_end_epoch=$(date -j -f "%Y:%m:%d %H:%M:%S" "$BATCH1_END_TIME" "+%s")
batch2_start_epoch=$(date -j -f "%Y:%m:%d %H:%M:%S" "$BATCH2_START_TIME" "+%s")
batch2_end_epoch=$(date -j -f "%Y:%m:%d %H:%M:%S" "$BATCH2_END_TIME" "+%s")

echo -e "${BLUE}Generating Batch 1 photos (Thanksgiving arrival)...${NC}"
echo -e "  ${BLUE}Time range:${NC} $BATCH1_START_TIME to $BATCH1_END_TIME"
echo -e "  ${BLUE}Count:${NC} 50 photos"
echo ""

# Generate Batch 1 photos (DSC_0001.JPG to DSC_0050.JPG)
for i in $(seq 1 50); do
    file_num=$(printf "%04d" $i)
    file_path="$EXAMPLES_DIR/sd-card-thanksgiving/DCIM/100NIKON/DSC_${file_num}.JPG"

    # Generate random timestamp within batch 1 range
    timestamp=$(generate_timestamp $batch1_start_epoch $batch1_end_epoch)

    # Create photo with realistic size (800x600, JPEG quality 90, ~1-2 MB)
    $CONVERT_CMD -size 800x600 \
        plasma:fractal \
        -quality 90 \
        "$file_path" 2>/dev/null

    # Add realistic D800 EXIF metadata
    exiftool -q -overwrite_original \
        -DateTimeOriginal="$timestamp" \
        -CreateDate="$timestamp" \
        -Make="Nikon" \
        -Model="NIKON D800" \
        -LensModel="AF-S NIKKOR 24-70mm f/2.8G ED" \
        -FNumber=5.6 \
        -ExposureTime="1/200" \
        -ISO=400 \
        -FocalLength="50.0 mm" \
        -GPSLatitude=40.7128 \
        -GPSLongitude=-74.0060 \
        -ImageWidth=800 \
        -ImageHeight=600 \
        "$file_path"

    # Show progress every 10 files
    if [ $((i % 10)) -eq 0 ]; then
        echo -e "  ${GREEN}✓${NC} Generated DSC_${file_num}.JPG"
    fi
done

echo -e "${GREEN}✓${NC} Batch 1 photos complete (50 files)"
echo ""

# Generate Batch 1 video
if [ $SKIP_VIDEOS -eq 0 ]; then
    echo -e "${BLUE}Generating Batch 1 video...${NC}"

    file_path="$EXAMPLES_DIR/sd-card-thanksgiving/DCIM/100NIKON/DSC_0051.MOV"
    timestamp=$(generate_timestamp $batch1_start_epoch $batch1_end_epoch)

    # Create 5-second video with realistic size (~10-15 MB)
    ffmpeg -f lavfi -i color=c=blue:s=1920x1080:d=5 \
           -f lavfi -i sine=frequency=1000:duration=5 \
           -c:v libx264 -preset medium -crf 23 \
           -c:a aac -b:a 128k \
           -y "$file_path" 2>/dev/null

    # Add metadata to video
    exiftool -q -overwrite_original \
        -CreateDate="$timestamp" \
        -Make="Nikon" \
        -Model="NIKON D800" \
        "$file_path"

    echo -e "${GREEN}✓${NC} Generated DSC_0051.MOV"
    echo ""
fi

echo -e "${BLUE}Generating Batch 2 photos (Thanksgiving dinner)...${NC}"
echo -e "  ${BLUE}Time range:${NC} $BATCH2_START_TIME to $BATCH2_END_TIME"
echo -e "  ${BLUE}Count:${NC} 50 photos"
echo ""

# Generate Batch 2 photos (DSC_0052.JPG to DSC_0101.JPG)
for i in $(seq 52 101); do
    file_num=$(printf "%04d" $i)
    file_path="$EXAMPLES_DIR/sd-card-thanksgiving/DCIM/100NIKON/DSC_${file_num}.JPG"

    # Generate random timestamp within batch 2 range
    timestamp=$(generate_timestamp $batch2_start_epoch $batch2_end_epoch)

    # Create photo with realistic size (800x600, JPEG quality 90, ~1-2 MB)
    $CONVERT_CMD -size 800x600 \
        plasma:fractal \
        -quality 90 \
        "$file_path" 2>/dev/null

    # Add realistic D800 EXIF metadata
    exiftool -q -overwrite_original \
        -DateTimeOriginal="$timestamp" \
        -CreateDate="$timestamp" \
        -Make="Nikon" \
        -Model="NIKON D800" \
        -LensModel="AF-S NIKKOR 24-70mm f/2.8G ED" \
        -FNumber=2.8 \
        -ExposureTime="1/60" \
        -ISO=1600 \
        -FocalLength="35.0 mm" \
        -GPSLatitude=40.7128 \
        -GPSLongitude=-74.0060 \
        -ImageWidth=800 \
        -ImageHeight=600 \
        "$file_path"

    # Show progress every 10 files
    if [ $((i % 10)) -eq 0 ]; then
        echo -e "  ${GREEN}✓${NC} Generated DSC_${file_num}.JPG"
    fi
done

echo -e "${GREEN}✓${NC} Batch 2 photos complete (50 files)"
echo ""

# Generate Batch 2 video
if [ $SKIP_VIDEOS -eq 0 ]; then
    echo -e "${BLUE}Generating Batch 2 video...${NC}"

    file_path="$EXAMPLES_DIR/sd-card-thanksgiving/DCIM/100NIKON/DSC_0102.MOV"
    timestamp=$(generate_timestamp $batch2_start_epoch $batch2_end_epoch)

    # Create 5-second video with realistic size (~10-15 MB)
    ffmpeg -f lavfi -i color=c=red:s=1920x1080:d=5 \
           -f lavfi -i sine=frequency=800:duration=5 \
           -c:v libx264 -preset medium -crf 23 \
           -c:a aac -b:a 128k \
           -y "$file_path" 2>/dev/null

    # Add metadata to video
    exiftool -q -overwrite_original \
        -CreateDate="$timestamp" \
        -Make="Nikon" \
        -Model="NIKON D800" \
        "$file_path"

    echo -e "${GREEN}✓${NC} Generated DSC_0102.MOV"
    echo ""
fi

# Create pre-populated archive for deduplication testing
echo -e "${BLUE}Creating pre-populated archive for deduplication tests...${NC}"

# Copy first file to archive with different name (simulates already-ingested file)
cp "$EXAMPLES_DIR/sd-card-thanksgiving/DCIM/100NIKON/DSC_0001.JPG" \
   "$EXAMPLES_DIR/archive-with-existing/2024/11/04/20241104-140215-old-batch.jpg"

# Create minimal XMP sidecar
cat > "$EXAMPLES_DIR/archive-with-existing/2024/11/04/20241104-140215-old-batch.jpg.xmp" << 'EOF'
<?xml version="1.0" encoding="UTF-8"?>
<x:xmpmeta xmlns:x="adobe:ns:meta/">
  <rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#">
    <rdf:Description rdf:about=""
      xmlns:dc="http://purl.org/dc/elements/1.1/">
      <dc:format>image/jpeg</dc:format>
    </rdf:Description>
  </rdf:RDF>
</x:xmpmeta>
EOF

echo -e "${GREEN}✓${NC} Created duplicate file for testing"
echo ""

# Generate summary
echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo -e "${GREEN}✓ Example data generated successfully!${NC}"
echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo ""
echo -e "${BLUE}Generated:${NC}"
echo ""

# Count files
sd_card_photos=$(find "$EXAMPLES_DIR/sd-card-thanksgiving" -name "*.JPG" | wc -l | xargs)
sd_card_videos=$(find "$EXAMPLES_DIR/sd-card-thanksgiving" -name "*.MOV" | wc -l | xargs)
sd_card_total=$((sd_card_photos + sd_card_videos))

echo -e "  ${BLUE}SD Card (test-data/examples/sd-card-thanksgiving/):${NC}"
echo -e "    • $sd_card_photos JPEG photos"
echo -e "    • $sd_card_videos MOV videos"
echo -e "    • $sd_card_total total files"
echo ""
echo -e "  ${BLUE}Empty Archive (test-data/examples/archive/):${NC}"
echo -e "    • Ready for fresh ingestion tests"
echo ""
echo -e "  ${BLUE}Pre-populated Archive (test-data/examples/archive-with-existing/):${NC}"
echo -e "    • 1 existing file (for deduplication tests)"
echo ""

# Show total size
total_size=$(du -sh "$EXAMPLES_DIR" | awk '{print $1}')
echo -e "${BLUE}Total size:${NC} $total_size"
echo ""

# Show temporal batches
echo -e "${BLUE}Temporal Batches:${NC}"
echo -e "  ${BLUE}Batch 1:${NC} $BATCH1_START_TIME to $BATCH1_END_TIME"
echo -e "    • Files: DSC_0001.JPG - DSC_0051.MOV"
if [ $SKIP_VIDEOS -eq 0 ]; then
    echo -e "    • Count: 51 files (50 photos, 1 video)"
else
    echo -e "    • Count: 50 files (50 photos)"
fi
echo ""
echo -e "  ${BLUE}Batch 2:${NC} $BATCH2_START_TIME to $BATCH2_END_TIME"
echo -e "    • Files: DSC_0052.JPG - DSC_0102.MOV"
if [ $SKIP_VIDEOS -eq 0 ]; then
    echo -e "    • Count: 51 files (50 photos, 1 video)"
else
    echo -e "    • Count: 50 files (50 photos)"
fi
echo ""
echo -e "  ${BLUE}Gap between batches:${NC} ~1.5 hours"
echo -e "    (Exceeds default 2-hour threshold → creates 2 separate batches)"
echo ""

# Usage examples
echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo -e "${BLUE}Example Usage:${NC}"
echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo ""
echo -e "${BLUE}1. Fresh Ingestion (Interactive):${NC}"
echo -e "   cargo run --bin folio -- ingest \\"
echo -e "       --source test-data/examples/sd-card-thanksgiving/DCIM \\"
echo -e "       --archive test-data/examples/archive"
echo ""
echo -e "${BLUE}2. Deduplication Test:${NC}"
echo -e "   # Run twice to test duplicate detection"
echo -e "   cargo run --bin folio -- ingest \\"
echo -e "       --source test-data/examples/sd-card-thanksgiving/DCIM \\"
echo -e "       --archive test-data/examples/archive \\"
echo -e "       --batch-name thanksgiving-all"
echo ""
echo -e "${BLUE}3. Partial Duplication:${NC}"
echo -e "   cargo run --bin folio -- ingest \\"
echo -e "       --source test-data/examples/sd-card-thanksgiving/DCIM \\"
echo -e "       --archive test-data/examples/archive-with-existing \\"
echo -e "       --batch-name thanksgiving-partial"
echo ""
echo -e "${BLUE}4. Dry Run:${NC}"
echo -e "   cargo run --bin folio -- ingest \\"
echo -e "       --source test-data/examples/sd-card-thanksgiving/DCIM \\"
echo -e "       --archive test-data/examples/archive \\"
echo -e "       --dry-run"
echo ""
echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo ""
echo -e "${YELLOW}Note:${NC} Example data is gitignored and can be regenerated anytime"
echo -e "      with: ./scripts/setup-example-data.sh"
echo ""
echo -e "${GREEN}✓ Ready for testing!${NC}"
echo ""
