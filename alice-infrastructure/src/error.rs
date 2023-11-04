use std::fmt::{Debug, Display};

use actix_http::{
    body::BoxBody,
    header::{self},
    StatusCode,
};
use actix_web::{http::header::ContentType, HttpResponseBuilder, Responder, ResponseError};
use alice_architecture::response::I18NEnum;
use base64::DecodeError;
use serde::Serialize;
use serde_json::json;

#[derive(Debug)]
pub struct AliceError(pub Box<dyn I18NEnum + 'static>);

impl Display for AliceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
impl AliceError {
    pub fn new(e: impl I18NEnum + 'static) -> Self {
        Self(Box::new(e))
    }
}
pub type AliceResult<T> = Result<T, AliceError>;

pub type AliceResponderResult<T> = AliceResult<AliceResponder<T>>;

pub struct AliceResponder<T: Serialize>(pub T);

impl<T: Serialize + 'static> Responder for AliceResponder<T> {
    type Body = BoxBody;

    fn respond_to(self, _req: &actix_web::HttpRequest) -> actix_web::HttpResponse<Self::Body> {
        let json = if std::mem::size_of::<T>() != 0 {
            json!({
                "status": 0,
                "content": self.0,
            })
        } else {
            json!({
                "status": 0,
            })
        };
        HttpResponseBuilder::new(StatusCode::OK)
            .content_type(ContentType::json())
            .body(serde_json::to_string(&json).unwrap())
    }
}

impl ResponseError for AliceError {
    fn status_code(&self) -> StatusCode {
        StatusCode::INTERNAL_SERVER_ERROR
    }

    fn error_response(&self) -> actix_web::HttpResponse<actix_web::body::BoxBody> {
        let mut builder = HttpResponseBuilder::new(self.status_code());
        match self.0.status() {
            400 | 401 | 403 => {
                builder.insert_header((header::WWW_AUTHENTICATE, ""));
            }
            _ => {}
        };
        HttpResponseBuilder::new(self.status_code()).finish()
    }
}

impl From<anyhow::Error> for AliceError {
    fn from(e: anyhow::Error) -> Self {
        AliceError(Box::new(AliceCommonError::InternalError { source: e }))
    }
}

impl From<jsonwebtoken::errors::Error> for AliceError {
    fn from(e: jsonwebtoken::errors::Error) -> Self {
        AliceError(Box::new(AliceCommonError::InvalidToken {
            error_description: e.to_string(),
        }))
    }
}

impl From<DecodeError> for AliceError {
    fn from(e: DecodeError) -> Self {
        AliceError(Box::new(AliceCommonError::InvalidToken {
            error_description: e.to_string(),
        }))
    }
}

impl From<serde_json::error::Error> for AliceError {
    fn from(e: serde_json::error::Error) -> Self {
        AliceError(Box::new(AliceCommonError::InternalError {
            source: e.into(),
        }))
    }
}

impl From<actix_http::header::ToStrError> for AliceError {
    fn from(e: actix_http::header::ToStrError) -> Self {
        AliceError(Box::new(AliceCommonError::InvalidToken {
            error_description: format!("Authorization header isn't string: {e}"),
        }))
    }
}

#[derive(Debug, thiserror::Error, I18NEnum)]
pub enum AliceCommonError {
    #[status(500)]
    #[error("InternalError - cause: {source}")]
    InternalError {
        #[source]
        source: anyhow::Error,
    },
    #[error(
        r#"Baerer realm="CO-COM",error="invalid_request",error_description="{error_description}""#
    )]
    #[status(400)]
    InvalidRequest { error_description: String },
    #[error(
        r#"Baerer realm="CO-COM",error="invalid_token",error_description="{error_description}""#
    )]
    #[status(401)]
    InvalidToken { error_description: String },
    #[error(
        r#"Baerer realm="CO-COM",error="insufficient_scope",error_description="{error_description}""#
    )]
    #[status(403)]
    InsufficientScope { error_description: String },
}
