//! Auto-update functionality for Perch

use std::time::Duration;

/// Current version from Cargo.toml
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Result of a version check
#[derive(Debug, Clone)]
pub enum VersionCheck {
    /// Running the latest version
    UpToDate,
    /// A newer version is available
    UpdateAvailable { latest: String, current: String },
    /// Could not check (network error, etc.)
    CheckFailed(String),
}

/// Check for updates using crates.io API
pub fn check_for_updates_crates_io() -> VersionCheck {
    check_for_updates_crates_io_timeout(Duration::from_secs(5))
}

/// Check for updates using crates.io API with custom timeout
pub fn check_for_updates_crates_io_timeout(timeout: Duration) -> VersionCheck {
    let url = "https://crates.io/api/v1/crates/perch";

    let agent = ureq::AgentBuilder::new().timeout(timeout).build();

    let result = agent
        .get(url)
        .set("User-Agent", &format!("perch/{}", VERSION))
        .call();

    match result {
        Ok(response) => match response.into_json::<serde_json::Value>() {
            Ok(json) => {
                if let Some(latest_str) = json
                    .get("crate")
                    .and_then(|c| c.get("max_version"))
                    .and_then(|v| v.as_str())
                {
                    let latest = latest_str.to_string();
                    let current = VERSION.to_string();

                    if version_is_newer(&latest, &current) {
                        VersionCheck::UpdateAvailable { latest, current }
                    } else {
                        VersionCheck::UpToDate
                    }
                } else {
                    VersionCheck::CheckFailed("Could not parse crates.io response".to_string())
                }
            }
            Err(e) => VersionCheck::CheckFailed(format!("Failed to parse response: {}", e)),
        },
        Err(e) => VersionCheck::CheckFailed(format!("Request failed: {}", e)),
    }
}

/// Compare semver versions, returns true if `latest` is newer than `current`
fn version_is_newer(latest: &str, current: &str) -> bool {
    let parse = |v: &str| -> Vec<u32> { v.split('.').filter_map(|s| s.parse().ok()).collect() };

    let latest_parts = parse(latest);
    let current_parts = parse(current);

    for i in 0..3 {
        let l = latest_parts.get(i).copied().unwrap_or(0);
        let c = current_parts.get(i).copied().unwrap_or(0);
        if l > c {
            return true;
        }
        if l < c {
            return false;
        }
    }
    false
}

/// Detected package manager for installation
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PackageManager {
    Cargo,
    Homebrew { formula: String },
}

impl PackageManager {
    /// Get display name
    pub fn name(&self) -> &'static str {
        match self {
            PackageManager::Cargo => "cargo",
            PackageManager::Homebrew { .. } => "brew",
        }
    }

    /// Get the update command
    pub fn update_command(&self) -> String {
        match self {
            PackageManager::Cargo => "cargo install perch".to_string(),
            PackageManager::Homebrew { formula } => format!("brew upgrade {}", formula),
        }
    }
}

/// Detect how perch was installed
pub fn detect_package_manager() -> PackageManager {
    if let Ok(exe_path) = std::env::current_exe() {
        let exe_str = exe_path.to_string_lossy();

        if exe_str.contains("/Cellar/") || exe_str.contains("/homebrew/") {
            if let Ok(output) = std::process::Command::new("brew")
                .args(["info", "--json=v2", "perch"])
                .output()
                && output.status.success()
                && let Ok(json) = serde_json::from_slice::<serde_json::Value>(&output.stdout)
                && let Some(formulae) = json.get("formulae").and_then(|f| f.as_array())
                && let Some(formula) = formulae.first()
                && let Some(full_name) = formula.get("full_name").and_then(|n| n.as_str())
            {
                return PackageManager::Homebrew {
                    formula: full_name.to_string(),
                };
            }
            return PackageManager::Homebrew {
                formula: "perch".to_string(),
            };
        }
    }

    PackageManager::Cargo
}

/// Run the update command and return the result
pub fn run_update(pm: &PackageManager) -> Result<(), String> {
    use std::process::Stdio;

    match pm {
        PackageManager::Cargo => {
            match std::process::Command::new("cargo")
                .args(["install", "perch"])
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
            {
                Ok(status) if status.success() => Ok(()),
                Ok(status) => Err(format!("Update failed with status: {}", status)),
                Err(e) => Err(format!("Failed to run cargo: {}", e)),
            }
        }
        PackageManager::Homebrew { formula } => {
            let _ = std::process::Command::new("brew")
                .args(["update"])
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status();

            match std::process::Command::new("brew")
                .args(["upgrade", formula])
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
            {
                Ok(status) if status.success() => Ok(()),
                Ok(_) => {
                    match std::process::Command::new("brew")
                        .args(["reinstall", formula])
                        .stdout(Stdio::null())
                        .stderr(Stdio::null())
                        .status()
                    {
                        Ok(status) if status.success() => Ok(()),
                        Ok(status) => Err(format!("Update failed with status: {}", status)),
                        Err(e) => Err(format!("Failed to run brew: {}", e)),
                    }
                }
                Err(e) => Err(format!("Failed to run brew: {}", e)),
            }
        }
    }
}
