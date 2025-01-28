# Tracer Daemon Instructions

## How to check if Tracer Daemon Is Running:

```bash
$ ps -e | grep tracer
```

---

## Running S3 Integration

This section outlines the requirements and setup necessary to use the S3 integration effectively. The S3 client supports flexible credential loading mechanisms.

### **Requirements**

1. **AWS Credentials**
   - Ensure your AWS credentials are available in one of the following locations:
     - `~/.aws/credentials` file with the appropriate profiles.
     - Environment variables (`AWS_ACCESS_KEY_ID`, `AWS_SECRET_ACCESS_KEY`).

2. **IAM Role (Optional)**
   - If running within an AWS environment (e.g., EC2, Lambda), you can use an IAM role to assume credentials automatically.

### **Initialization**

The S3 client initializes with the following options:
- **`profile`**: Load credentials from a named profile in the `~/.aws/credentials` file.
- **`role_arn`**: Assume an IAM role to obtain temporary credentials.
- **Fallback**: Automatically loads credentials from env when neither `profile` nor `role_arn` is provided.

#### Credential Sources

1. **Profile Name (`profile`)**:
   - Set up a profile in your `~/.aws/credentials` file.
   - Example:
     ```ini
     [my-profile]
     aws_access_key_id = YOUR_ACCESS_KEY_ID
     aws_secret_access_key = YOUR_SECRET_ACCESS_KEY
     ```
   - Pass the profile name as an argument to `new`.

2. **Assume Role (`role_arn`)**:
   - Provide a valid `role_arn` to assume an IAM role and retrieve temporary credentials. E.g: `"arn:aws:iam::123456789012:role/MyRole"`

3. **Default Credentials**:
   - If no `profile` or `role_arn` is provided, credentials are loaded automatically based on the default AWS configuration.

### **Docker Integration**

1. **Why use Docker with LocalStack**
   - To test the S3 integration, the client uses LocalStack, which is set up using Docker.
   - The Docker Compose file is located in the root of the repo.

2. **How to run Docker with LocalStack**
   - Ensure LocalStack is installed and running. You can start it using Docker:
     ```bash
     docker run -d -p 4566:4566 -p 4571:4571 localstack/localstack
     ```

---

### Grafana Loki 
- You need to start Grafana Loki sperately: docker-compose up -d loki
- Check if it is running: docker ps | grep loki  

### **Notes**

1. **Credential Resolution**
   - The function will panic if both `profile` and `role_arn` are provided.
   - It will also panic if no valid credentials are found during initialization.

2. **AWS Region**
   - Ensure the specified `region` matches the location of your S3 buckets.

---

## Development

### **Troubleshooting**

- If you encounter the error: `failed to run custom build command for 'openssl-sys v0.9.103'` or SSL-related issues, install the required dependencies:
  ```bash
  sudo apt install libssl-dev pkg-config
  ```

# Docker Container Registry

To speed up our CI pipeline, we utilize a custom Docker container registry on GitHub, known as the GitHub Container Registry (GCHR). This allows us to efficiently manage and deploy our Docker images.

### Steps to Use the Docker Container Registry
1. **Build the docker file**
   ```bash
   docker build -t rust-ci-arm64 -f ci.Dockerfile .
   ```

2. **Tag the Docker Image**  
   Tag your Docker image with the appropriate repository name:
   ```bash
   docker tag rust-ci-arm64 ghcr.io/tracer-cloud/tracer-cloud:rust-ci-arm64
   ```
3. **Authenticate with the GitHub Container Registry**  
   Use your GitHub token to log in to the registry. This step is necessary for pushing images:
   ```bash
   echo $GITHUB_TOKEN | docker login ghcr.io -u Tracer-Cloud --password-stdin
   ```

4. **Push the Docker Image to the Registry**  
   Push the tagged image to the GitHub Container Registry:
   ```bash
   docker push ghcr.io/tracer-cloud/tracer-cloud:rust-ci-arm64
   ```


5. **Repeat Tagging and Pushing**  
   If you need to tag and push the image again, you can repeat the tagging and pushing steps:
   ```bash
   docker tag rust-ci-arm64 ghcr.io/tracer-cloud/tracer-cloud:rust-ci-arm64
   docker push ghcr.io/tracer-cloud/tracer-cloud:rust-ci-arm64
   ```

### Note
Ensure that your GitHub token has the necessary permissions to access the GitHub Container Registry.

