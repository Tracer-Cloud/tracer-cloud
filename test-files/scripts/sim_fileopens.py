import sys
import time
import os

if len(sys.argv) < 2:
    print("Usage: python3 sim_fileopens.py <dataset_file>")
    sys.exit(1)

file_path = sys.argv[1]

if not os.path.exists(file_path):
    print(f"File not found: {file_path}")
    sys.exit(1)

print(f"Processing dataset: {file_path}")

with open(file_path, "r+", encoding="utf8") as f:
    time.sleep(2)  # Simulate processing time

print(f"Finished processing: {file_path}")
