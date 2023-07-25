#![warn(missing_docs)]
//! Bind XML, JSON, URL-encoded or query-string form data in Axum.
//!
//! # Example
//!
//! ```rust
//! use axum::http::StatusCode;
//! use axum_bindform::{BindForm, TryBindForm};
//! use serde::{Deserialize, Serialize};
//!
//! #[derive(Deserialize, Serialize)]
//! struct Human {
//!     name: String,
//!     age: u8,
//! }
//!
//! async fn greet_human(BindForm(form): BindForm<Human>) -> String {
//!     format!("Hello {} year old named {}!", form.age, form.name)
//! }
//!
//! async fn try_greet_human(
//!     TryBindForm(form): TryBindForm<Human>,
//! ) -> Result<String, (StatusCode, String)> {
//!     let form = form.map_err(|e| {
//!         (
//!             StatusCode::BAD_REQUEST,
//!             format!("Error parsing form: {}", e),
//!         )
//!     })?;
//!     Ok(format!("Hello {} year old named {}!", form.age, form.name))
//! }
//! ```
use std::convert::Infallible;

use axum::{
    async_trait,
    body::HttpBody,
    extract::{rejection::BytesRejection, FromRequest},
    http::Request,
    response::IntoResponse,
    BoxError,
};
use serde::de::DeserializeOwned;
use thiserror::Error;

mod bind;

/// Errors that can occur when binding.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum BindError {
    /// Invalid mime type.
    #[error("invalid mime type")]
    InvalidMimeType,
    /// Body read error.
    #[error("body read error: {0}")]
    BodyReadError(BytesRejection),
    /// JSON deserialization error.
    #[cfg(feature = "json")]
    #[error("json error: {0}")]
    JsonError(serde_json::Error),
    /// URL-encoded deserialization error.
    #[cfg(feature = "urlencoded")]
    #[error("urlencoded error: {0}")]
    UrlEncodedError(serde_urlencoded::de::Error),
    /// XML deserialization error.
    #[cfg(feature = "xml")]
    #[error("xml error: {0}")]
    XmlError(serde_xml_rs::Error),
}

/// Result of binding.
pub type BindResult<T> = Result<T, BindError>;

/// Try to bind form data in Axum and return the result, does not reject.
pub struct TryBindForm<T: DeserializeOwned>(pub BindResult<T>);

#[async_trait]
impl<S, B, T> FromRequest<S, B> for TryBindForm<T>
where
    T: DeserializeOwned,
    B: HttpBody + Send + 'static,
    B::Data: Send,
    B::Error: Into<BoxError>,
    S: Send + Sync,
{
    type Rejection = Infallible;

    async fn from_request(req: Request<B>, state: &S) -> Result<Self, Self::Rejection> {
        let deserialized = bind::bind_request(req, state).await;

        Ok(TryBindForm(deserialized))
    }
}

/// Bind form data in Axum, rejects on error.
pub struct BindForm<T: DeserializeOwned>(pub T);

/// Rejection for [`BindForm`].
pub struct BindFormRejection(BindError);

impl IntoResponse for BindFormRejection {
    fn into_response(self) -> axum::response::Response {
        let body = format!("{}", self.0);
        (
            axum::http::StatusCode::BAD_REQUEST,
            [(axum::http::header::CONTENT_TYPE, "text/plain")],
            body,
        )
            .into_response()
    }
}

#[async_trait]
impl<S, B, T> FromRequest<S, B> for BindForm<T>
where
    T: DeserializeOwned,
    B: HttpBody + Send + 'static,
    B::Data: Send,
    B::Error: Into<BoxError>,
    S: Send + Sync,
{
    type Rejection = BindFormRejection;

    async fn from_request(req: Request<B>, state: &S) -> Result<Self, Self::Rejection> {
        let deserialized = bind::bind_request(req, state).await;

        match deserialized {
            Ok(deserialized) => Ok(BindForm(deserialized)),
            Err(err) => Err(BindFormRejection(err)),
        }
    }
}
