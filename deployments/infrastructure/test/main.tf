data "aws_vpc" "default" {
  default = true
}

variable "region" {
  description = "The AWS region to deploy resources"
  default     = "us-east-1"
}

variable "api_key" {
  description = "API key for Tracer service"
  type        = string
  sensitive   = true # This prevents it from being logged in Terraform outputs
  default     = "your-secret-api-key"
}

provider "aws" {
  region  = var.region
  profile = "default"
}


variable "perm_key" {
  description = "Permission Key for accessing the instance"
  type        = string
  default     = "tracer-from-ami"
}
