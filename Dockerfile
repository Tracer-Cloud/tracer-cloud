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
    libssl-dev \
    pkg-config \
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

EXPOSE 4566
