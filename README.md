# Tracer Daemon Instructions

## How to check if Tracer Daemon Is Running:

```bash
$ps -e | grep tracer
```


### Running S3 Integration

This section outlines the requirements and setup necessary to use the S3 integration effectively. The S3 client supports flexible credential loading mechanisms.

---

### **Requirements**

1. **AWS Credentials**
   - Ensure your AWS credentials are available in one of the following locations:
     - `~/.aws/credentials` file with the appropriate profiles.
     - Environment variables (`AWS_ACCESS_KEY_ID`, `AWS_SECRET_ACCESS_KEY`).

2. **IAM Role (Optional)**
   - If running within an AWS environment (e.g., EC2, Lambda), you can use an IAM role to assume credentials automatically.


---

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

#### Docker
1. **why you need to run docker with localstack**
- To test the S3 integration the client uses localstack which is setup by docker. 
- The docker compose file is located in the root of the repo. 

2. **how to run docker with localstack**
- ensure LocalStack is installed and running. You can start it using Docker:
   ```bash
   docker run -d -p 4566:4566 -p 4571:4571 localstack/localstack
   ```

---

### **Notes**
1. **Credential Resolution**
   - The function will panic if both `profile` and `role_arn` are provided.
   - It will also panic if no valid credentials are found during initialization.

2. **AWS Region**
   - Ensure the specified `region` matches the location of your S3 buckets.

---

## Development


### Troubleshooting

-  error: failed to run custom build command for `openssl-sys v0.9.103` or ssl related issues
install libssl-dev & pkg_config if not installed
	```bash
	sudo apt install libssl-dev pkg-config 
	```



# Changes made Jan 24th 2025
- Switch from us-east-2 to us-east-1 because this is the default region used by most people, and it can cause confusion. 
- Switch to "default" profile in aws config if "me" profile is not found. 

