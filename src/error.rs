use thiserror::Error;

#[derive(Debug, Error)]
pub enum FetchPaperError {
    #[error("error during network request: {0}")]
    Network(#[from] reqwest::Error),

    #[error("failed to parse url: {0}")]
    UrlParse(#[from] url::ParseError),

    #[error("found no pdf url on sci-hub")]
    MissingPdfUrl,

    #[error("found no paper ingo on sci-hub")]
    MissingPaperInfo,
}

#[derive(Debug, Error)]
pub enum FetchBaseUrlError {
    #[error("error during network request: {0}")]
    Reqwest(#[from] reqwest::Error),

    #[error("failed to parse url: {0}")]
    UrlParse(#[from] url::ParseError),

    #[error("no base urls found in provider(s)")]
    NoneFound,
}
