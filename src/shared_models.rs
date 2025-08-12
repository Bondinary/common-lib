use chrono::{TimeZone, Utc};
use mongodb::bson::oid::{self, ObjectId};
use mongodb::bson::{Bson, DateTime};
use rocket_okapi::okapi::openapi3::SchemaObject;
use rocket_okapi::okapi::schemars::r#gen::SchemaGenerator;
use rocket_okapi::okapi::schemars::schema::Schema;
use rocket_okapi::okapi::schemars::JsonSchema;
use rocket_okapi::okapi::schemars::{self};
use serde::de::{self, Visitor};
use serde::ser::Error;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::cmp::{Eq, Ord, PartialEq, PartialOrd};
use std::fmt;

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct MyObjectId(pub ObjectId);

impl MyObjectId {
    pub fn new() -> Self {
        MyObjectId(ObjectId::new())
    }
    
    pub fn to_string(&self) -> String {
        self.0.to_hex()
    }

    pub fn parse_string(s: &str) -> Result<Self, oid::Error> {
        ObjectId::parse_str(s).map(MyObjectId)
    }

    pub fn try_parse_str(s: &str) -> Result<Self, mongodb::bson::oid::Error> {
        ObjectId::parse_str(s).map(MyObjectId)
    }
    
    // Check if the MyObjectId is "empty" (i.e., it has the default ObjectId)
    pub fn is_empty(&self) -> bool {
        self.0 == ObjectId::new() // Compare it to a newly generated ObjectId
    }
}

impl fmt::Display for MyObjectId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0.to_hex())
    }
}

// Implement From<MyObjectId> for Bson to allow conversion in the filter
impl From<MyObjectId> for Bson {
    fn from(my_object_id: MyObjectId) -> Self {
        Bson::ObjectId(my_object_id.0) // Convert MyObjectId to Bson::ObjectId
    }
}

impl From<ObjectId> for MyObjectId {
    fn from(oid: ObjectId) -> Self {
        MyObjectId(oid)
    }
}

// Implement JsonSchema for the newtype
impl JsonSchema for MyObjectId {
    fn schema_name() -> String {
        "ObjectId".to_string()
    }

    fn json_schema(r#gen: &mut schemars::r#gen::SchemaGenerator) -> schemars::schema::Schema {
        <String as JsonSchema>::json_schema(r#gen)
    }
}

// Custom serializer for MyObjectId to serialize it as a hexadecimal string
pub fn serialize_object_id<S>(oid: &MyObjectId, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_str(&oid.0.to_hex()) // Convert the ObjectId to a hex string
}

// Custom serializer for Option<MyObjectId> to serialize it as a hexadecimal string
pub fn serialize_object_id_option<S>(oid: &Option<MyObjectId>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    match oid {
        Some(id) => serializer.serialize_some(&id.0.to_hex()), // Convert ObjectId to a hex string if Some
        None => serializer.serialize_none(),                 // Serialize None as null
    }
}

#[derive(Debug, Clone)]
pub struct MyDateTime(pub DateTime);

// Implementing JsonSchema for NaiveDateTime
impl JsonSchema for MyDateTime {
    fn schema_name() -> String {
        "MyDateTime".to_string()
    }

    fn json_schema(_gen: &mut SchemaGenerator) -> Schema {
        let mut schema = SchemaObject::default();
        schema.format = Some("date-time".to_string()); // Specify it is a date-time
        Schema::Object(schema)
    }
}

impl Serialize for MyDateTime {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        // Convert `bson::DateTime` to a `chrono::DateTime<Utc>` and serialize as RFC 3339 string
        let chrono_dt: chrono::DateTime<Utc> = Utc
            .timestamp_millis_opt(self.0.timestamp_millis())
            .single() // This will handle the option
            .ok_or_else(|| S::Error::custom("Invalid timestamp"))?; // Handle the case where the result is None
        serializer.serialize_str(&chrono_dt.to_rfc3339())
    }
}

impl<'de> Deserialize<'de> for MyDateTime {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        // Define a visitor to convert the string to a DateTime<Utc>
        struct MyDateTimeVisitor;

        impl<'de> Visitor<'de> for MyDateTimeVisitor {
            type Value = MyDateTime;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                write!(formatter, "a valid RFC 3339 datetime string")
            }

            fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                // Parse the string as a chrono DateTime<Utc>
                let chrono_dt = DateTime::parse_rfc3339_str(value)
                    .map_err(|_| E::custom("invalid RFC 3339 datetime format"))?;
                Ok(MyDateTime(chrono_dt))
            }
        }

        deserializer.deserialize_str(MyDateTimeVisitor)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(crate = "rocket::serde")]
pub struct EncryptedMessage {
    pub address: String,
    pub encrypted_message: String,
}

impl EncryptedMessage {
    pub fn is_valid(&self) -> bool {
        // Check if the address is empty
        if self.address.is_empty() {
            return false;
        }

        // Check if the encrypted message contains any data
        if self.encrypted_message.is_empty() {
            return false;
        }

        // If both conditions are met, the message is considered valid
        true
    }

    // Method to convert EncryptedMessage to JSON string
    pub fn to_json(&self) -> String {
        // Serialize the EncryptedMessage to JSON
        serde_json::to_string(self).unwrap_or_else(|_| String::new())
    }
}

#[derive(Deserialize, JsonSchema)]
pub struct DevicesDeleteRequest {
    pub device_ids: Vec<String>,
}

#[derive(Serialize, Deserialize)]
pub struct TwilioApiKeyResponse {
    pub sid: String,
    pub friendly_name: String,
    pub date_created: String,
    pub date_updated: String,
    pub secret: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct IdNamePair {
    pub id: String,
    pub name: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct IdKeyPair {
    pub id: String,
    pub key: String,
}
