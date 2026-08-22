#[derive(Debug, thiserror::Error)]
pub enum RpcError {
    #[error("transport error: {0}")]
    Transport(#[from] tokio_tungstenite::tungstenite::Error),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("malformed message: {0}")]
    Malformed(String),
    #[error("server error: {0}")]
    Server(String),
    #[error("method '{0}' not found")]
    MethodNotFound(String),
    #[error("request timed out")]
    Timeout,
    #[error("connection closed")]
    Closed,
}
