# Use Ubuntu 24.04 for ARM64 compatibility
FROM ubuntu:24.04

# Install basic dependencies for Rust, Localstack, Loki, and Python virtual environments
RUN apt-get update && apt-get install -y \
    curl \
    wget \
    build-essential \
    python3-pip \
    python3-venv \
    jq \
    unzip \
    && apt-get clean

# Install Rust and cargo-nextest
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
ENV PATH="/root/.cargo/bin:${PATH}"
RUN cargo install cargo-nextest

# Set up a Python virtual environment and install Localstack
RUN python3 -m venv /venv \
    && /venv/bin/pip install --upgrade pip \
    && /venv/bin/pip install localstack
ENV PATH="/venv/bin:${PATH}"

# Install Loki binary
RUN wget https://github.com/grafana/loki/releases/download/v3.3.0/loki-linux-arm64.zip \
    && unzip loki-linux-arm64.zip -d /usr/bin/ \
    && rm loki-linux-arm64.zip

# Configure Grafana datasource for Loki
RUN mkdir -p /etc/grafana/provisioning/datasources
RUN echo "\
apiVersion: 1\n\
datasources:\n\
- name: Loki\n\
  type: loki\n\
  access: proxy\n\
  orgId: 1\n\
  url: http://localhost:3100\n\
  basicAuth: false\n\
  isDefault: true\n\
  version: 1\n\
  editable: false" > /etc/grafana/provisioning/datasources/ds.yaml

# Set Grafana environment variables
ENV GF_PATHS_PROVISIONING="/etc/grafana/provisioning" \
    GF_AUTH_ANONYMOUS_ENABLED="true" \
    GF_AUTH_ANONYMOUS_ORG_ROLE="Admin" \
    GF_FEATURE_TOGGLES_ENABLE="alertingSimplifiedRouting,alertingQueryAndExpressionsStepMode"

# Expose necessary ports
EXPOSE 4566 3100 3000

# Copy entrypoint script
COPY start.sh /start.sh
RUN chmod +x /start.sh

# Entrypoint to start services
ENTRYPOINT ["/start.sh"]
