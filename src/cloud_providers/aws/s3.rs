use std::str::FromStr;

use aws_config::BehaviorVersion;
use aws_sdk_s3::{
    config::ProvideCredentials,
    types::{BucketLocationConstraint, CreateBucketConfiguration},
};

pub struct S3Handler {
    client: aws_sdk_s3::Client,
    region: String,
}

#[allow(dead_code)]
impl S3Handler {
    pub async fn new(profile: Option<&str>, role_arn: Option<&str>, region: &'static str) -> Self {
        let config_loader = aws_config::defaults(BehaviorVersion::latest());
        let config = match (profile, role_arn) {
            (Some(_), Some(_)) => {
                panic!("Cannot set both profile and role_arn")
            }
            (Some(profile), None) => config_loader.profile_name(profile),
            (None, Some(arn)) => {
                let assumed_role_provider = aws_config::sts::AssumeRoleProvider::builder(arn)
                    .session_name("tracer-client-session")
                    .build()
                    .await;

                let assumed_credentials_provider = assumed_role_provider
                    .provide_credentials()
                    .await
                    .expect("Failed to get assumed session role");

                config_loader.credentials_provider(assumed_credentials_provider)
            }
            (None, None) => aws_config::from_env(),
        }
        .region(region)
        .load()
        .await;

        let credentials_provider = config.credentials_provider().unwrap();
        let _ = credentials_provider
            .provide_credentials()
            .await
            .expect("No Credentials Loaded");

        Self {
            client: aws_sdk_s3::Client::new(&config),
            region: region.to_string(),
        }
    }

    pub async fn list_buckets(&self) -> Vec<String> {
        let mut processed_buckets = Vec::new();
        let mut buckets = self.client.list_buckets().into_paginator().send();

        while let Some(Ok(output)) = buckets.next().await {
            for bucket in output.buckets() {
                processed_buckets.push(bucket.name().unwrap_or_default().to_string());
            }
        }
        println!("Buckets {:?}", processed_buckets);
        processed_buckets
    }

    pub async fn create_bucket(
        &self,
        bucket_name: &str,
        bucket_config: Option<CreateBucketConfiguration>,
    ) -> Result<(), String> {
        let bucket_config = bucket_config.unwrap_or(
            CreateBucketConfiguration::builder()
                .location_constraint(
                    BucketLocationConstraint::from_str(&self.region)
                        .map_err(|err| err.to_string())?,
                )
                .build(),
        );

        if let Err(create_err) = self
            .client
            .create_bucket()
            .create_bucket_configuration(bucket_config)
            .bucket(bucket_name)
            .send()
            .await
        {
            if create_err
                .as_service_error()
                .map(|se| se.is_bucket_already_exists() || se.is_bucket_already_owned_by_you())
                == Some(true)
            {
                return Ok(());
            } else {
                return Err(format!("{:?}", create_err));
            }
        }
        Ok(())
    }

    pub async fn remove_object(&self, bucket_name: &str, key: &str) -> Result<(), String> {
        if let Err(err) = self
            .client
            .delete_object()
            .bucket(bucket_name)
            .key(key)
            .send()
            .await
        {
            return Err(err.to_string());
        }
        Ok(())
    }

    pub async fn delete_objects(
        &self,
        bucket_name: &str,
        objects_to_delete: Vec<&str>,
    ) -> Result<(), String> {
        let mut delete_object_ids = vec![];

        for key in objects_to_delete {
            let obj_id = aws_sdk_s3::types::ObjectIdentifier::builder()
                .key(key)
                .build()
                .map_err(|err| err.to_string())?;
            delete_object_ids.push(obj_id);
        }

        if let Err(err) = self
            .client
            .delete_objects()
            .bucket(bucket_name)
            .delete(
                aws_sdk_s3::types::Delete::builder()
                    .set_objects(Some(delete_object_ids))
                    .build()
                    .map_err(|err| err.to_string())?,
            )
            .send()
            .await
        {
            return Err(err.to_string());
        }
        Ok(())
    }

    pub async fn delete_bucket(&self, bucket_name: &str) -> Result<(), String> {
        if let Err(err) = self.client.delete_bucket().bucket(bucket_name).send().await {
            return Err(err.to_string());
        }
        Ok(())
    }

    pub async fn put_object(
        &self,
        bucket_name: &str,
        file_path: &str,
        key: &str,
    ) -> Result<(), String> {
        let body = aws_sdk_s3::primitives::ByteStream::from_path(std::path::Path::new(file_path))
            .await
            .map_err(|err| err.to_string())?;

        if let Err(err) = self
            .client
            .put_object()
            .bucket(bucket_name)
            .key(key)
            .body(body)
            .send()
            .await
        {
            return Err(format!("{err:?}"));
        }

        Ok(())
    }
}
