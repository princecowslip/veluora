use serde::{Deserialize, Serialize};

use crate::context::AppContext;
use crate::error::Result;

/// Backs both `GET /diagnostics/summary` and `veloura doctor`, per
/// `docs/19-local-api.md` and `docs/10-cli.md`.
#[derive(Debug, Serialize, Deserialize)]
pub struct DiagnosticsSummary {
    pub schema_version: u32,
    pub applied_migrations: i64,
    pub data_dir: String,
    pub db_path: String,
}

pub struct DiagnosticsService;

impl DiagnosticsService {
    pub fn summary(ctx: &AppContext) -> Result<DiagnosticsSummary> {
        Ok(DiagnosticsSummary {
            schema_version: 1,
            applied_migrations: ctx.db.applied_migration_count()?,
            data_dir: ctx.data_dir.display().to_string(),
            db_path: ctx.db.path().display().to_string(),
        })
    }
}
