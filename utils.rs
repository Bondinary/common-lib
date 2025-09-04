use hex::encode;
use rand::Rng;
use rocket::tokio::io::AsyncReadExt;
use rusoto_core::Region;
use rusoto_s3::{ GetObjectRequest, S3Client, S3 };
use tracing::{ debug, error, warn };
use std::error::Error;
use crate::common_lib::shared_models::MyObjectId;
use chrono::{ TimeZone, Utc };
use mongodb::bson::DateTime;

pub fn generate_random_token() -> String {
    let mut rng = rand::rng();
    let random_number = rng.random_range(111111..=999999);
    random_number.to_string()
}

pub fn generate_random_alphanumeric_string() -> String {
    let mut rng = rand::rng();
    let random_key_bytes: [u8; 32] = rng.random();

    // Convert the byte array to a hexadecimal string
    encode(random_key_bytes)
}

pub fn get_env_var(
    key: &str,
    default_value: Option<&str>
) -> Result<String, Box<dyn Error + Send + Sync>> {
    match std::env::var(key) {
        Ok(value) => Ok(value),
        Err(e) => {
            if let Some(default) = default_value {
                // If a default value is provided, use it and log a warning
                warn!("Environment variable '{}' not set. Using default value: '{}'", key, default);
                Ok(default.to_string())
            } else {
                // If no default value is provided, and the variable is missing, return an error
                let error_message = format!("Environment variable {key} not set: {e}");
                error!("{error_message}");
                Err(Box::new(std::io::Error::other(error_message)) as Box<dyn Error + Send + Sync>)
            }
        }
    }
}

pub async fn download_file_from_s3(
    bucket_name: &str,
    object_key: &str
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

    let secret = match secret_manager.get_secret_value().secret_id(secret_name).send().await {
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
            return Err(Box::new(err));
        }
    };

    // For a list of exceptions thrown, see
    // https://docs.aws.amazon.com/secretsmanager/latest/apireference/API_GetSecretValue.html

    Ok(secret)
}

// === ObjectId Parsing Utilities ===

/// Parse an optional ObjectId string, returning None for empty or None strings
pub fn parse_optional_object_id(id_str: Option<&str>) -> Result<Option<MyObjectId>, String> {
    match id_str {
        Some(s) if !s.is_empty() =>
            MyObjectId::parse_string(s)
                .map(Some)
                .map_err(|e| e.to_string()),
        _ => Ok(None),
    }
}

/// Parse a required ObjectId string from a String reference
pub fn parse_required_object_id_from_string(id_str: &str) -> Result<MyObjectId, String> {
    MyObjectId::parse_string(id_str).map_err(|e| e.to_string())
}

/// Parse a required ObjectId string, returning an error for empty or None strings
pub fn parse_required_object_id(
    id_str: Option<&str>,
    field_name: &str
) -> Result<MyObjectId, String> {
    match id_str {
        Some(s) if !s.is_empty() =>
            MyObjectId::parse_string(s).map_err(|e|
                format!("Invalid {} format: {}", field_name, e)
            ),
        _ => Err(format!("Missing required field: {}", field_name)),
    }
}

/// Parse an optional ObjectId from an Option<String>, handling Option<String> cases
pub fn parse_optional_object_id_from_option_string(
    id_str: Option<String>
) -> Result<Option<MyObjectId>, String> {
    match id_str {
        Some(s) if !s.is_empty() =>
            MyObjectId::parse_string(&s)
                .map(Some)
                .map_err(|e| e.to_string()),
        _ => Ok(None),
    }
}

/// Convert an optional MyObjectId to an optional string
pub fn optional_object_id_to_string(id: &Option<MyObjectId>) -> Option<String> {
    id.as_ref().map(|oid| oid.to_string())
}

// === DateTime Conversion Utilities ===

/// Convert MongoDB DateTime to Chrono DateTime<Utc>
pub fn chrono_from_mongo_datetime(dt: &DateTime) -> Result<chrono::DateTime<Utc>, String> {
    Utc.timestamp_millis_opt(dt.timestamp_millis())
        .single()
        .ok_or_else(|| format!("Invalid timestamp: {}", dt.timestamp_millis()))
}

/// Convert Chrono DateTime<Utc> to MongoDB DateTime
pub fn mongo_from_chrono_datetime(dt: chrono::DateTime<Utc>) -> DateTime {
    DateTime::from_millis(dt.timestamp_millis())
}
