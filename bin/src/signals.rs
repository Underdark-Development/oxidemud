use std::time::Duration;

use tokio::signal::unix;

pub async fn shutdown_signal() {
    let mut sigint = unix::signal(unix::SignalKind::interrupt())
        .unwrap_or_else(|_| panic!("failed to install SIGINT handler"));
    let mut sigterm = unix::signal(unix::SignalKind::terminate())
        .unwrap_or_else(|_| panic!("failed to install SIGTERM handler"));

    // SIGTERM — immediate shutdown (systemd/init, never accidental).
    // SIGINT — back-to-back required: first prints warning, second within
    // 5 seconds triggers shutdown.
    loop {
        tokio::select! {
            _ = sigterm.recv() => break,
            _ = sigint.recv() => {
                eprintln!(
                    "[!] Press Ctrl+C again within 5 seconds to shut down.\n\
                        (SIGTERM will shut down immediately.)"
                );

                tokio::select! {
                    _ = sigint.recv() => break,
                    _ = sigterm.recv() => break,
                    _ = tokio::time::sleep(Duration::from_secs(5)) => {
                        eprintln!("Shutdown cancelled.");
                        continue;
                    }
                }
            }
        }
    }
}
