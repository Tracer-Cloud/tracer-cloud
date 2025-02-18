provider "aws" {
  region  = "us-east-1"  # Change as needed
  profile = "default"
}

# IAM Role for EC2 Instance Connect
resource "aws_iam_role" "ec2_instance_connect" {
  name = "EC2InstanceConnectRole"

  assume_role_policy = jsonencode({
    Version = "2012-10-17"
    Statement = [{
      Effect    = "Allow"
      Principal = { Service = "ec2.amazonaws.com" }
      Action    = "sts:AssumeRole"
    }]
  })
}

# Create IAM policy for EC2 Instance Connect
resource "aws_iam_role_policy" "ec2_instance_connect" {
  name = "EC2InstanceConnectPolicy"
  role = aws_iam_role.ec2_instance_connect.id

  policy = jsonencode({
    Version = "2012-10-17"
    Statement = [
      {
        Effect = "Allow"
        Action = [
          "ec2-instance-connect:SendSSHPublicKey"
        ],
        Resource = "*"
      }
    ]
  })
}

# Create IAM instance profile
resource "aws_iam_instance_profile" "ec2_instance_connect" {
  name = "EC2InstanceConnectProfile"
  role = aws_iam_role.ec2_instance_connect.name
}

# Get the default VPC dynamically
data "aws_vpc" "default" {
  default = true
}

# Security Group for SSH & EC2 Instance Connect
resource "aws_security_group" "rust_server_sg" {
  name_prefix = "rust-sg-"
  description = "Allow SSH and EC2 Instance Connect"
  vpc_id      = data.aws_vpc.default.id

  # Allow SSH from any IP (Restrict in production)
  ingress {
    from_port   = 22
    to_port     = 22
    protocol    = "tcp"
    cidr_blocks = ["0.0.0.0/0"]
  }

  # Allow all outbound traffic
  egress {
    from_port   = 0
    to_port     = 0
    protocol    = "-1"
    cidr_blocks = ["0.0.0.0/0"]
  }

  tags = {
    Name = "rust-server-security-group"
  }
}

# EC2 Instance with SSH and EC2 Instance Connect
resource "aws_instance" "rust_server" {
  ## Do not change the ami because this one works for graviton 3 and is faster to start up
    ami           = "ami-06f77771310e204b7"  # Ubuntu 22.04 LTS (Adjust for your region)

  instance_type        = "c7g.large"
  key_name            = "rapid-ec2-v1"
  iam_instance_profile = aws_iam_instance_profile.ec2_instance_connect.name
  vpc_security_group_ids = [aws_security_group.rust_server_sg.id]

  metadata_options {
    http_tokens                 = "optional"
    http_put_response_hop_limit = 1
    http_endpoint              = "enabled"
  }

  root_block_device {
    volume_size = 50
    volume_type = "gp3"
  }

  tags = {
    Name = "Rust-EC2-Instance"
  }

  user_data = <<-EOF
    #!/bin/bash
    sudo apt update -y
    sudo apt install -y curl git unzip build-essential pkg-config

    # Install dependencies
    sudo apt-get update
    sudo apt-get install -y \
    build-essential \
    pkg-config \
    libssl-dev \
    clang \
    cmake \
    gcc \
    g++ \
    zlib1g-dev \
    libclang-dev

    # Install Rust for ubuntu user
    su - ubuntu -c '
    curl --proto "=https" --tlsv1.2 -sSf https://sh.rustup.rs > /home/ubuntu/rustup-init.sh && \
    chmod +x /home/ubuntu/rustup-init.sh && \
    /home/ubuntu/rustup-init.sh -y && \
    echo "source \$HOME/.cargo/env" >> /home/ubuntu/.bashrc && \
    . /home/ubuntu/.cargo/env'

    # Install GitHub CLI
    type -p curl >/dev/null || sudo apt install curl -y
    curl -fsSL https://cli.github.com/packages/githubcli-archive-keyring.gpg | sudo dd of=/usr/share/keyrings/githubcli-archive-keyring.gpg
    sudo chmod go+r /usr/share/keyrings/githubcli-archive-keyring.gpg
    echo "deb [signed-by=/usr/share/keyrings/githubcli-archive-keyring.gpg] https://cli.github.com/packages stable main" | sudo tee /etc/apt/sources.list.d/github-cli.list > /dev/null
    sudo apt update -y
    sudo apt install gh -y

    # Add Rust to system-wide path for immediate use
    echo "export PATH=/home/ubuntu/.cargo/bin:\$PATH" | sudo tee /etc/profile.d/rust.sh
    sudo chmod +x /etc/profile.d/rust.sh

    # Verify installation (log to a file since user_data runs non-interactively)
    echo "Installation verification:" | sudo tee /home/ubuntu/install_log.txt
    sudo -u ubuntu bash -c 'source /home/ubuntu/.cargo/env && rustc --version' >> /home/ubuntu/install_log.txt
    sudo -u ubuntu bash -c 'source /home/ubuntu/.cargo/env && cargo --version' >> /home/ubuntu/install_log.txt
    git --version >> /home/ubuntu/install_log.txt
    gh --version >> /home/ubuntu/install_log.txt

    # Install the tracer github repository 
    git clone https://github.com/Tracer-Cloud/tracer-client.git
    cd tracer-client

    # Install nextest
    cargo install --locked cargo-nextest

    # Run a nextest test to verify the installation
    cargo run nextest 

    # Install the binary
    cargo build --release
    sudo cp target/release/tracer-client /usr/local/bin/

  EOF
}