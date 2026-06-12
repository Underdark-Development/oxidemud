use tokio::signal;

pub async fn shutdown_signal() {
    let ctrl_c = signal::ctrl_c();
    let mut term = signal::unix::signal(signal::unix::SignalKind::terminate())
        .unwrap_or_else(|_| panic!("failed to install SIGTERM handler"));

    tokio::select! {
        _ = ctrl_c => {}
        _ = term.recv() => {}
    }
}
