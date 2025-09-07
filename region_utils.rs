use std::collections::HashMap;

/// Data regions for MongoDB Atlas Global Cluster sharding
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum DataRegion {
    EU, // European Union - GDPR compliant
    US, // United States
    APAC, // Asia Pacific
}

impl DataRegion {
    /// Convert DataRegion to string for database storage
    pub fn to_string(&self) -> String {
        match self {
            DataRegion::EU => "EU".to_string(),
            DataRegion::US => "US".to_string(),
            DataRegion::APAC => "APAC".to_string(),
        }
    }

    /// Parse string back to DataRegion
    pub fn from_string(s: &str) -> Option<DataRegion> {
        match s {
            "EU" => Some(DataRegion::EU),
            "US" => Some(DataRegion::US),
            "APAC" => Some(DataRegion::APAC),
            _ => None,
        }
    }
}

/// Service to determine data region based on country code
pub struct RegionService;

impl RegionService {
    /// Map country code to data region for strict data residency compliance
    /// This mapping ensures users' data is stored in their designated geographical zone
    pub fn get_region_for_country(country_code: &str) -> DataRegion {
        // Create static mapping of country codes to regions
        let region_map = Self::create_country_region_map();

        region_map.get(country_code.to_uppercase().as_str()).cloned().unwrap_or(DataRegion::US) // Default to US for unknown countries
    }

    /// Create comprehensive mapping of country codes to data regions
    /// Based on data sovereignty laws and geographical proximity
    fn create_country_region_map() -> HashMap<&'static str, DataRegion> {
        let mut map = HashMap::new();

        // European Union countries + GDPR-compliant regions
        let eu_countries = [
            "AT",
            "BE",
            "BG",
            "CY",
            "CZ",
            "DE",
            "DK",
            "EE",
            "ES",
            "FI",
            "FR",
            "GR",
            "HR",
            "HU",
            "IE",
            "IT",
            "LT",
            "LU",
            "LV",
            "MT",
            "NL",
            "PL",
            "PT",
            "RO",
            "SE",
            "SI",
            "SK",
            // Brexit but still GDPR-like requirements
            "GB",
            "UK",
            // EEA countries
            "IS",
            "LI",
            "NO",
            // Other European countries with similar data protection laws
            "CH",
            "AL",
            "BA",
            "BY",
            "MD",
            "ME",
            "MK",
            "RS",
            "UA",
            "XK",
        ];

        // Asia-Pacific countries
        let apac_countries = [
            "AU",
            "JP",
            "KR",
            "SG",
            "HK",
            "TW",
            "NZ",
            "MY",
            "TH",
            "ID",
            "PH",
            "VN",
            "IN",
            "BD",
            "LK",
            "MM",
            "KH",
            "LA",
            "BN",
            "MN",
            "KZ",
            "KG",
            "TJ",
            "TM",
            "UZ",
            "AF",
            "PK",
            "NP",
            "BT",
            "MV",
            "CN",
            "FJ",
            "PG",
            "SB",
            "VU",
            "NC",
            "PF",
            "WS",
            "TO",
            "KI",
            "NR",
            "TV",
            "FM",
            "MH",
            "PW",
            "CK",
            "NU",
            "TK",
            "WF",
            "AS",
            "GU",
            "MP",
        ];

        // Add EU countries to map
        for country in eu_countries.iter() {
            map.insert(*country, DataRegion::EU);
        }

        // Add APAC countries to map
        for country in apac_countries.iter() {
            map.insert(*country, DataRegion::APAC);
        }

        // Americas default to US (including North, Central, South America)
        let us_countries = [
            "US",
            "CA",
            "MX",
            "BR",
            "AR",
            "CL",
            "CO",
            "PE",
            "VE",
            "EC",
            "BO",
            "UY",
            "PY",
            "GY",
            "SR",
            "FK",
            "GF",
            "GT",
            "HN",
            "SV",
            "NI",
            "CR",
            "PA",
            "BZ",
            "JM",
            "HT",
            "DO",
            "TT",
            "BB",
            "GD",
            "LC",
            "VC",
            "AG",
            "DM",
            "KN",
            "BS",
            "CU",
            "PR",
            "VI",
            "VG",
            "AI",
            "MS",
            "KY",
            "TC",
            "BM",
            "GL",
            "PM",
            "BQ",
            "CW",
            "AW",
            "SX",
            "MQ",
            "GP",
            "BL",
            "MF",
        ];

        for country in us_countries.iter() {
            map.insert(*country, DataRegion::US);
        }

        // Middle East and Africa default to EU (closest data protection regulations)
        let mea_countries = [
            "AE",
            "SA",
            "QA",
            "KW",
            "BH",
            "OM",
            "JO",
            "LB",
            "SY",
            "IQ",
            "IR",
            "IL",
            "PS",
            "TR",
            "EG",
            "LY",
            "TN",
            "DZ",
            "MA",
            "SD",
            "SS",
            "ET",
            "ER",
            "DJ",
            "SO",
            "KE",
            "UG",
            "TZ",
            "RW",
            "BI",
            "CD",
            "CF",
            "CM",
            "TD",
            "NE",
            "NG",
            "BJ",
            "TG",
            "GH",
            "BF",
            "CI",
            "LR",
            "SL",
            "GN",
            "GW",
            "GM",
            "SN",
            "MR",
            "ML",
            "CV",
            "ST",
            "GQ",
            "GA",
            "CG",
            "AO",
            "ZM",
            "MW",
            "MZ",
            "ZW",
            "BW",
            "NA",
            "ZA",
            "LS",
            "SZ",
            "MG",
            "MU",
            "SC",
            "KM",
            "YT",
            "RE",
            "SH",
            "YE",
            "GE",
            "AM",
            "AZ",
        ];

        for country in mea_countries.iter() {
            map.insert(*country, DataRegion::EU);
        }

        map
    }

    /// Get all supported regions
    pub fn get_all_regions() -> Vec<DataRegion> {
        vec![DataRegion::EU, DataRegion::US, DataRegion::APAC]
    }

    /// Check if a country requires strict data residency (GDPR-like)
    pub fn requires_strict_residency(country_code: &str) -> bool {
        let region = Self::get_region_for_country(country_code);
        matches!(region, DataRegion::EU) // EU has strictest data residency requirements
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_eu_countries_mapped_correctly() {
        assert_eq!(RegionService::get_region_for_country("DE"), DataRegion::EU);
        assert_eq!(RegionService::get_region_for_country("FR"), DataRegion::EU);
        assert_eq!(RegionService::get_region_for_country("GB"), DataRegion::EU);
        assert_eq!(RegionService::get_region_for_country("CH"), DataRegion::EU);
    }

    #[test]
    fn test_us_countries_mapped_correctly() {
        assert_eq!(RegionService::get_region_for_country("US"), DataRegion::US);
        assert_eq!(RegionService::get_region_for_country("CA"), DataRegion::US);
        assert_eq!(RegionService::get_region_for_country("BR"), DataRegion::US);
        assert_eq!(RegionService::get_region_for_country("MX"), DataRegion::US);
    }

    #[test]
    fn test_apac_countries_mapped_correctly() {
        assert_eq!(RegionService::get_region_for_country("JP"), DataRegion::APAC);
        assert_eq!(RegionService::get_region_for_country("AU"), DataRegion::APAC);
        assert_eq!(RegionService::get_region_for_country("SG"), DataRegion::APAC);
        assert_eq!(RegionService::get_region_for_country("IN"), DataRegion::APAC);
    }

    #[test]
    fn test_unknown_country_defaults_to_us() {
        assert_eq!(RegionService::get_region_for_country("UNKNOWN"), DataRegion::US);
        assert_eq!(RegionService::get_region_for_country("ZZ"), DataRegion::US);
    }

    #[test]
    fn test_case_insensitive() {
        assert_eq!(RegionService::get_region_for_country("us"), DataRegion::US);
        assert_eq!(RegionService::get_region_for_country("de"), DataRegion::EU);
        assert_eq!(RegionService::get_region_for_country("jp"), DataRegion::APAC);
    }

    #[test]
    fn test_data_region_string_conversion() {
        assert_eq!(DataRegion::EU.to_string(), "EU");
        assert_eq!(DataRegion::US.to_string(), "US");
        assert_eq!(DataRegion::APAC.to_string(), "APAC");

        assert_eq!(DataRegion::from_string("EU"), Some(DataRegion::EU));
        assert_eq!(DataRegion::from_string("US"), Some(DataRegion::US));
        assert_eq!(DataRegion::from_string("APAC"), Some(DataRegion::APAC));
        assert_eq!(DataRegion::from_string("INVALID"), None);
    }

    #[test]
    fn test_strict_residency_requirements() {
        assert!(RegionService::requires_strict_residency("DE")); // Germany requires strict GDPR
        assert!(RegionService::requires_strict_residency("FR")); // France requires strict GDPR
        assert!(!RegionService::requires_strict_residency("US")); // US doesn't require strict residency
        assert!(!RegionService::requires_strict_residency("JP")); // Japan doesn't require strict residency
    }
}
