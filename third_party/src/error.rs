#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error(transparent)]
    HttpClient(#[from] http_client::Error),

    #[error("API call could not be found '{0}'")]
    CallNotFound(String),

    #[error(transparent)]
    Utf8Conversion(#[from] std::str::Utf8Error),

    #[error(transparent)]
    JsonSerialization(#[from] serde_json::Error),

    #[error(transparent)]
    ExtractorParseFailure(#[from] serde_json_path::ParseError),

    #[error("extractor value from expression '{0}' not found")]
    ExtractorValueNotFound(String),

    #[error("cannot handle request with a body and a dependency to append into it")]
    CannotHandleBodyWithDependencyBody,

    #[error("cannot convert multipart request into JSON request")]
    CannotConvertMultipartToJsonRequest,

    #[error("request cannot contain both JSON body and multipart form body")]
    CannotUseJsonAndMultipartTogether,

    #[error("cannot apply dependency body field extractor to multipart request")]
    CannotApplyDependencyBodyFieldToMultipart,
}
