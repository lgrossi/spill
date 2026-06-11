use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result};

const REPO_OWNER: &str = "lgrossi";
const REPO_NAME: &str = "spill";
const CHECK_INTERVAL_SECS: u64 = 24 * 60 * 60;

/// Explicit `spill update`: fetch and apply the latest release now.
pub fn update_now() -> Result<()> {
    let status = self_update::backends::github::Update::configure()
        .repo_owner(REPO_OWNER)
        .repo_name(REPO_NAME)
        .bin_name(REPO_NAME)
        .current_version(env!("CARGO_PKG_VERSION"))
        .show_download_progress(true)
        .build()
        .context("configure updater")?
        .update()
        .context("update")?;
    if status.updated() {
        println!("spill: updated to {}", status.version());
    } else {
        println!("spill: already up to date ({})", status.version());
    }
    Ok(())
}

/// Throttled, best-effort check run before other commands. A downloaded update
/// replaces the on-disk binary and takes effect on the next invocation; errors
/// (offline, no release, rate limit) are swallowed so commands never block.
pub fn maybe_auto_update() {
    if !throttle_elapsed() {
        return;
    }
    record_check();
    if let Ok(true) = try_update_quiet() {
        eprintln!("spill: a newer version was installed; it takes effect on the next run.");
    }
}

fn try_update_quiet() -> Result<bool> {
    let status = self_update::backends::github::Update::configure()
        .repo_owner(REPO_OWNER)
        .repo_name(REPO_NAME)
        .bin_name(REPO_NAME)
        .current_version(env!("CARGO_PKG_VERSION"))
        .no_confirm(true)
        .show_download_progress(false)
        .show_output(false)
        .build()?
        .update()?;
    Ok(status.updated())
}

fn check_marker() -> Option<std::path::PathBuf> {
    Some(dirs::config_dir()?.join("spill").join("last_update_check"))
}

fn throttle_elapsed() -> bool {
    let Some(path) = check_marker() else {
        return false;
    };
    let Ok(contents) = std::fs::read_to_string(&path) else {
        return true;
    };
    let last: u64 = contents.trim().parse().unwrap_or(0);
    now().saturating_sub(last) >= CHECK_INTERVAL_SECS
}

fn record_check() {
    if let Some(path) = check_marker() {
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let _ = std::fs::write(&path, now().to_string());
    }
}

fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}
