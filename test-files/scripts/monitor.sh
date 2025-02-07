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

# Run Python script for the calculated duration
python3 -c 'import time; [print(i) or time.sleep(1) for i in range(1, 1000)]' &
PYTHON_PID=$!

# Sleep for the Python duration
sleep "$python_duration"

# Kill the Python process
kill $PYTHON_PID

# Run top for the remaining duration
timeout "$top_duration"s top
