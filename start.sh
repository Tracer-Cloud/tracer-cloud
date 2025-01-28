#!/bin/bash
set -e

# Start Localstack
echo "Starting Localstack..."
localstack start -d &

# Start Loki
echo "Starting Loki..."
loki -config.file=/etc/loki/local-config.yaml &

# Start Grafana
echo "Starting Grafana..."
/run.sh &

# Keep the container running
echo "All services started. Keeping the container alive..."
tail -f /dev/null
