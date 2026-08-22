#[cfg(unix)]
use std::time::Duration;

#[cfg(unix)]
use tokio::signal::unix;

/// Server control requests translated from OS signals.
pub enum SignalRequest {
    /// Shut down immediately through the graceful shutdown path.
    ImmediateShutdown,
    /// Start the built-in five-minute restart countdown.
    RestartCountdown,
}

#[cfg(unix)]
pub async fn next_signal_request() -> SignalRequest {
    let mut sigint = match unix::signal(unix::SignalKind::interrupt()) {
        Ok(signal) => signal,
        Err(e) => {
            tracing::error!(error = %e, "Failed to install SIGINT handler");
            return SignalRequest::ImmediateShutdown;
        }
    };
    let mut sigterm = match unix::signal(unix::SignalKind::terminate()) {
        Ok(signal) => signal,
        Err(e) => {
            tracing::error!(error = %e, "Failed to install SIGTERM handler");
            return SignalRequest::ImmediateShutdown;
        }
    };
    let mut sigquit = match unix::signal(unix::SignalKind::quit()) {
        Ok(signal) => signal,
        Err(e) => {
            tracing::error!(error = %e, "Failed to install SIGQUIT handler");
            return SignalRequest::ImmediateShutdown;
        }
    };
    let mut sigusr1 = match unix::signal(unix::SignalKind::user_defined1()) {
        Ok(signal) => signal,
        Err(e) => {
            tracing::error!(error = %e, "Failed to install SIGUSR1 handler");
            return SignalRequest::ImmediateShutdown;
        }
    };

    loop {
        tokio::select! {
            _ = sigterm.recv() => return SignalRequest::ImmediateShutdown,
            _ = sigquit.recv() => return SignalRequest::ImmediateShutdown,
            _ = sigusr1.recv() => return SignalRequest::RestartCountdown,
            _ = sigint.recv() => {
                eprintln!(
                    "[!] Press Ctrl+C again within 5 seconds to shut down.\n\
                        (SIGTERM or SIGQUIT will shut down immediately; SIGUSR1 schedules a restart countdown.)"
                );

                tokio::select! {
                    _ = sigint.recv() => return SignalRequest::ImmediateShutdown,
                    _ = sigterm.recv() => return SignalRequest::ImmediateShutdown,
                    _ = sigquit.recv() => return SignalRequest::ImmediateShutdown,
                    _ = sigusr1.recv() => return SignalRequest::RestartCountdown,
                    _ = tokio::time::sleep(Duration::from_secs(5)) => {
                        eprintln!("Shutdown cancelled.");
                    }
                }
            }
        }
    }
}

#[cfg(not(unix))]
pub async fn next_signal_request() -> SignalRequest {
    // Non-Unix fallback (e.g. Windows) uses the standard Ctrl+C signal.
    if let Err(e) = tokio::signal::ctrl_c().await {
        eprintln!("Failed to install Ctrl+C handler: {}", e);
    }
    SignalRequest::ImmediateShutdown
}
