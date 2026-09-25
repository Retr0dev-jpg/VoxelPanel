// Copyright (C) 2026 Marco Simone Cannizzaro
// Licensed under the GNU Affero General Public License v3.0 or later.
// See the LICENSE file in the project root.

/// Stable error categories. The UI maps them to localized text and falls back to `message`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorCode {
    Generic,
    NotFound,
    InvalidInput,
    ServerRunning,
    ServerStopped,
    EulaRequired,
    PortInUse,
    Network,
    Io,
}

#[derive(Debug, Clone, thiserror::Error)]
#[error("{message}")]
pub struct PanelError {
    pub code: ErrorCode,
    pub message: String,
}

pub type PanelResult<T> = Result<T, PanelError>;

impl PanelError {
    #[flutter_rust_bridge::frb(ignore)]
    pub fn new(code: ErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }

    #[flutter_rust_bridge::frb(ignore)]
    pub fn not_found(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::NotFound, message)
    }

    #[flutter_rust_bridge::frb(ignore)]
    pub fn invalid(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::InvalidInput, message)
    }

    #[flutter_rust_bridge::frb(ignore)]
    pub fn running() -> Self {
        Self::new(ErrorCode::ServerRunning, "Ferma il server prima di continuare.")
    }

    #[flutter_rust_bridge::frb(ignore)]
    pub fn stopped() -> Self {
        Self::new(ErrorCode::ServerStopped, "Il server non è in esecuzione")
    }

    #[flutter_rust_bridge::frb(ignore)]
    pub fn network(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::Network, message)
    }

    #[flutter_rust_bridge::frb(ignore)]
    pub fn io(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::Io, message)
    }
}

impl From<String> for PanelError {
    fn from(message: String) -> Self {
        Self::new(ErrorCode::Generic, message)
    }
}

impl From<&str> for PanelError {
    fn from(message: &str) -> Self {
        Self::new(ErrorCode::Generic, message)
    }
}

impl From<std::io::Error> for PanelError {
    fn from(error: std::io::Error) -> Self {
        if error.kind() == std::io::ErrorKind::NotFound {
            Self::not_found(error.to_string())
        } else {
            Self::io(error.to_string())
        }
    }
}

impl From<reqwest::Error> for PanelError {
    fn from(error: reqwest::Error) -> Self {
        Self::network(error.to_string())
    }
}

impl From<serde_json::Error> for PanelError {
    fn from(error: serde_json::Error) -> Self {
        Self::invalid(error.to_string())
    }
}

impl From<tokio::task::JoinError> for PanelError {
    fn from(error: tokio::task::JoinError) -> Self {
        Self::new(ErrorCode::Generic, error.to_string())
    }
}
