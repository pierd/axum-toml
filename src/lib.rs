use axum::{
    async_trait,
    body::HttpBody,
    extract::FromRequest,
    http::{header, HeaderValue, Request, StatusCode},
    response::{IntoResponse, Response},
    BoxError,
};

const TOML_MIME: &str = "application/toml";
const TEXT_UTF8_MIME: &str = "text/plain; charset=utf-8";

#[derive(Debug, thiserror::Error)]
enum TomlRejection {
    #[error("Failed to deserialize the request body")]
    DeserializationError(#[from] toml::de::Error),
    #[error("Request body didn't contain valid UTF-8")]
    StringRejection(#[from] axum::extract::rejection::StringRejection),
}

impl IntoResponse for TomlRejection {
    fn into_response(self) -> Response {
        match self {
            Self::DeserializationError(err) => (
                StatusCode::BAD_REQUEST,
                [(
                    header::CONTENT_TYPE,
                    HeaderValue::from_static(TEXT_UTF8_MIME),
                )],
                err.to_string(),
            )
                .into_response(),
            Self::StringRejection(err) => err.into_response(),
        }
    }
}

struct Toml<T>(pub T);

#[async_trait]
impl<T, S, B> FromRequest<S, B> for Toml<T>
where
    T: serde::de::DeserializeOwned,
    B: HttpBody + Send + 'static,
    B::Data: Send,
    B::Error: Into<BoxError>,
    S: Send + Sync,
{
    type Rejection = TomlRejection;

    async fn from_request(req: Request<B>, state: &S) -> Result<Self, Self::Rejection> {
        let text = String::from_request(req, state).await?;
        Ok(Toml(toml::from_str(&text)?))
    }
}

impl<T> IntoResponse for Toml<T>
where
    T: serde::Serialize,
{
    fn into_response(self) -> Response {
        match toml::to_string(&self.0) {
            Ok(serialized) => (
                StatusCode::OK,
                [(header::CONTENT_TYPE, HeaderValue::from_static(TOML_MIME))],
                serialized,
            )
                .into_response(),
            Err(err) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                [(
                    header::CONTENT_TYPE,
                    HeaderValue::from_static(TEXT_UTF8_MIME),
                )],
                err.to_string(),
            )
                .into_response(),
        }
    }
}
