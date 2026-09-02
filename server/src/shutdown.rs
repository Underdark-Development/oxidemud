use std::collections::BTreeMap;
use std::fmt;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::Duration;

use tokio::sync::watch;

/// Per-minute marks broadcast inside the final five minutes, in seconds remaining.
const FINAL_MINUTES_SECS: [u64; 5] = [300, 240, 180, 120, 60];
/// Sub-minute marks broadcast during the final stretch, in seconds remaining.
const FINAL_SECONDS_MARKS: [u64; 7] = [30, 10, 5, 4, 3, 2, 1];
/// Hourly announcements stop once this much time remains; the per-minute
/// countdown takes over below this threshold.
const FINAL_TAIL_THRESHOLD_SECS: u64 = 300;

static SHUTDOWN_TX: OnceLock<watch::Sender<bool>> = OnceLock::new();
static SCHEDULED_SHUTDOWN_ACTIVE: AtomicBool = AtomicBool::new(false);
static SCHEDULED_TASK: OnceLock<Mutex<Option<tokio::task::JoinHandle<()>>>> = OnceLock::new();
/// Bumped on every schedule, cancel, and natural completion so in-flight
/// countdown tasks can detect that they have been superseded.
static SCHEDULE_EPOCH: AtomicU64 = AtomicU64::new(0);

/// Errors returned by server shutdown control operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ShutdownControlError {
    /// The shutdown channel has not been registered yet.
    NotInstalled,
    /// The shutdown channel was already registered.
    AlreadyInstalled,
    /// A scheduled shutdown or restart countdown is already running.
    AlreadyScheduled,
    /// The requested shutdown delay is not acceptable.
    InvalidDelay(String),
    /// The shutdown receiver side has gone away.
    ReceiverClosed,
}

impl fmt::Display for ShutdownControlError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotInstalled => write!(f, "shutdown control has not been installed"),
            Self::AlreadyInstalled => write!(f, "shutdown control is already installed"),
            Self::AlreadyScheduled => write!(
                f,
                "a scheduled shutdown is already active (cancel it first)"
            ),
            Self::InvalidDelay(msg) => write!(f, "invalid shutdown delay: {msg}"),
            Self::ReceiverClosed => write!(f, "shutdown receiver is closed"),
        }
    }
}

impl std::error::Error for ShutdownControlError {}

/// How a shutdown request was dispatched.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShutdownDispatch {
    /// The server was told to shut down immediately.
    Immediate,
    /// A countdown task now owns the shutdown and will fire later.
    Scheduled,
}

/// How the countdown broadcasts are worded.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CountdownKind {
    Shutdown,
    Restart,
}

impl CountdownKind {
    fn verb(&self) -> &'static str {
        match self {
            Self::Shutdown => "shut down",
            Self::Restart => "restart",
        }
    }

    fn final_message(&self) -> &'static str {
        match self {
            Self::Shutdown => "Shutting down now!",
            Self::Restart => "Restarting now!",
        }
    }
}

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

/// Validate a delay expressed in whole minutes against the configured cap.
///
/// Zero is always valid and yields a zero duration (= shut down immediately).
pub fn validate_delay_minutes(mins: u32) -> Result<Duration, ShutdownControlError> {
    let cap = crate::config::shutdown_max_delay_mins();
    if mins > cap {
        return Err(ShutdownControlError::InvalidDelay(format!(
            "delay must be at most {cap} minutes"
        )));
    }
    Ok(Duration::from_secs(u64::from(mins) * 60))
}

/// Shut down after `delay`, or immediately when `delay` is zero.
///
/// This is the single entry point all callers (signals, console, in-game
/// command, API) should use to request a future shutdown.
pub fn request_shutdown(
    delay: Duration,
    reason: &str,
) -> Result<ShutdownDispatch, ShutdownControlError> {
    if delay.is_zero() {
        request_immediate_shutdown(reason)?;
        Ok(ShutdownDispatch::Immediate)
    } else {
        schedule_delayed_shutdown(delay, reason)?;
        Ok(ShutdownDispatch::Scheduled)
    }
}

/// Schedule an announced, cancellable graceful shutdown countdown.
pub fn schedule_delayed_shutdown(
    delay: Duration,
    reason: &str,
) -> Result<(), ShutdownControlError> {
    let cap = crate::config::shutdown_max_delay_mins();
    if delay > Duration::from_secs(u64::from(cap) * 60) {
        return Err(ShutdownControlError::InvalidDelay(format!(
            "delay must be at most {cap} minutes"
        )));
    }
    let marks = countdown_marks(delay, CountdownKind::Shutdown);
    spawn_countdown_task(marks, delay, reason.to_owned())
}

/// Broadcast a five-minute restart countdown, then gracefully shut down.
///
/// Used by SIGUSR1 / the deploy restart workflow; shares the countdown
/// machinery with [`schedule_delayed_shutdown`] but uses restart wording.
pub fn schedule_restart_countdown() -> Result<(), ShutdownControlError> {
    let delay = Duration::from_secs(FINAL_TAIL_THRESHOLD_SECS);
    let marks = countdown_marks(delay, CountdownKind::Restart);
    spawn_countdown_task(marks, delay, "scheduled restart countdown".to_owned())
}

/// Cancel a pending scheduled shutdown, if one is active.
///
/// Returns whether a countdown was actually cancelled. Players are notified.
pub fn cancel_scheduled_shutdown(reason: &str) -> bool {
    let slot = SCHEDULED_TASK.get_or_init(|| Mutex::new(None));
    let handle = slot.lock().ok().and_then(|mut guard| guard.take());
    let Some(handle) = handle else {
        tracing::info!(
            reason,
            "Cancel requested but no scheduled shutdown is active"
        );
        return false;
    };

    // If the task already finished naturally, the shutdown may have just been
    // signalled — do not announce a cancellation that cancels nothing.
    if handle.is_finished() {
        return false;
    }

    handle.abort();
    SCHEDULE_EPOCH.fetch_add(1, Ordering::SeqCst);
    SCHEDULED_SHUTDOWN_ACTIVE.store(false, Ordering::SeqCst);
    tracing::warn!(target: "audit", reason, "Scheduled shutdown cancelled");

    // Notify players from a detached task so this stays callable from sync
    // contexts (e.g. in-game command handlers). Best-effort only.
    tokio::spawn(async move {
        crate::console_broadcast("Server shutdown cancelled.").await;
    });
    true
}

/// Whether a delayed shutdown or restart countdown is currently pending.
pub fn scheduled_shutdown_pending() -> bool {
    SCHEDULED_SHUTDOWN_ACTIVE.load(Ordering::SeqCst)
}

fn begin_scheduled_shutdown() -> Result<u64, ShutdownControlError> {
    let _ = shutdown_sender()?;
    SCHEDULED_SHUTDOWN_ACTIVE
        .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        .map_err(|_| ShutdownControlError::AlreadyScheduled)?;
    let epoch = SCHEDULE_EPOCH.fetch_add(1, Ordering::SeqCst) + 1;
    Ok(epoch)
}

fn clear_scheduled_state() {
    SCHEDULE_EPOCH.fetch_add(1, Ordering::SeqCst);
    SCHEDULED_SHUTDOWN_ACTIVE.store(false, Ordering::SeqCst);
    if let Some(slot) = SCHEDULED_TASK.get() {
        if let Ok(mut guard) = slot.lock() {
            *guard = None;
        }
    }
}

fn spawn_countdown_task(
    marks: Vec<(Duration, String)>,
    total: Duration,
    reason: String,
) -> Result<(), ShutdownControlError> {
    let my_epoch = begin_scheduled_shutdown()?;
    // Convert remaining-time marks into absolute offsets from task start.
    let schedule: Vec<(Duration, String)> = marks
        .into_iter()
        .map(|(remaining, message)| (total - remaining, message))
        .collect();

    let slot = SCHEDULED_TASK.get_or_init(|| Mutex::new(None));
    let handle = tokio::spawn(async move {
        let start = tokio::time::Instant::now();
        for (offset, message) in &schedule {
            if SCHEDULE_EPOCH.load(Ordering::SeqCst) != my_epoch {
                return;
            }
            tokio::time::sleep_until(start + *offset).await;
            let sent = crate::console_broadcast(message).await;
            tracing::info!(
                in_secs = offset.as_secs(),
                sent,
                "Scheduled shutdown countdown broadcast"
            );
        }
        if SCHEDULE_EPOCH.load(Ordering::SeqCst) != my_epoch {
            return;
        }
        if let Err(e) = request_immediate_shutdown(&reason) {
            tracing::error!(error = %e, "Failed to request scheduled shutdown");
        }
        clear_scheduled_state();
    });

    if let Ok(mut guard) = slot.lock() {
        *guard = Some(handle);
    }
    Ok(())
}

fn shutdown_sender() -> Result<&'static watch::Sender<bool>, ShutdownControlError> {
    SHUTDOWN_TX.get().ok_or(ShutdownControlError::NotInstalled)
}

/// Build the ordered countdown mark list (remaining time, broadcast text) for
/// a total delay, skipping marks that fall beyond it.
fn countdown_marks(total: Duration, kind: CountdownKind) -> Vec<(Duration, String)> {
    let total_secs = total.as_secs();
    let mut marks: BTreeMap<u64, String> = BTreeMap::new();

    // Schedule-time announcement.
    marks.insert(total_secs, remaining_message(kind, total_secs));

    // Hourly announcements while more than the final tail remains.
    let mut hours = 1u64;
    while hours * 3600 < total_secs && total_secs - hours * 3600 > FINAL_TAIL_THRESHOLD_SECS {
        let remaining = total_secs - hours * 3600;
        marks
            .entry(remaining)
            .or_insert_with(|| remaining_message(kind, remaining));
        hours += 1;
    }

    // Final five minutes, one announcement per minute.
    for &secs in &FINAL_MINUTES_SECS {
        if secs < total_secs {
            marks
                .entry(secs)
                .or_insert_with(|| remaining_message(kind, secs));
        }
    }

    // Final stretch seconds.
    for &secs in &FINAL_SECONDS_MARKS {
        if secs < total_secs {
            marks
                .entry(secs)
                .or_insert_with(|| tail_message(kind, secs));
        }
    }

    // The final "now" announcement.
    marks.insert(0, kind.final_message().to_string());

    marks
        .into_iter()
        .rev()
        .map(|(secs, message)| (Duration::from_secs(secs), message))
        .collect()
}

fn remaining_message(kind: CountdownKind, secs: u64) -> String {
    format!(
        "Server will {} in {}...",
        kind.verb(),
        human_duration(Duration::from_secs(secs))
    )
}

fn tail_message(kind: CountdownKind, secs: u64) -> String {
    if secs >= 10 {
        format!("Server will {} in {secs} seconds...", kind.verb())
    } else {
        format!("{secs}...")
    }
}

fn human_duration(d: Duration) -> String {
    let secs = d.as_secs();
    if secs.is_multiple_of(3600) {
        let hours = secs / 3600;
        if hours == 1 {
            "1 hour".to_string()
        } else {
            format!("{hours} hours")
        }
    } else if secs.is_multiple_of(60) {
        let mins = secs / 60;
        if mins == 1 {
            "1 minute".to_string()
        } else {
            format!("{mins} minutes")
        }
    } else if secs == 1 {
        "1 second".to_string()
    } else {
        format!("{secs} seconds")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn messages(marks: &[(Duration, String)]) -> Vec<&str> {
        marks.iter().map(|(_, m)| m.as_str()).collect()
    }

    fn remaining_secs(marks: &[(Duration, String)]) -> Vec<u64> {
        marks.iter().map(|(d, _)| d.as_secs()).collect()
    }

    #[test]
    fn shutdown_five_minutes_has_full_countdown() {
        let marks = countdown_marks(Duration::from_secs(300), CountdownKind::Shutdown);
        let secs = remaining_secs(&marks);
        assert_eq!(secs, vec![300, 240, 180, 120, 60, 30, 10, 5, 4, 3, 2, 1, 0]);
        let msgs = messages(&marks);
        assert_eq!(msgs[0], "Server will shut down in 5 minutes...");
        assert_eq!(msgs[4], "Server will shut down in 1 minute...");
        assert_eq!(msgs[5], "Server will shut down in 30 seconds...");
        assert_eq!(msgs[6], "Server will shut down in 10 seconds...");
        assert_eq!(msgs[7], "5...");
        assert_eq!(msgs[12], "Shutting down now!");
    }

    #[test]
    fn restart_wording_is_used_for_restart_kind() {
        let marks = countdown_marks(Duration::from_secs(300), CountdownKind::Restart);
        let msgs = messages(&marks);
        assert_eq!(msgs[0], "Server will restart in 5 minutes...");
        assert_eq!(msgs[4], "Server will restart in 1 minute...");
        assert_eq!(msgs[12], "Restarting now!");
    }

    #[test]
    fn long_delay_announces_hourly_then_tail() {
        let marks = countdown_marks(Duration::from_secs(5400), CountdownKind::Shutdown);
        assert_eq!(
            messages(&marks)[0],
            "Server will shut down in 90 minutes..."
        );
        assert!(
            marks
                .iter()
                .any(|(_, m)| m == "Server will shut down in 30 minutes..."),
            "expected an hourly mark at 30 minutes remaining"
        );
        // No per-hour marks between 30m and the 5-minute tail.
        let secs = remaining_secs(&marks);
        assert!(!secs.iter().any(|&s| s > 300 && s < 1800));
    }

    #[test]
    fn multi_hour_delay_uses_hour_wording() {
        let marks = countdown_marks(Duration::from_secs(10800), CountdownKind::Shutdown);
        assert_eq!(messages(&marks)[0], "Server will shut down in 3 hours...");
        assert!(marks
            .iter()
            .any(|(_, m)| m == "Server will shut down in 1 hour..."));
    }

    #[test]
    fn one_minute_delay_skips_longer_marks() {
        let marks = countdown_marks(Duration::from_secs(60), CountdownKind::Shutdown);
        let secs = remaining_secs(&marks);
        assert_eq!(secs, vec![60, 30, 10, 5, 4, 3, 2, 1, 0]);
        assert_eq!(messages(&marks)[0], "Server will shut down in 1 minute...");
    }

    #[test]
    fn sub_minute_delay_skips_minute_marks() {
        let marks = countdown_marks(Duration::from_secs(45), CountdownKind::Shutdown);
        let secs = remaining_secs(&marks);
        assert_eq!(secs, vec![45, 30, 10, 5, 4, 3, 2, 1, 0]);
        assert_eq!(
            messages(&marks)[0],
            "Server will shut down in 45 seconds..."
        );
    }

    #[test]
    fn human_duration_formats_hours_minutes_seconds() {
        assert_eq!(human_duration(Duration::from_secs(3600)), "1 hour");
        assert_eq!(human_duration(Duration::from_secs(7200)), "2 hours");
        assert_eq!(human_duration(Duration::from_secs(120)), "2 minutes");
        assert_eq!(human_duration(Duration::from_secs(60)), "1 minute");
        assert_eq!(human_duration(Duration::from_secs(1)), "1 second");
        assert_eq!(human_duration(Duration::from_secs(45)), "45 seconds");
    }

    #[test]
    fn validate_delay_accepts_zero_through_cap() {
        assert_eq!(validate_delay_minutes(0).unwrap(), Duration::from_secs(0));
        assert_eq!(validate_delay_minutes(1).unwrap(), Duration::from_secs(60));
        assert_eq!(
            validate_delay_minutes(300).unwrap(),
            Duration::from_secs(18000)
        );
    }

    #[test]
    fn validate_delay_rejects_values_over_cap() {
        assert!(matches!(
            validate_delay_minutes(301),
            Err(ShutdownControlError::InvalidDelay(_))
        ));
    }

    #[tokio::test]
    async fn cancel_aborts_pending_shutdown_without_signalling() {
        let (_tx, mut rx) = watch::channel(false);
        let _ = install_shutdown_sender(_tx.clone());

        schedule_delayed_shutdown(Duration::from_secs(2), "test").expect("schedule should succeed");
        assert!(scheduled_shutdown_pending());
        assert!(cancel_scheduled_shutdown("test"));
        assert!(!scheduled_shutdown_pending());

        // The countdown must not fire the shutdown channel after cancellation.
        let result = tokio::time::timeout(Duration::from_millis(700), rx.changed()).await;
        assert!(
            result.is_err(),
            "shutdown signal should not fire after cancel"
        );
    }
}
