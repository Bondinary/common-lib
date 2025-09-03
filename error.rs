use rocket::{
    http::{ ContentType, Status },
    request::Request,
    response::{ self, Responder, Response },
};
use rocket_okapi::{
    r#gen::OpenApiGenerator,
    okapi::openapi3::Responses,
    response::OpenApiResponderInner,
    OpenApiError,
};
use rocket_okapi::okapi::schemars::Map;
use serde::{ Deserialize, Serialize };
use serde_json::json;
use std::{ error::Error, fmt::{ Display, Formatter } };
use rocket_okapi::okapi::schemars::JsonSchema;
use rocket_okapi::okapi::schemars::{ self };

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "type", content = "details")]
pub enum ApiError {
    NotFound {
        message: String,
    },
    InternalServerError {
        message: String,
    },
    BadRequest {
        message: String,
    },
    Unauthorized {
        message: String,
    },
    PaymentRequired {
        message: String,
    },
    QuotaExceeded {
        resource: String,
        monthly_count: i32,
        lifetime_count: i32,
        monthly_limit: i32,
        lifetime_limit: i32,
    },
    #[serde(rename = "REGISTRATION_REQUIRED")] RegistrationRequired {
        message: String,
        reason: String,
        suggested_action: String,
    },
}

impl ApiError {
    pub fn http_status(&self) -> Status {
        match self {
            ApiError::NotFound { .. } => Status::NotFound,
            ApiError::InternalServerError { .. } => Status::InternalServerError,
            ApiError::BadRequest { .. } => Status::BadRequest,
            ApiError::Unauthorized { .. } => Status::Unauthorized,
            ApiError::PaymentRequired { .. } => Status::PaymentRequired,
            ApiError::QuotaExceeded { .. } => Status::PaymentRequired,
            ApiError::RegistrationRequired { .. } => Status::PreconditionRequired, // 428
        }
    }

    pub fn registration_required(action: &str) -> Self {
        ApiError::RegistrationRequired {
            message: format!("Registration required to {}", action),
            reason: format!("You need to register with your phone number to {}", action),
            suggested_action: "Please complete registration to continue".to_string(),
        }
    }

    pub fn status_code(&self) -> u16 {
        match self {
            ApiError::NotFound { .. } => 404,
            ApiError::InternalServerError { .. } => 500,
            ApiError::BadRequest { .. } => 400,
            ApiError::Unauthorized { .. } => 401,
            ApiError::PaymentRequired { .. } => 402,
            ApiError::QuotaExceeded { .. } => 402,
            ApiError::RegistrationRequired { .. } => 428, // 428 Precondition Required
        }
    }
}

impl Display for ApiError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            ApiError::NotFound { message } => { write!(f, "Not Found: {message}") }
            ApiError::InternalServerError { message } => {
                write!(f, "Internal Server Error: {message}")
            }
            ApiError::BadRequest { message } => { write!(f, "Bad Request Error: {message}") }
            ApiError::Unauthorized { message } => { write!(f, "Unauthorized Error: {message}") }
            ApiError::PaymentRequired { message } => { write!(f, "Payment Required: {message}") }
            ApiError::QuotaExceeded {
                resource,
                monthly_count,
                lifetime_count,
                monthly_limit,
                lifetime_limit,
            } => {
                write!(
                    f,
                    "Quota exceeded for '{resource}': monthly {monthly_count}/{monthly_limit} ; lifetime {lifetime_count}/{lifetime_limit}"
                )
            }
            ApiError::RegistrationRequired { message, reason, suggested_action } => {
                write!(f, "Registration Required: {message} - {reason} - {suggested_action}")
            }
        }
    }
}

impl Error for ApiError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        // Implement the source method if your error type wraps another error
        None
    }
}

impl ApiError {
    // Associated function to convert any error to ApiError
    pub fn from(error: impl std::error::Error + 'static) -> Self {
        ApiError::InternalServerError {
            message: error.to_string(),
        }
    }
}

impl From<Box<dyn std::error::Error>> for ApiError {
    fn from(err: Box<dyn std::error::Error>) -> Self {
        // Convert the error to ApiError here
        ApiError::InternalServerError {
            message: err.to_string(),
        }
    }
}

impl OpenApiResponderInner for ApiError {
    fn responses(_generator: &mut OpenApiGenerator) -> Result<Responses, OpenApiError> {
        use rocket_okapi::okapi::openapi3::{ RefOr, Response as OpenApiResponse };

        let mut responses = Map::new();
        responses.insert(
            "400".to_string(),
            RefOr::Object(OpenApiResponse {
                description: "\
                # [400 Bad Request](https://developer.mozilla.org/en-US/docs/Web/HTTP/Status/400)\n\
                The request given is wrongly formatted or data asked could not be fulfilled. \
                ".to_string(),
                ..Default::default()
            })
        );
        responses.insert(
            "401".to_string(),
            RefOr::Object(OpenApiResponse {
                description: "\
                # [404 Unauthorized](https://developer.mozilla.org/en-US/docs/Web/HTTP/Status/404)\n\
                This response is given when your request is not authorized.\
                ".to_string(),
                ..Default::default()
            })
        );
        responses.insert(
            "404".to_string(),
            RefOr::Object(OpenApiResponse {
                description: "\
                # [404 Not Found](https://developer.mozilla.org/en-US/docs/Web/HTTP/Status/404)\n\
                This response is given when you request a page that does not exists.\
                ".to_string(),
                ..Default::default()
            })
        );
        responses.insert(
            "422".to_string(),
            RefOr::Object(OpenApiResponse {
                description: "\
                # [422 Unprocessable Entity](https://developer.mozilla.org/en-US/docs/Web/HTTP/Status/422)\n\
                This response is given when you request body is not correctly formatted. \
                ".to_string(),
                ..Default::default()
            })
        );
        responses.insert(
            "500".to_string(),
            RefOr::Object(OpenApiResponse {
                description: "\
                # [500 Internal Server Error](https://developer.mozilla.org/en-US/docs/Web/HTTP/Status/500)\n\
                This response is given when something wend wrong on the server. \
                ".to_string(),
                ..Default::default()
            })
        );
        Ok(Responses {
            responses,
            ..Default::default()
        })
    }
}

impl<'r> Responder<'r, 'static> for ApiError {
    fn respond_to(self, _: &'r Request<'_>) -> response::Result<'static> {
        let status_code = self.http_status();
        let error_response = json!({ "error": self.to_string() });
        let body = serde_json::to_string(&error_response).unwrap();

        Response::build()
            .sized_body(body.len(), std::io::Cursor::new(body))
            .header(ContentType::JSON)
            .status(status_code)
            .ok()
    }
}

impl From<String> for ApiError {
    fn from(message: String) -> Self {
        // By default, convert generic String errors to InternalServerError
        ApiError::InternalServerError { message: format!("Generic conversion error: {message}") }
    }
}
