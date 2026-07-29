//! A real WASM sandbox proving `docs/18-plugin-system.md`'s default-deny
//! security model and resource limits, independent of there being any
//! real plugin to run yet.
//!
//! Default-deny is structural, not a checklist: [`Instance::new`] is
//! called with an **empty** imports list and no [`wasmtime::Linker`] is
//! ever built, so a module that declares any host import — network,
//! filesystem, process execution, clipboard, local API, notifications —
//! simply fails to instantiate. There is nothing to "turn off"; there
//! was never anything to turn on.
//!
//! Fuel exhaustion is the primary, deterministic proof that a resource
//! limit actually halts a runaway plugin (see the tests below) — it
//! doesn't depend on wall-clock timing, so it isn't flaky under load.
//! Epoch-based interruption is also configured as a wall-clock timeout
//! backstop (`docs/18`'s "Request timeout"), but isn't asserted by a
//! timing-dependent test, to avoid CI flakiness.

use std::time::Duration;

use thiserror::Error;
use wasmtime::{
    Config, Engine, Instance, Module, Store, StoreLimitsBuilder, Trap, WasmParams, WasmResults,
};

#[derive(Debug, Clone)]
pub struct SandboxLimits {
    pub max_memory_bytes: usize,
    pub fuel: u64,
    pub timeout: Duration,
}

impl Default for SandboxLimits {
    fn default() -> Self {
        Self {
            max_memory_bytes: 32 * 1024 * 1024,
            fuel: 10_000_000,
            timeout: Duration::from_secs(2),
        }
    }
}

#[derive(Debug, Error)]
pub enum SandboxError {
    #[error("could not compile plugin module: {0}")]
    Compile(String),
    #[error("could not instantiate plugin (unlinked import or resource limit exceeded at instantiation): {0}")]
    Instantiate(String),
    #[error("plugin export not found or has the wrong signature: {0}")]
    Export(String),
    #[error("plugin exceeded its resource limit: {0}")]
    ResourceLimitExceeded(String),
    #[error("plugin trapped: {0}")]
    Trap(String),
}

pub struct LoadedPlugin {
    module: Module,
}

/// Builds an `Engine` configured for fuel and epoch enforcement once,
/// and instantiates+calls plugin modules under those limits per call.
pub struct PluginSandbox {
    engine: Engine,
    limits: SandboxLimits,
}

impl PluginSandbox {
    pub fn new(limits: SandboxLimits) -> Result<Self, SandboxError> {
        let mut config = Config::new();
        config.consume_fuel(true);
        config.epoch_interruption(true);
        let engine = Engine::new(&config).map_err(|e| SandboxError::Compile(e.to_string()))?;
        Ok(Self { engine, limits })
    }

    /// Compiles `wat_or_wasm` — either WebAssembly text format or a
    /// compiled binary module — with no external toolchain required
    /// (wasmtime parses WAT text natively).
    pub fn load_module(&self, wat_or_wasm: &str) -> Result<LoadedPlugin, SandboxError> {
        let module = Module::new(&self.engine, wat_or_wasm)
            .map_err(|e| SandboxError::Compile(e.to_string()))?;
        Ok(LoadedPlugin { module })
    }

    /// Instantiates `plugin` fresh (a new `Store`, so one call's fuel
    /// spend or a trap never affects another) and calls its
    /// zero-import export named `export_name`, under this sandbox's
    /// memory/fuel/timeout limits.
    pub fn call_export<Params, Results>(
        &self,
        plugin: &LoadedPlugin,
        export_name: &str,
        params: Params,
    ) -> Result<Results, SandboxError>
    where
        Params: WasmParams,
        Results: WasmResults,
    {
        let store_limits = StoreLimitsBuilder::new()
            .memory_size(self.limits.max_memory_bytes)
            .trap_on_grow_failure(true)
            .build();
        let mut store = Store::new(&self.engine, store_limits);
        store.limiter(|limits| limits);
        store
            .set_fuel(self.limits.fuel)
            .map_err(|e| SandboxError::Instantiate(e.to_string()))?;
        store.set_epoch_deadline(1);

        // A wall-clock backstop: increments the epoch after `timeout`
        // so a plugin that's cheap on fuel but somehow still runs long
        // (e.g. a host-call-free busy loop with unusually low
        // per-instruction fuel cost) is still eventually interrupted.
        // Detached deliberately — incrementing the epoch after this
        // call has already returned is harmless.
        let engine_for_deadline = self.engine.clone();
        let timeout = self.limits.timeout;
        std::thread::spawn(move || {
            std::thread::sleep(timeout);
            engine_for_deadline.increment_epoch();
        });

        // Zero imports, always — see the module doc comment. A module
        // that declares any import fails right here.
        let instance = Instance::new(&mut store, &plugin.module, &[])
            .map_err(|e| SandboxError::Instantiate(e.to_string()))?;
        let func = instance
            .get_typed_func::<Params, Results>(&mut store, export_name)
            .map_err(|e| SandboxError::Export(e.to_string()))?;

        func.call(&mut store, params)
            .map_err(|err| classify_call_error(&err))
    }
}

fn classify_call_error(err: &wasmtime::Error) -> SandboxError {
    if let Some(trap) = err.downcast_ref::<Trap>() {
        return match trap {
            Trap::OutOfFuel | Trap::Interrupt => {
                SandboxError::ResourceLimitExceeded(trap.to_string())
            }
            other => SandboxError::Trap(other.to_string()),
        };
    }
    SandboxError::Trap(err.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    const WELL_BEHAVED: &str = include_str!("../fixtures/well_behaved.wat");
    const UNLINKED_IMPORT: &str = include_str!("../fixtures/unlinked_import.wat");
    const INFINITE_LOOP: &str = include_str!("../fixtures/infinite_loop.wat");
    const MEMORY_HOG: &str = include_str!("../fixtures/memory_hog.wat");

    #[test]
    fn a_well_behaved_plugin_runs_and_returns_its_value() {
        let sandbox = PluginSandbox::new(SandboxLimits::default()).unwrap();
        let plugin = sandbox.load_module(WELL_BEHAVED).unwrap();
        let result: i32 = sandbox.call_export(&plugin, "identify", ()).unwrap();
        assert_eq!(result, 42);
    }

    #[test]
    fn a_plugin_requesting_an_unlinked_host_import_fails_to_instantiate() {
        let sandbox = PluginSandbox::new(SandboxLimits::default()).unwrap();
        let plugin = sandbox.load_module(UNLINKED_IMPORT).unwrap();
        let err = sandbox
            .call_export::<(), i32>(&plugin, "identify", ())
            .unwrap_err();
        assert!(matches!(err, SandboxError::Instantiate(_)));
    }

    #[test]
    fn an_infinite_loop_is_halted_by_running_out_of_fuel() {
        // A tiny fuel budget makes this deterministic and fast — no
        // reliance on the wall-clock timeout thread.
        let limits = SandboxLimits {
            fuel: 10_000,
            ..SandboxLimits::default()
        };
        let sandbox = PluginSandbox::new(limits).unwrap();
        let plugin = sandbox.load_module(INFINITE_LOOP).unwrap();
        let err = sandbox
            .call_export::<(), ()>(&plugin, "spin", ())
            .unwrap_err();
        assert!(matches!(err, SandboxError::ResourceLimitExceeded(_)));
    }

    #[test]
    fn a_plugin_that_over_grows_memory_is_rejected() {
        let limits = SandboxLimits {
            max_memory_bytes: 64 * 1024, // one page
            ..SandboxLimits::default()
        };
        let sandbox = PluginSandbox::new(limits).unwrap();
        let plugin = sandbox.load_module(MEMORY_HOG).unwrap();
        let err = sandbox
            .call_export::<(), i32>(&plugin, "grow_a_lot", ())
            .unwrap_err();
        assert!(matches!(err, SandboxError::Trap(_)));
    }
}
