#!/bin/bash

# Get latest Ubuntu AMI
UBUNTU_AMI=$(aws ssm get-parameters-by-path --path "/aws/service/canonical/ubuntu/server/22.04/stable" \
    --query "Parameters[?ends_with(Name, 'amd64/hvm/ebs-gp3/ami-id')].Value" --output text)

# Define parameters
INSTANCE_TYPE="t3.medium"
SECURITY_GROUP_ID="sg-0c945e66a325f3119"  # Replace with your security group ID
KEY_NAME="rapid-ec2-v1"  # Replace with the correct AWS Key Pair name

# Debugging Output
echo "Using Ubuntu AMI: $UBUNTU_AMI"
echo "Using Security Group: $SECURITY_GROUP_ID"
echo "Using Key Pair: $KEY_NAME"

# Check if key pair exists
KEY_CHECK=$(aws ec2 describe-key-pairs --key-names "$KEY_NAME" --query "KeyPairs[*].KeyName" --output text 2>/dev/null)
if [[ -z "$KEY_CHECK" ]]; then
    echo "❌ ERROR: Key Pair '$KEY_NAME' does not exist! Create it before launching an instance."
    exit 1
fi

# Launch the instance
INSTANCE_ID=$(aws ec2 run-instances \
    --image-id "$UBUNTU_AMI" \
    --count 1 \
    --instance-type "$INSTANCE_TYPE" \
    --key-name "$KEY_NAME" \
    --security-group-ids "$SECURITY_GROUP_ID" \
    --tag-specifications 'ResourceType=instance,Tags=[{Key=Name,Value=AMI-Build-Ubuntu}]' \
    --query "Instances[0].InstanceId" --output text)

echo "✅ Instance launched: $INSTANCE_ID"

# Get Public IP for SSH
PUBLIC_IP=$(aws ec2 describe-instances --instance-ids "$INSTANCE_ID" \
    --query "Reservations[0].Instances[0].PublicIpAddress" --output text)

echo "🚀 SSH into the instance using:"
echo "ssh -i ~/.ssh/ec2-test-instance-c5.pem ubuntu@$PUBLIC_IP"
