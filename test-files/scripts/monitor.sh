#!/bin/bash

# Check if arguments are provided
if [ "$#" -ne 2 ]; then
    echo "Usage: $0 <total_duration_seconds> <python_time_ratio>"
    exit 1
fi

total_duration=$1
python_ratio=$2

# Calculate Python and top durations
python_duration=$(echo "$total_duration * $python_ratio" | bc)
top_duration=$(echo "$total_duration - $python_duration" | bc)

# Run Python script that's more resilient to signals
python3 -c '
import time
import signal
import sys

def handler(signum, frame):
    sys.exit(0)

signal.signal(signal.SIGTERM, handler)

try:
    for i in range(1, 1000):
        print(i)
        time.sleep(1)
except KeyboardInterrupt:
    sys.exit(0)
' &

PYTHON_PID=$!

# Sleep for the Python duration
sleep "$python_duration"

# Kill the Python process gracefully
kill -TERM $PYTHON_PID 2>/dev/null

# Run top for the remaining duration
if command -v timeout >/dev/null 2>&1; then
    timeout "$top_duration"s top
else
    # Fallback if timeout is not available
    top -n "$top_duration"
fi
