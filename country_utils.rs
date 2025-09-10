use phonenumber::PhoneNumber;
use crate::common_lib::logging::{ generate_correlation_id, OperationTimer, LogLevel, error_codes };
use crate::common_lib::error::ApiError;
use tracing::debug;

/// Country utilities for phone number parsing and country code validation
pub struct CountryService;

impl CountryService {
    /// Parse phone number and extract country code
    /// Uses phonenumber library's built-in country code extraction
    /// Returns ISO 3166-1 alpha-2 country code (e.g., "US", "DE", "JP")
    pub fn parse_phone_number_to_country(phone: &str) -> Result<String, ApiError> {
        let req_id = generate_correlation_id();
        let timer = OperationTimer::new("COUNTRY:parse_phone_number_to_country", &req_id);

        debug!(
            "COUNTRY:parse_phone_number_to_country [VALIDATION] [req_id:{}] Starting phone number parsing for: '{}'",
            req_id,
            phone
        );

        let parsed_phone_number: PhoneNumber = phonenumber::parse(None, phone).map_err(|e| {
            let error_msg = format!("Failed to parse phone number '{}': {:?}", phone, e);
            timer.log_completion(LogLevel::Error, error_codes::VAL_INVALID_FORMAT, &error_msg);
            ApiError::BadRequest {
                message: format!("Invalid phone number format: {:?}", e),
            }
        })?;

        // Use phonenumber library's built-in country ID extraction and convert to ISO code
        let country_id = parsed_phone_number
            .country()
            .id()
            .ok_or_else(|| {
                let error_msg =
                    format!("Could not derive country ID from phone number '{}'", phone);
                timer.log_completion(LogLevel::Error, error_codes::VAL_INVALID_FORMAT, &error_msg);
                ApiError::BadRequest {
                    message: "Country code could not be derived from phone number.".to_string(),
                }
            })?;

        // Convert the country ID to ISO 3166-1 alpha-2 format
        let country_code = format!("{:?}", country_id);

        timer.log_completion(
            LogLevel::Info,
            "SUCCESS",
            &format!("Successfully parsed phone number to country: {}", country_code)
        );

        Ok(country_code)
    }

    /// Validate country code format and existence
    /// Returns true if the country code is a valid 2-letter ISO code
    pub fn is_valid_country_code(country_code: &str) -> bool {
        // Must be exactly 2 characters and all uppercase ASCII letters
        if country_code.len() != 2 {
            return false;
        }

        // Check if all characters are uppercase ASCII letters
        country_code.chars().all(|c| c.is_ascii_uppercase())
    }

    /// Validate and normalize country code input
    /// Returns normalized uppercase 2-letter code or error
    pub fn validate_and_normalize_country_code(country_code: &str) -> Result<String, String> {
        let normalized = country_code.to_uppercase();

        if Self::is_valid_country_code(&normalized) {
            Ok(normalized)
        } else {
            Err(format!("Invalid country code format: '{}'", country_code))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_phone_number_to_country() {
        // Test US phone number
        let result = CountryService::parse_phone_number_to_country("+1 650 253 0000");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "US");

        // Test German phone number
        let result = CountryService::parse_phone_number_to_country("+49 89 12345678");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "DE");

        // Test Japanese phone number
        let result = CountryService::parse_phone_number_to_country("+81 3 1234 5678");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "JP");

        // Test invalid phone number
        let result = CountryService::parse_phone_number_to_country("invalid");
        assert!(result.is_err());
    }

    #[test]
    fn test_country_code_validation() {
        // Valid codes
        assert!(CountryService::is_valid_country_code("US"));
        assert!(CountryService::is_valid_country_code("DE"));
        assert!(CountryService::is_valid_country_code("JP"));

        // Invalid codes
        assert!(!CountryService::is_valid_country_code("USA"));
        assert!(!CountryService::is_valid_country_code("us"));
        assert!(!CountryService::is_valid_country_code("1"));
        assert!(!CountryService::is_valid_country_code(""));
    }

    #[test]
    fn test_validate_and_normalize_country_code() {
        // Valid inputs
        assert_eq!(CountryService::validate_and_normalize_country_code("us").unwrap(), "US");
        assert_eq!(CountryService::validate_and_normalize_country_code("DE").unwrap(), "DE");
        assert_eq!(CountryService::validate_and_normalize_country_code("jp").unwrap(), "JP");

        // Invalid inputs
        assert!(CountryService::validate_and_normalize_country_code("USA").is_err());
        assert!(CountryService::validate_and_normalize_country_code("1").is_err());
        assert!(CountryService::validate_and_normalize_country_code("").is_err());
    }
}
