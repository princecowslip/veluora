use std::path::PathBuf;

use plugin_host::{LocalPluginRegistry, PluginManifest};

use super::{print_error_message, print_json};
use crate::cli_args::{OutputFormat, PluginStatusArg};
use crate::exit_code::ExitCode;

fn resolve_registry_path(format: OutputFormat, quiet: bool) -> Result<PathBuf, ExitCode> {
    let data_dir = plugin_host::resolve_data_dir().ok_or_else(|| {
        print_error_message(
            format,
            quiet,
            "could not resolve a data directory for this platform",
        );
        ExitCode::ConfigurationFailure
    })?;
    Ok(plugin_host::registry_path(&data_dir))
}

pub fn validate(format: OutputFormat, quiet: bool, manifest_path: PathBuf) -> ExitCode {
    let manifest = match PluginManifest::load(&manifest_path) {
        Ok(m) => m,
        Err(e) => {
            print_error_message(format, quiet, &format!("could not load manifest: {e}"));
            return ExitCode::InvalidArguments;
        }
    };
    let issues = manifest.validate();
    let summary = manifest.permissions.summary_lines();

    match format {
        OutputFormat::Json | OutputFormat::Jsonl => print_json(&serde_json::json!({
            "schema_version": 1,
            "id": manifest.id,
            "name": manifest.name,
            "publisher": manifest.publisher,
            "valid": issues.is_empty(),
            "issues": issues,
            "permissions_summary": summary,
        })),
        OutputFormat::Text | OutputFormat::Table => {
            if !quiet {
                println!("{} ({})", manifest.name, manifest.id);
                println!("  publisher: {}", manifest.publisher);
                println!("  permissions:");
                for line in &summary {
                    println!("    {line}");
                }
                if issues.is_empty() {
                    println!("  valid: yes");
                } else {
                    println!("  valid: no");
                    for issue in &issues {
                        println!("    - {}: {}", issue.field, issue.message);
                    }
                }
            }
        }
    }

    if issues.is_empty() {
        ExitCode::Success
    } else {
        ExitCode::InvalidArguments
    }
}

pub fn registry_list(format: OutputFormat, quiet: bool) -> ExitCode {
    let path = match resolve_registry_path(format, quiet) {
        Ok(p) => p,
        Err(code) => return code,
    };
    let registry = match LocalPluginRegistry::load(&path) {
        Ok(r) => r,
        Err(e) => {
            print_error_message(format, quiet, &format!("could not load registry: {e}"));
            return ExitCode::ConfigurationFailure;
        }
    };

    match format {
        OutputFormat::Json | OutputFormat::Jsonl => print_json(&registry.list()),
        OutputFormat::Text | OutputFormat::Table => {
            if !quiet {
                if registry.list().is_empty() {
                    println!("(no plugins registered)");
                }
                for entry in registry.list() {
                    println!(
                        "{} [{:?}] — {}",
                        entry.manifest.id, entry.status, entry.manifest.name
                    );
                }
            }
        }
    }
    ExitCode::Success
}

pub fn registry_add(
    format: OutputFormat,
    quiet: bool,
    manifest_path: PathBuf,
    status: PluginStatusArg,
) -> ExitCode {
    let path = match resolve_registry_path(format, quiet) {
        Ok(p) => p,
        Err(code) => return code,
    };
    let manifest = match PluginManifest::load(&manifest_path) {
        Ok(m) => m,
        Err(e) => {
            print_error_message(format, quiet, &format!("could not load manifest: {e}"));
            return ExitCode::InvalidArguments;
        }
    };
    let mut registry = match LocalPluginRegistry::load(&path) {
        Ok(r) => r,
        Err(e) => {
            print_error_message(format, quiet, &format!("could not load registry: {e}"));
            return ExitCode::ConfigurationFailure;
        }
    };

    let id = manifest.id.clone();
    if let Err(e) = registry.add_entry(manifest, status.into()) {
        print_error_message(format, quiet, &e.to_string());
        return ExitCode::InvalidArguments;
    }
    if let Err(e) = registry.save(&path) {
        print_error_message(format, quiet, &format!("could not save registry: {e}"));
        return ExitCode::ConfigurationFailure;
    }

    match format {
        OutputFormat::Json | OutputFormat::Jsonl => print_json(&serde_json::json!({
            "schema_version": 1,
            "ok": true,
            "id": id,
        })),
        OutputFormat::Text | OutputFormat::Table => {
            if !quiet {
                println!("added {id} to the local registry");
            }
        }
    }
    ExitCode::Success
}

pub fn registry_set_status(
    format: OutputFormat,
    quiet: bool,
    id: String,
    status: PluginStatusArg,
) -> ExitCode {
    let path = match resolve_registry_path(format, quiet) {
        Ok(p) => p,
        Err(code) => return code,
    };
    let mut registry = match LocalPluginRegistry::load(&path) {
        Ok(r) => r,
        Err(e) => {
            print_error_message(format, quiet, &format!("could not load registry: {e}"));
            return ExitCode::ConfigurationFailure;
        }
    };

    if let Err(e) = registry.set_status(&id, status.into()) {
        print_error_message(format, quiet, &e.to_string());
        return ExitCode::NotFound;
    }
    if let Err(e) = registry.save(&path) {
        print_error_message(format, quiet, &format!("could not save registry: {e}"));
        return ExitCode::ConfigurationFailure;
    }

    match format {
        OutputFormat::Json | OutputFormat::Jsonl => {
            print_json(&serde_json::json!({ "schema_version": 1, "ok": true }))
        }
        OutputFormat::Text | OutputFormat::Table => {
            if !quiet {
                println!("updated status for {id}");
            }
        }
    }
    ExitCode::Success
}
