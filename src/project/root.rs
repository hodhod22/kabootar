//! Project root resolution — cwd or `KABOOTAR_PROJECT_ROOT`.

use std::path::PathBuf;

/// Resolve the active Kabootar project directory.
pub fn project_root() -> Result<PathBuf, String> {
    if let Ok(raw) = std::env::var("KABOOTAR_PROJECT_ROOT") {
        let path = PathBuf::from(raw);
        if path.is_dir() {
            return Ok(path);
        }
        return Err(format!(
            "KABOOTAR_PROJECT_ROOT is not a directory: {}",
            path.display()
        ));
    }
    std::env::current_dir().map_err(|e| format!("Failed to get cwd: {e}"))
}
