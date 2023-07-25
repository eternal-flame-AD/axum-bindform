use axum::{body::HttpBody, extract::FromRequest, http::Request, BoxError};
use serde::de::DeserializeOwned;

use crate::{BindError, BindResult};

pub(crate) async fn bind_request<S, B, T>(req: Request<B>, state: &S) -> BindResult<T>
where
    T: DeserializeOwned,
    B: HttpBody + Send + 'static,
    B::Data: Send,
    B::Error: Into<BoxError>,
    S: Send + Sync,
{
    let content_type = req
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    #[cfg(feature = "urlencoded")]
    if content_type.is_empty() {
        let query = req.uri().query().unwrap_or("");
        let deserialized =
            serde_urlencoded::from_str::<T>(query).map_err(BindError::UrlEncodedError)?;

        return Ok(deserialized);
    }

    let mime = match content_type.parse::<mime::Mime>() {
        Ok(mime) => mime,
        Err(_) => return Err(BindError::InvalidMimeType),
    };

    #[cfg(feature = "json")]
    if mime.type_() == mime::APPLICATION
        && (mime.subtype() == mime::JSON || mime.suffix() == Some(mime::JSON))
    {
        let bytes = axum::body::Bytes::from_request(req, state)
            .await
            .map_err(BindError::BodyReadError)?;

        let deserialized = serde_json::from_slice(&bytes).map_err(BindError::JsonError)?;

        return Ok(deserialized);
    }

    #[cfg(feature = "urlencoded")]
    if mime.type_() == mime::APPLICATION
        && (mime.subtype() == mime::WWW_FORM_URLENCODED
            || mime.suffix() == Some(mime::WWW_FORM_URLENCODED))
    {
        let bytes = axum::body::Bytes::from_request(req, state)
            .await
            .map_err(BindError::BodyReadError)?;

        let query = std::str::from_utf8(&bytes).unwrap_or("");
        let deserialized =
            serde_urlencoded::from_str::<T>(query).map_err(BindError::UrlEncodedError)?;

        return Ok(deserialized);
    }

    #[cfg(feature = "xml")]
    if mime.type_() == mime::APPLICATION
        && (mime.subtype() == mime::XML || mime.suffix() == Some(mime::XML))
    {
        let bytes = axum::body::Bytes::from_request(req, state)
            .await
            .map_err(BindError::BodyReadError)?;

        let deserialized = serde_xml_rs::from_reader(&bytes[..]).map_err(BindError::XmlError)?;

        return Ok(deserialized);
    }

    Err(BindError::InvalidMimeType)
}
