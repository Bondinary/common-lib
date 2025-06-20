use hex::encode;
use rand::Rng;
use rocket::tokio::io::AsyncReadExt;
use rusoto_core::Region;
use rusoto_s3::{GetObjectRequest, S3Client, S3};
use tracing::debug;
use std::{collections::HashMap, env};

use crate::constants::{ENV, KSA, LOCAL_ENV, UK};

pub fn generate_random_token() -> String {
    let mut rng = rand::thread_rng();
    let random_number = rng.gen_range(111111..=999999);
    random_number.to_string()
}

pub fn generate_random_alphanumeric_string() -> String {
    let mut rng = rand::thread_rng();
    let random_key_bytes: [u8; 32] = rng.r#gen();

    // Convert the byte array to a hexadecimal string
    encode(&random_key_bytes)
}

pub fn get_env_var(name: &str, default: Option<&str>) -> String {
    match env::var(name) {
        Ok(val) => val,
        Err(_) => {
            debug!("Error loading {} env variable", name);
            match default {
                Some(val) => val.to_string(),
                None => String::new(),
            }
        }
    }
}

pub fn is_local_env() -> bool {
    let env = get_env_var(ENV, None);
    env == LOCAL_ENV
}

pub async fn download_file_from_s3(
    bucket_name: &str,
    object_key: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    // Create an S3 client
    let region = Region::EuWest2;
    let s3_client = S3Client::new(region);

    // Create request to get object from S3
    let request = GetObjectRequest {
        bucket: bucket_name.to_string(),
        key: object_key.to_string(),
        ..Default::default()
    };

    // Send request to S3
    let response = s3_client.get_object(request).await?;
    let body = response.body.unwrap();
    debug!("Received this from S3 {:?}", body);

    // Read the body (file contents) into a Vec<u8>
    let mut bytes: Vec<u8> = Vec::new();
    body.into_async_read().read_to_end(&mut bytes).await?;

    // Convert bytes to a UTF-8 encoded string
    let content = String::from_utf8(bytes)?;
    Ok(content)
}

pub async fn get_secret_value(secret_name: &str) -> Result<String, Box<dyn std::error::Error>> {
    let config = aws_config::load_from_env().await;
    let secret_manager = aws_sdk_secretsmanager::Client::new(&config);

    let secret = match secret_manager
        .get_secret_value()
        .secret_id(secret_name)
        .send()
        .await
    {
        Ok(s) => {
            if let Some(secret) = s.secret_string {
                secret
            } else {
                debug!("No secret for {secret_name}");
                return Err("No secret found for {secret_name}".into());
            }
        }
        Err(err) => {
            debug!("Error trying to get connection secret for {secret_name}: {err:?}");
            return Err(Box::new(err))
        }
    };

    // For a list of exceptions thrown, see
    // https://docs.aws.amazon.com/secretsmanager/latest/apireference/API_GetSecretValue.html

    Ok(secret)
}

pub fn get_country_code_from_phone_number(phone_number: &str) -> String {
    // Simple static map of country code prefixes to country names
    let country_codes: HashMap<&str, &str> = [
        ("44", UK),      // United Kingdom
        ("966", KSA),      // United Kingdom
    ]
    .iter()
    .cloned()
    .collect();

    // Remove the leading "+" sign and parse the country code prefix
    let clean_number = phone_number.trim_start_matches('+'); // Remove leading '+'
    let country_code = clean_number.get(0..2).unwrap_or(&clean_number[0..1]);

    // Look up the country name based on the country code prefix
    country_codes
        .get(country_code)
        .map(|&country| country.to_string()) 
        .unwrap_or_else(|| "Unknown".to_string()) 
}