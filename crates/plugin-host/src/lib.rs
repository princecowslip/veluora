//! Plugin manifest parsing/validation, local registry governance, and a
//! WASM sandbox proving default-deny capability enforcement — the parts
//! of `docs/18-plugin-system.md` that don't require a real connector to
//! exist (Milestone F, skipped, isn't built). See the Milestone H plan
//! for exactly what is and isn't in scope here.

pub mod manifest;
pub mod paths;
pub mod permissions;
pub mod registry;
pub mod sandbox;

pub use manifest::{ManifestError, PluginManifest, ValidationIssue};
pub use paths::{registry_path, resolve_data_dir};
pub use permissions::{
    CredentialPermission, FilesystemPermission, LocalApiPermission, LocalApiScope,
    NetworkPermission, PluginPermissions,
};
pub use registry::{LocalPluginRegistry, PluginRegistryEntry, PluginStatus, RegistryError};
pub use sandbox::{LoadedPlugin, PluginSandbox, SandboxError, SandboxLimits};
