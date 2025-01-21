use std::str::FromStr;

use aws_config::BehaviorVersion;
use aws_sdk_s3::{
    config::ProvideCredentials,
    types::{BucketLocationConstraint, CreateBucketConfiguration},
};

pub struct S3Client {
    client: aws_sdk_s3::Client,
    region: String,
}

#[allow(dead_code)]
impl S3Client {
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

    pub async fn new_with_s3_config(config: aws_sdk_s3::config::Config, region: &str) -> Self {
        Self {
            client: aws_sdk_s3::Client::from_conf(config),
            region: region.to_string(),
        }
    }

    pub async fn list_buckets(&self) -> Result<Vec<String>, String> {
        let mut processed_buckets = Vec::new();
        let response = self
            .client
            .list_buckets()
            .send()
            .await
            .map_err(|err| format!("{err:?}"))?;

        if let Some(buckets) = response.buckets {
            for bucket in buckets {
                processed_buckets.push(bucket.name().unwrap_or_default().to_string());
            }
        }
        println!("Buckets {:?}", processed_buckets);
        Ok(processed_buckets)
    }

    pub async fn list_buckets_paginated(&self) -> Vec<String> {
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
                        .map_err(|err| format!("Invalid region: {}", err))?,
                )
                .build(),
        );

        let result = self
            .client
            .create_bucket()
            .bucket(bucket_name)
            .create_bucket_configuration(bucket_config)
            .send()
            .await;

        match result {
            Ok(_) => Ok(()),
            Err(create_err) => {
                println!("Error creating bucket: {:?}", create_err);

                if let Some(service_error) = create_err.as_service_error() {
                    if service_error.is_bucket_already_exists()
                        || service_error.is_bucket_already_owned_by_you()
                    {
                        println!("Bucket already exists or owned by you, proceeding.");
                        return Ok(());
                    }
                }
                Err(format!("Failed to create bucket: {:?}", create_err))
            }
        }
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

#[cfg(test)]
mod tests {

    use super::*;
    use std::env;

    fn setup_env_vars(region: &str) {
        // Set S3 configuration
        env::set_var("S3_ENDPOINT_URL", "http://0.0.0.0:4566/");
        env::set_var("AWS_ACCESS_KEY_ID", "000000");
        env::set_var("AWS_SECRET_ACCESS_KEY", "000000000000");
        env::set_var("AWS_REGION", region);
        env::set_var("AWS_LOG_LEVEL", "debug");
    }

    #[tokio::test]
    async fn test_s3_actions() {
        // initialize
        let region = "us-east-2";
        setup_env_vars(region);
        let endpoint_url = std::env::var("S3_ENDPOINT_URL").unwrap();
        let config = aws_config::defaults(BehaviorVersion::latest())
            .region(region)
            .endpoint_url(endpoint_url.clone())
            .load()
            .await;

        let s3_config = aws_sdk_s3::config::Builder::from(&config)
            .force_path_style(true)
            .build();

        let s3_client = S3Client::new_with_s3_config(s3_config.clone(), &region).await;

        let test_bucket_name = "tracer-client-test";
        let location_constraint = BucketLocationConstraint::UsEast2;

        let bucket_config = CreateBucketConfiguration::builder()
            .location_constraint(location_constraint)
            .build();

        let file_path = "test-files/exports/test_run/bd01d5c9-8658-4a22-b059-3d504f346f8e.parquet";
        let key = "exports/test_run/bd01d5c9-8658-4a22-b059-3d504f346f8e.parquet";

        s3_client
            .create_bucket(test_bucket_name, Some(bucket_config))
            .await
            .expect("s3 handler failed");

        let buckets = s3_client.list_buckets().await.unwrap();
        assert_eq!(buckets.len(), 1);

        s3_client
            .put_object(test_bucket_name, file_path, key)
            .await
            .expect("Failed to put object");

        // list objects
        let objects = s3_client
            .client
            .list_objects()
            .bucket(test_bucket_name)
            .max_keys(1)
            .send()
            .await
            .expect("Failed to list objects")
            .contents
            .unwrap();

        let object = objects.first();

        assert_eq!(object.unwrap().key, Some(key.to_string()));

        s3_client
            .remove_object(test_bucket_name, key)
            .await
            .unwrap();

        // list objects after delete
        let objects = s3_client
            .client
            .list_objects()
            .bucket(test_bucket_name)
            .max_keys(1)
            .send()
            .await
            .expect("Failed to list objects")
            .contents;

        assert!(objects.is_none());

        s3_client.delete_bucket(test_bucket_name).await.unwrap();

        let buckets = s3_client.list_buckets().await.unwrap();

        assert!(buckets.is_empty());
    }

    #[tokio::test]
    async fn test_additional_s3_actions() {
        // Initialize
        let region = "us-east-2";
        setup_env_vars(region);
        let endpoint_url = std::env::var("S3_ENDPOINT_URL").unwrap();
        let config = aws_config::defaults(BehaviorVersion::latest())
            .region(region)
            .endpoint_url(endpoint_url.clone())
            .load()
            .await;

        let s3_config = aws_sdk_s3::config::Builder::from(&config)
            .force_path_style(true)
            .build();

        let s3_client = S3Client::new_with_s3_config(s3_config.clone(), &region).await;

        let test_bucket_name = "test-additional-actions-bucket";
        let key_1 = "exports/test_run/file1.parquet";
        let key_2 = "exports/test_run/file2.parquet";
        let file_path = "test-files/exports/test_run/bd01d5c9-8658-4a22-b059-3d504f346f8e.parquet";

        // Create bucket
        s3_client
            .create_bucket(test_bucket_name, None)
            .await
            .expect("Failed to create bucket");

        // Add multiple objects
        s3_client
            .put_object(test_bucket_name, file_path, key_1)
            .await
            .expect("Failed to put object 1");

        s3_client
            .put_object(test_bucket_name, file_path, key_2)
            .await
            .expect("Failed to put object 2");

        // List buckets (paginated)
        let buckets = s3_client.list_buckets_paginated().await;
        assert!(buckets.contains(&test_bucket_name.to_string()));

        // Delete multiple objects
        s3_client
            .delete_objects(test_bucket_name, vec![key_1, key_2])
            .await
            .expect("Failed to delete objects");

        // Verify objects deletion
        let objects_after_delete = s3_client
            .client
            .list_objects()
            .bucket(test_bucket_name)
            .send()
            .await
            .expect("Failed to list objects after delete")
            .contents;

        assert!(objects_after_delete.is_none());

        // Delete bucket
        s3_client
            .delete_bucket(test_bucket_name)
            .await
            .expect("Failed to delete bucket");

        // Verify bucket deletion
        let buckets = s3_client.list_buckets().await.unwrap();
        assert!(!buckets.contains(&test_bucket_name.to_string()));
    }
}
