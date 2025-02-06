#!/bin/bash

# Check if Python is available
if ! command -v python3 &> /dev/null; then
    echo "Python3 is required but not installed. Exiting."
    exit 1
fi

# Get script directory
SCRIPT_DIR="$(dirname "$(realpath "$0")")"

# Dataset directory
DATASET_DIR="$SCRIPT_DIR/../test-data-samples"

# Ensure the dataset directory exists
mkdir -p "$DATASET_DIR"

# Generate test .fa files
for i in {1..3}; do
    echo -e ">test_sequence\nAGCTTAGCTA" > "$DATASET_DIR/test$i.fa"
done

echo "Generated test datasets: test1.fa, test2.fa, test3.fa"

# Process each dataset using Python script
for file in "$DATASET_DIR"/*.fa; do
    python3 "$SCRIPT_DIR/sim_fileopens.py" "$file"
done

# Cleanup after processing
rm -rf "$DATASET_DIR"/
echo "Dataset processing completed."