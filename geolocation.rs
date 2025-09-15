use std::collections::HashMap;
use std::sync::Arc;
use std::time::{ Duration, Instant };
use reqwest::Client;
use serde::{ Deserialize, Serialize };
use tokio::sync::RwLock;
use tracing::{ debug, error, info };

use crate::common_lib::error::ApiError;
use crate::common_lib::logging::{ generate_correlation_id, OperationTimer, LogLevel };

/// Geolocation information extracted from IP address
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocationInfo {
    pub country_code: String,
    pub country_name: String,
    pub city: Option<String>,
    pub region: Option<String>,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub timezone: Option<String>,
}

/// Response structure for ip-api.com fallback service
#[derive(Debug, Deserialize)]
struct FallbackApiResponse {
    status: String,
    country: String,
    #[serde(rename = "countryCode")]
    country_code: String,
    #[allow(dead_code)]
    region: String,
    #[serde(rename = "regionName")]
    region_name: String,
    city: String,
    #[allow(dead_code)]
    zip: String,
    lat: f64,
    lon: f64,
    timezone: String,
    #[allow(dead_code)]
    isp: String,
    #[allow(dead_code)]
    org: String,
    #[serde(rename = "as")]
    #[allow(dead_code)]
    as_name: String,
    #[allow(dead_code)]
    query: String,
    message: Option<String>, // Error message when status != "success"
}

/// Cache entry for geolocation results
#[derive(Debug, Clone)]
struct CacheEntry {
    location: LocationInfo,
    timestamp: Instant,
}

/// Configuration for geolocation service
#[derive(Debug, Clone)]
pub struct GeolocationConfig {
    pub api_key: String,
    pub service_url: String,
    pub timeout_seconds: u64,
    pub cache_ttl_seconds: u64,
    pub max_cache_entries: usize,
}

impl Default for GeolocationConfig {
    fn default() -> Self {
        Self {
            api_key: String::new(),
            service_url: "https://api.maxmind.com/geoip/v2.1/city".to_string(),
            timeout_seconds: 5,
            cache_ttl_seconds: 3600, // 1 hour
            max_cache_entries: 10000,
        }
    }
}

/// MaxMind GeoIP2 API response structure
#[derive(Debug, Deserialize)]
struct MaxMindResponse {
    country: MaxMindCountry,
    city: Option<MaxMindCity>,
    location: Option<MaxMindLocation>,
    subdivisions: Option<Vec<MaxMindSubdivision>>,
}

#[derive(Debug, Deserialize)]
struct MaxMindCountry {
    iso_code: String,
    names: HashMap<String, String>,
}

#[derive(Debug, Deserialize)]
struct MaxMindCity {
    names: HashMap<String, String>,
}

#[derive(Debug, Deserialize)]
struct MaxMindLocation {
    latitude: Option<f64>,
    longitude: Option<f64>,
    time_zone: Option<String>,
}

#[derive(Debug, Deserialize)]
struct MaxMindSubdivision {
    names: HashMap<String, String>,
}

/// High-performance geolocation service with caching
pub struct GeolocationService {
    client: Arc<Client>,
    config: GeolocationConfig,
    cache: Arc<RwLock<HashMap<String, CacheEntry>>>,
}

impl GeolocationService {
    /// Create new geolocation service with configuration
    pub fn new(client: Arc<Client>, config: GeolocationConfig) -> Self {
        Self {
            client,
            config,
            cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Get location information for IP address with caching
    pub async fn get_location(&self, ip_address: &str) -> Result<LocationInfo, ApiError> {
        let req_id = generate_correlation_id();
        let timer = OperationTimer::new("GEO:get_location", &req_id);

        debug!(
            "GEO:get_location [START] [req_id:{}] Processing IP lookup - ip: {}",
            req_id,
            ip_address
        );

        // 1. Input validation
        if ip_address.trim().is_empty() {
            error!("GEO:get_location [VALIDATION] [req_id:{}] Empty IP address provided", req_id);
            return Err(ApiError::BadRequest {
                message: "IP address is required".to_string(),
            });
        }

        // 2. Check cache first
        if let Some(cached_location) = self.get_from_cache(ip_address).await {
            debug!(
                "GEO:get_location [CACHE_HIT] [req_id:{}] Found cached location - ip: {}, country: {}",
                req_id,
                ip_address,
                cached_location.country_code
            );

            timer.log_completion(
                LogLevel::Info,
                "CACHE_HIT",
                &format!(
                    "Location retrieved from cache - ip: {}, country: {}",
                    ip_address,
                    cached_location.country_code
                )
            );

            return Ok(cached_location);
        }

        // 3. Call external geolocation API
        debug!(
            "GEO:get_location [API_CALL] [req_id:{}] Cache miss, calling external API - ip: {}",
            req_id,
            ip_address
        );

        let location = self.fetch_from_api(ip_address, &req_id).await?;

        // 4. Cache the result
        self.cache_location(ip_address, &location).await;

        debug!(
            "GEO:get_location [SUCCESS] [req_id:{}] Location retrieved and cached - ip: {}, country: {}, city: {:?}",
            req_id,
            ip_address,
            location.country_code,
            location.city
        );

        timer.log_completion(
            LogLevel::Info,
            "SUCCESS",
            &format!(
                "Location retrieved from API - ip: {}, country: {}",
                ip_address,
                location.country_code
            )
        );

        Ok(location)
    }

    /// Get location from cache if valid
    async fn get_from_cache(&self, ip_address: &str) -> Option<LocationInfo> {
        let cache = self.cache.read().await;

        if let Some(entry) = cache.get(ip_address) {
            let age = entry.timestamp.elapsed();
            let ttl = Duration::from_secs(self.config.cache_ttl_seconds);

            if age < ttl {
                return Some(entry.location.clone());
            }
        }

        None
    }

    /// Cache location result
    async fn cache_location(&self, ip_address: &str, location: &LocationInfo) {
        let mut cache = self.cache.write().await;

        // Clean old entries if cache is too large
        if cache.len() >= self.config.max_cache_entries {
            let now = Instant::now();
            let ttl = Duration::from_secs(self.config.cache_ttl_seconds);

            cache.retain(|_, entry| now.duration_since(entry.timestamp) < ttl);

            // If still too large, remove oldest entries
            if cache.len() >= self.config.max_cache_entries {
                let mut entries_with_timestamps: Vec<(String, Instant)> = cache
                    .iter()
                    .map(|(ip, entry)| (ip.clone(), entry.timestamp))
                    .collect();

                entries_with_timestamps.sort_by_key(|(_, timestamp)| *timestamp);

                let to_remove = cache.len() - self.config.max_cache_entries + 1;
                for (ip, _) in entries_with_timestamps.into_iter().take(to_remove) {
                    cache.remove(&ip);
                }
            }
        }

        cache.insert(ip_address.to_string(), CacheEntry {
            location: location.clone(),
            timestamp: Instant::now(),
        });
    }

    /// Fetch location from external API (MaxMind or fallback)
    async fn fetch_from_api(
        &self,
        ip_address: &str,
        req_id: &str
    ) -> Result<LocationInfo, ApiError> {
        // First try MaxMind if we have a valid API key
        if
            !self.config.api_key.is_empty() &&
            self.config.api_key != "demo_key" &&
            self.config.api_key != "your_maxmind_api_key"
        {
            match self.fetch_from_maxmind(ip_address, req_id).await {
                Ok(location) => {
                    return Ok(location);
                }
                Err(e) => {
                    debug!(
                        "GEO:fetch_from_api [MAXMIND_FALLBACK] [req_id:{}] MaxMind failed, trying fallback - ip: {}, error: {}",
                        req_id,
                        ip_address,
                        e
                    );
                }
            }
        }

        // Fallback to free service
        self.fetch_from_fallback_service(ip_address, req_id).await
    }

    /// Fetch location from MaxMind API
    async fn fetch_from_maxmind(
        &self,
        ip_address: &str,
        req_id: &str
    ) -> Result<LocationInfo, ApiError> {
        // Construct API URL
        let url = format!("{}/{}", self.config.service_url, ip_address);

        debug!(
            "GEO:fetch_from_api [API_REQUEST] [req_id:{}] Calling MaxMind API - url: {}",
            req_id,
            url
        );

        // Build request with authentication and timeout
        let response = self.client
            .get(&url)
            .basic_auth(&self.config.api_key, Some(""))
            .timeout(Duration::from_secs(self.config.timeout_seconds))
            .send().await
            .map_err(|e| {
                error!(
                    "GEO:fetch_from_api [API_ERROR] [req_id:{}] Request failed - ip: {}, error: {}",
                    req_id,
                    ip_address,
                    e
                );
                ApiError::InternalServerError {
                    message: format!("Geolocation API request failed: {e}"),
                }
            })?;

        // Check HTTP status
        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();

            error!(
                "GEO:fetch_from_api [API_ERROR] [req_id:{}] Non-success status - ip: {}, status: {}, body: {}",
                req_id,
                ip_address,
                status,
                body
            );

            // Handle specific error cases
            match status.as_u16() {
                401 => {
                    return Err(ApiError::InternalServerError {
                        message: "Geolocation service authentication failed".to_string(),
                    });
                }
                404 => {
                    return Ok(self.default_location());
                } // IP not found, use default
                429 => {
                    return Err(ApiError::InternalServerError {
                        message: "Geolocation service rate limited".to_string(),
                    });
                }
                _ => {
                    return Err(ApiError::InternalServerError {
                        message: format!("Geolocation service error: {status}"),
                    });
                }
            }
        }

        // Parse response
        let maxmind_response: MaxMindResponse = response.json().await.map_err(|e| {
            error!(
                "GEO:fetch_from_api [PARSE_ERROR] [req_id:{}] JSON parsing failed - ip: {}, error: {}",
                req_id,
                ip_address,
                e
            );
            ApiError::InternalServerError {
                message: format!("Failed to parse geolocation response: {e}"),
            }
        })?;

        // Convert to our location format
        let location = self.convert_maxmind_response(maxmind_response);

        debug!(
            "GEO:fetch_from_maxmind [API_SUCCESS] [req_id:{}] Response parsed - ip: {}, country: {}, city: {:?}",
            req_id,
            ip_address,
            location.country_code,
            location.city
        );

        Ok(location)
    }

    /// Fetch location from fallback free service (ip-api.com)
    async fn fetch_from_fallback_service(
        &self,
        ip_address: &str,
        req_id: &str
    ) -> Result<LocationInfo, ApiError> {
        let url = format!("http://ip-api.com/json/{ip_address}");

        debug!(
            "GEO:fetch_from_fallback_service [API_REQUEST] [req_id:{}] Calling fallback API - url: {}",
            req_id,
            url
        );

        let response = self.client
            .get(&url)
            .timeout(Duration::from_secs(self.config.timeout_seconds))
            .send().await
            .map_err(|e| {
                error!(
                    "GEO:fetch_from_fallback_service [API_ERROR] [req_id:{}] Request failed - ip: {}, error: {}",
                    req_id,
                    ip_address,
                    e
                );
                ApiError::InternalServerError {
                    message: format!("Fallback geolocation API request failed: {e}"),
                }
            })?;

        if !response.status().is_success() {
            let status = response.status();
            error!(
                "GEO:fetch_from_fallback_service [API_ERROR] [req_id:{}] Non-success status - ip: {}, status: {}",
                req_id,
                ip_address,
                status
            );
            return Ok(self.default_location());
        }

        // Parse ip-api.com response format
        let fallback_response: FallbackApiResponse = response.json().await.map_err(|e| {
            error!(
                "GEO:fetch_from_fallback_service [PARSE_ERROR] [req_id:{}] JSON parsing failed - ip: {}, error: {}",
                req_id,
                ip_address,
                e
            );
            ApiError::InternalServerError {
                message: format!("Failed to parse fallback geolocation response: {e}"),
            }
        })?;

        if fallback_response.status != "success" {
            debug!(
                "GEO:fetch_from_fallback_service [API_ERROR] [req_id:{}] API returned failure - ip: {}, message: {:?}",
                req_id,
                ip_address,
                fallback_response.message
            );
            return Ok(self.default_location());
        }

        let location = LocationInfo {
            country_code: fallback_response.country_code,
            country_name: fallback_response.country,
            city: Some(fallback_response.city),
            region: Some(fallback_response.region_name),
            latitude: Some(fallback_response.lat),
            longitude: Some(fallback_response.lon),
            timezone: Some(fallback_response.timezone),
        };

        debug!(
            "GEO:fetch_from_fallback_service [API_SUCCESS] [req_id:{}] Response parsed - ip: {}, country: {}, city: {:?}",
            req_id,
            ip_address,
            location.country_code,
            location.city
        );

        Ok(location)
    }

    /// Convert MaxMind response to our LocationInfo format
    fn convert_maxmind_response(&self, response: MaxMindResponse) -> LocationInfo {
        let country_code = response.country.iso_code;
        let country_name = response.country.names
            .get("en")
            .cloned()
            .unwrap_or_else(|| country_code.clone());

        let city = response.city.and_then(|c| c.names.get("en").cloned());

        let region = response.subdivisions
            .as_ref()
            .and_then(|subdivisions| subdivisions.first())
            .and_then(|subdivision| subdivision.names.get("en"))
            .cloned();

        let (latitude, longitude, timezone) = response.location
            .map(|loc| (loc.latitude, loc.longitude, loc.time_zone))
            .unwrap_or((None, None, None));

        LocationInfo {
            country_code,
            country_name,
            city,
            region,
            latitude,
            longitude,
            timezone,
        }
    }

    /// Fallback location when IP lookup fails
    fn default_location(&self) -> LocationInfo {
        LocationInfo {
            country_code: "US".to_string(),
            country_name: "United States".to_string(),
            city: None,
            region: None,
            latitude: None,
            longitude: None,
            timezone: None,
        }
    }

    /// Health check for geolocation service
    pub async fn health_check(&self) -> Result<(), ApiError> {
        let req_id = generate_correlation_id();

        debug!("GEO:health_check [START] [req_id:{}] Testing service connectivity", req_id);

        // Test with a known IP (Google DNS)
        match self.get_location("8.8.8.8").await {
            Ok(location) => {
                info!(
                    "GEO:health_check [SUCCESS] [req_id:{}] Service healthy - test_country: {}",
                    req_id,
                    location.country_code
                );
                Ok(())
            }
            Err(e) => {
                error!(
                    "GEO:health_check [FAILED] [req_id:{}] Service unhealthy - error: {}",
                    req_id,
                    e
                );
                Err(e)
            }
        }
    }

    /// Get cache statistics for monitoring
    pub async fn get_cache_stats(&self) -> (usize, usize) {
        let cache = self.cache.read().await;
        let total_entries = cache.len();

        let now = Instant::now();
        let ttl = Duration::from_secs(self.config.cache_ttl_seconds);
        let valid_entries = cache
            .values()
            .filter(|entry| now.duration_since(entry.timestamp) < ttl)
            .count();

        (total_entries, valid_entries)
    }
}

/// Extract real client IP from request headers (handles API Gateway forwarding)
pub fn extract_client_ip_from_headers(headers: &rocket::http::HeaderMap) -> Option<String> {
    // Try X-Forwarded-For first (API Gateway standard)
    if let Some(forwarded_for) = headers.get_one("X-Forwarded-For") {
        // X-Forwarded-For can contain multiple IPs: "client, proxy1, proxy2"
        // The first IP is usually the real client IP
        if let Some(client_ip) = forwarded_for.split(',').next() {
            let trimmed_ip = client_ip.trim();
            if !trimmed_ip.is_empty() && trimmed_ip != "unknown" {
                return Some(trimmed_ip.to_string());
            }
        }
    }

    // Try X-Real-IP (Nginx proxy standard)
    if let Some(real_ip) = headers.get_one("X-Real-IP") {
        let trimmed_ip = real_ip.trim();
        if !trimmed_ip.is_empty() && trimmed_ip != "unknown" {
            return Some(trimmed_ip.to_string());
        }
    }

    // Try CF-Connecting-IP (Cloudflare)
    if let Some(cf_ip) = headers.get_one("CF-Connecting-IP") {
        let trimmed_ip = cf_ip.trim();
        if !trimmed_ip.is_empty() && trimmed_ip != "unknown" {
            return Some(trimmed_ip.to_string());
        }
    }

    // Try X-Client-IP
    if let Some(client_ip) = headers.get_one("X-Client-IP") {
        let trimmed_ip = client_ip.trim();
        if !trimmed_ip.is_empty() && trimmed_ip != "unknown" {
            return Some(trimmed_ip.to_string());
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_client_ip_from_headers() {
        let mut headers = rocket::http::HeaderMap::new();

        // Test X-Forwarded-For with single IP
        headers.add_raw("X-Forwarded-For", "192.168.1.1");
        assert_eq!(extract_client_ip_from_headers(&headers), Some("192.168.1.1".to_string()));

        // Test X-Forwarded-For with multiple IPs
        headers.replace_raw("X-Forwarded-For", "192.168.1.1, 10.0.0.1, 172.16.0.1");
        assert_eq!(extract_client_ip_from_headers(&headers), Some("192.168.1.1".to_string()));

        // Test X-Real-IP fallback
        headers.remove("X-Forwarded-For");
        headers.add_raw("X-Real-IP", "203.0.113.1");
        assert_eq!(extract_client_ip_from_headers(&headers), Some("203.0.113.1".to_string()));

        // Test no headers
        headers.remove("X-Real-IP");
        assert_eq!(extract_client_ip_from_headers(&headers), None);
    }

    #[test]
    fn test_location_info_serialization() {
        let location = LocationInfo {
            country_code: "US".to_string(),
            country_name: "United States".to_string(),
            city: Some("New York".to_string()),
            region: Some("New York".to_string()),
            latitude: Some(40.7128),
            longitude: Some(-74.006),
            timezone: Some("America/New_York".to_string()),
        };

        let json = serde_json::to_string(&location).unwrap();
        let deserialized: LocationInfo = serde_json::from_str(&json).unwrap();

        assert_eq!(location.country_code, deserialized.country_code);
        assert_eq!(location.city, deserialized.city);
    }
}
