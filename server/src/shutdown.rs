use std::fmt;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::OnceLock;
use std::time::Duration;

use tokio::sync::watch;

const RESTART_COUNTDOWN_MINUTES: [u64; 5] = [5, 4, 3, 2, 1];

static SHUTDOWN_TX: OnceLock<watch::Sender<bool>> = OnceLock::new();
static SCHEDULED_SHUTDOWN_ACTIVE: AtomicBool = AtomicBool::new(false);

/// Errors returned by server shutdown control operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShutdownControlError {
    /// The shutdown channel has not been registered yet.
    NotInstalled,
    /// The shutdown channel was already registered.
    AlreadyInstalled,
    /// A scheduled shutdown or restart countdown is already running.
    AlreadyScheduled,
    /// The shutdown receiver side has gone away.
    ReceiverClosed,
}

impl fmt::Display for ShutdownControlError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotInstalled => write!(f, "shutdown control has not been installed"),
            Self::AlreadyInstalled => write!(f, "shutdown control is already installed"),
            Self::AlreadyScheduled => write!(f, "a scheduled shutdown is already active"),
            Self::ReceiverClosed => write!(f, "shutdown receiver is closed"),
        }
    }
}

impl std::error::Error for ShutdownControlError {}

/// Register the server's shutdown watch channel for signal and API callers.
pub fn install_shutdown_sender(sender: watch::Sender<bool>) -> Result<(), ShutdownControlError> {
    SHUTDOWN_TX
        .set(sender)
        .map_err(|_| ShutdownControlError::AlreadyInstalled)
}

/// Request an immediate graceful shutdown through the normal server path.
pub fn request_immediate_shutdown(reason: &str) -> Result<(), ShutdownControlError> {
    tracing::info!(reason, "Immediate graceful shutdown requested");
    shutdown_sender()?
        .send(true)
        .map_err(|_| ShutdownControlError::ReceiverClosed)
}

/// Schedule a delayed graceful shutdown without bypassing server cleanup.
pub fn schedule_delayed_shutdown(
    delay: Duration,
    reason: String,
) -> Result<(), ShutdownControlError> {
    begin_scheduled_shutdown()?;
    tokio::spawn(async move {
        tracing::info!(?delay, reason, "Delayed graceful shutdown scheduled");
        if delay > Duration::ZERO {
            tokio::time::sleep(delay).await;
        }
        if let Err(e) = request_immediate_shutdown("scheduled delayed shutdown") {
            tracing::error!(error = %e, "Failed to request delayed graceful shutdown");
        }
        SCHEDULED_SHUTDOWN_ACTIVE.store(false, Ordering::SeqCst);
    });
    Ok(())
}

/// Broadcast a five-minute restart countdown, then gracefully shut down.
pub fn schedule_restart_countdown(version: Option<String>) -> Result<(), ShutdownControlError> {
    begin_scheduled_shutdown()?;
    tokio::spawn(async move {
        tracing::info!(?version, "Restart countdown scheduled");
        for minutes in RESTART_COUNTDOWN_MINUTES {
            let message = restart_countdown_message(version.as_deref(), minutes);
            let sent = crate::console_broadcast(&message).await;
            tracing::info!(minutes, sent, "Restart countdown broadcast sent");
            tokio::time::sleep(Duration::from_secs(60)).await;
        }
        if let Err(e) = request_immediate_shutdown("scheduled restart countdown complete") {
            tracing::error!(error = %e, "Failed to request restart shutdown");
        }
        SCHEDULED_SHUTDOWN_ACTIVE.store(false, Ordering::SeqCst);
    });
    Ok(())
}

fn begin_scheduled_shutdown() -> Result<(), ShutdownControlError> {
    let _ = shutdown_sender()?;
    SCHEDULED_SHUTDOWN_ACTIVE
        .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        .map_err(|_| ShutdownControlError::AlreadyScheduled)?;
    Ok(())
}

fn shutdown_sender() -> Result<&'static watch::Sender<bool>, ShutdownControlError> {
    SHUTDOWN_TX.get().ok_or(ShutdownControlError::NotInstalled)
}

fn restart_countdown_message(version: Option<&str>, minutes: u64) -> String {
    match (version, minutes) {
        (Some(version), 5) => {
            format!("New server version upgrade (v{version}), server will restart in 5 minutes...")
        }
        (_, 1) => "Server will restart in 1 minute...".to_string(),
        (_, minutes) => format!("Server will restart in {minutes} minutes..."),
    }
}

#[cfg(test)]
mod tests {
    use super::restart_countdown_message;

    #[test]
    fn restart_countdown_mentions_version_on_first_message() {
        assert_eq!(
            restart_countdown_message(Some("0.7.1"), 5),
            "New server version upgrade (v0.7.1), server will restart in 5 minutes..."
        );
    }

    #[test]
    fn restart_countdown_uses_singular_minute() {
        assert_eq!(
            restart_countdown_message(None, 1),
            "Server will restart in 1 minute..."
        );
    }
}
