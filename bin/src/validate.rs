use std::path::Path;

use oxide_core::content::load_registry_report;

/// Run content preflight validation against a content directory.
///
/// Parses `server.toml` (if present), loads the full template tree, runs the
/// registry cross-reference validation rules, and prints a human-readable
/// report to stdout. Returns the process exit code: `0` when clean, `1` when
/// any parse or validation error was found.
///
/// This mode is side-effect free: it never opens the database, binds ports,
/// writes logs, or starts the game loop. Intended for deployment pipelines
/// and operator preflight checks before a version cutover.
pub fn run_preflight(content_path: &Path, config_path: &Path) -> i32 {
    let mut errors = 0usize;

    // Preflight runs before the runtime logging stack is built, so install a
    // minimal stdout subscriber for diagnostics emitted via tracing (e.g.
    // zero-spawn warnings). Levels below WARN are suppressed.
    let _ = tracing_subscriber::fmt()
        .with_writer(std::io::stdout)
        .with_max_level(tracing::level_filters::LevelFilter::WARN)
        .try_init();

    println!("OxideMUD content preflight — {}", content_path.display());

    // 1. server.toml (strict: parse errors are failures, unlike runtime
    //    init which falls back to defaults)
    if config_path.exists() {
        match oxide_server::config::validate_file(config_path) {
            Ok(()) => println!("  [ok]   {}", config_path.display()),
            Err(e) => {
                errors += 1;
                println!("  [fail] {e}");
            }
        }
    } else {
        println!(
            "  [skip] {} (not found; runtime will use built-in defaults)",
            config_path.display()
        );
    }

    // 2. Content tree parse
    let report = load_registry_report(content_path);
    for err in &report.errors {
        errors += 1;
        println!(
            "  [fail] {} ({}): {}",
            err.path.display(),
            err.category,
            err.message
        );
    }

    // 3. Cross-reference validation rules
    let validation_errors = report.registry.validate();
    for err in &validation_errors {
        errors += 1;
        println!(
            "  [fail] {} '{}': {} ({})",
            err.template_type, err.template_id, err.message, err.field
        );
    }

    // A world with no spawn points is not an error: new characters fall back
    // to the Void room. Surface it through the logging layer so operators see
    // a real WARN record.
    let total_spawns: usize = report.registry.areas.values().map(|a| a.spawns.len()).sum();
    if total_spawns == 0 {
        tracing::warn!(
            "no spawn points defined across all areas — new characters will appear in The Void"
        );
    }

    // Summary
    let r = &report.registry;
    println!(
        "Loaded: {} races, {} classes, {} items, {} mobs, {} areas, {} skills, {} quests",
        r.races.len(),
        r.classes.len(),
        r.items.len(),
        r.mobs.len(),
        r.areas.len(),
        r.skills.len(),
        r.quests.len()
    );

    if errors == 0 {
        println!("Result: OK — no content errors found.");
        0
    } else {
        println!("Result: FAILED — {errors} error(s) found.");
        1
    }
}
