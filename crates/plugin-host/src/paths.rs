//! Where plugin governance files live on disk — resolved independently
//! of `application::AppContext` (this crate deliberately doesn't
//! depend on `database`/`application`; see the crate doc comment) but
//! matching its exact data-dir resolution, so both land in the same
//! place.

use std::path::{Path, PathBuf};

use directories::ProjectDirs;

/// Mirrors `application::AppContext::open_default`'s resolution
/// exactly (`directories::ProjectDirs::from("", "", "veloura")`).
pub fn resolve_data_dir() -> Option<PathBuf> {
    Some(
        ProjectDirs::from("", "", "veloura")?
            .data_dir()
            .to_path_buf(),
    )
}

pub fn registry_path(data_dir: &Path) -> PathBuf {
    data_dir.join("plugins").join("registry.json")
}
