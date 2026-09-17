//! counter_reader — reads the `lez_counter` program's on-chain state.
//!
//! Read-only by construction: no wallet, no key material, no transactions. A
//! LEZ read is JSON-RPC plus a Borsh decode, so none of the proving stack a
//! writer needs is compiled in here.
//!
//! Two layers: [`chain`] is the part worth testing and
//! touches no Logos type; everything below is the thin Logos adapter.

pub mod chain;
pub mod schema;

#[cfg(not(test))]
use serde_json::{json, Value};

#[cfg(not(test))]
use crate::chain::{CounterView, ReadError, Reader, DEFAULT_SEQUENCER_URL};

/// The program this reader is pointed at, deployed on the LEZ public testnet.
/// Configuration, never a constant baked past reach: a redeploy of the program
/// produces a NEW program_id and therefore a fresh, empty set of accounts, and
/// a reader pinned to the old one silently shows an empty world.
#[cfg(not(test))]
const DEFAULT_PROGRAM_ID: &str =
    "6beee6fdb01ea14baf382309d7a84c9dee32a60148407aae653b28cb4b4f1e55";

/// A read-only view of one `lez_counter` deployment.
///
/// Every method returns a JSON string: one shape — `{"ok":true,…}` /
/// `{"ok":false,"error":…}` — across a surface whose failure mode is always
/// "the network, or the chain, said no". It is also the convention the wallet
/// modules in this ecosystem use.
#[cfg(not(test))]
pub trait CounterReaderModule: Send + 'static {
    /// The sequencer this reader is pointed at.
    fn endpoint(&mut self) -> String;

    /// Point the reader at a different sequencer. Must be `https`, or `http`
    /// against a loopback host. Returns `{ ok, endpoint }`.
    fn set_endpoint(&mut self, url: String) -> String;

    /// The program id being read, as 64 hex characters.
    fn program_id(&mut self) -> String;

    /// Point the reader at a different deployment of the program.
    /// Returns `{ ok, programId, pda }` — the PDA changes with it.
    fn set_program_id(&mut self, hex: String) -> String;

    /// The address the counter lives at, derived from the program id alone.
    fn counter_address(&mut self) -> String;

    /// The counter's current state —
    /// `{ ok, pda, count, owner, lastUpdated, blockHeight }`.
    ///
    /// Three distinct refusals, because they need three different fixes:
    /// `not_initialised` (nobody has run `initialize` against this program),
    /// `not_ours` (the account exists but belongs to another program, or is the
    /// wrong size), and a transport error. Collapsing them into "failed" would
    /// hide which one it is.
    fn counter(&mut self) -> String;

    /// Current chain height — `{ ok, blockHeight }`. The cheapest liveness check.
    fn block_height(&mut self) -> String;

    fn on_context_ready(&mut self, _ctx: &RustModuleContext) {}
}

/// Typed events. `counter_changed` fires only when a read observes a `count`
/// different from the last one reported, so a subscriber is woken by an actual
/// on-chain change rather than by the act of polling.
#[cfg(not(test))]
pub trait CounterReaderModuleEvents {
    fn counter_changed(&self, count: i64, last_updated: i64);
}

#[cfg(not(test))]
// The builder injects the generated scaffold here. It brings its own imports
// into THIS module (std::sync::Mutex among them), so anything the author also
// imports at module scope collides -- `Mutex` must be defined only once in the
// type namespace. Qualify such types instead of importing them.
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/generated/provider_gen.rs"));

#[cfg(not(test))]
struct CounterReader {
    reader: std::sync::Mutex<Option<Reader>>,
    program_hex: std::sync::Mutex<String>,
    last_count: std::sync::Mutex<Option<u64>>,
}

#[cfg(not(test))]
impl Default for CounterReader {
    fn default() -> Self {
        Self {
            reader: std::sync::Mutex::new(Reader::new(DEFAULT_SEQUENCER_URL).ok()),
            program_hex: std::sync::Mutex::new(DEFAULT_PROGRAM_ID.to_string()),
            last_count: std::sync::Mutex::new(None),
        }
    }
}

#[cfg(not(test))]
fn err(e: impl std::fmt::Display) -> String {
    json!({ "ok": false, "error": e.to_string() }).to_string()
}

/// A refusal that also says which KIND it is, so the view can act on it.
#[cfg(not(test))]
fn err_kind(kind: &str, e: impl std::fmt::Display) -> String {
    json!({ "ok": false, "kind": kind, "error": e.to_string() }).to_string()
}

#[cfg(not(test))]
fn ok(v: Value) -> String {
    let mut obj = json!({ "ok": true });
    if let (Some(o), Some(extra)) = (obj.as_object_mut(), v.as_object()) {
        for (k, val) in extra {
            o.insert(k.clone(), val.clone());
        }
    }
    obj.to_string()
}

#[cfg(not(test))]
impl CounterReader {
    fn program_words(&self) -> Result<[u32; 8], String> {
        let hex = self
            .program_hex
            .lock()
            .map_err(|_| "module state is poisoned".to_string())?
            .clone();
        chain::program_id_from_hex(&hex).map_err(|e| e.to_string())
    }

    fn with<T>(&self, f: impl FnOnce(&Reader) -> Result<T, ReadError>) -> Result<T, ReadError> {
        let guard = self
            .reader
            .lock()
            .map_err(|_| ReadError::Transport("module state is poisoned".into()))?;
        let r = guard
            .as_ref()
            .ok_or_else(|| ReadError::BadUrl("no sequencer configured".into()))?;
        f(r)
    }

    /// Report a count, emitting only when it actually moved.
    fn observe(&self, v: &CounterView) {
        if let Ok(mut last) = self.last_count.lock() {
            if *last != Some(v.count) {
                *last = Some(v.count);
                emit_counter_changed(v.count as i64, v.last_updated as i64);
            }
        }
    }
}

#[cfg(not(test))]
impl CounterReaderModule for CounterReader {
    fn endpoint(&mut self) -> String {
        match self.reader.lock() {
            Ok(g) => g.as_ref().map(|r| r.url().to_string()).unwrap_or_default(),
            Err(_) => String::new(),
        }
    }

    fn set_endpoint(&mut self, url: String) -> String {
        match Reader::new(&url) {
            Ok(r) => {
                let endpoint = r.url().to_string();
                match self.reader.lock() {
                    Ok(mut g) => {
                        *g = Some(r);
                        // A different sequencer may be a different chain, so the
                        // remembered count is no longer a baseline.
                        if let Ok(mut c) = self.last_count.lock() {
                            *c = None;
                        }
                        ok(json!({ "endpoint": endpoint }))
                    }
                    Err(_) => err("module state is poisoned"),
                }
            }
            Err(e) => err(e),
        }
    }

    fn program_id(&mut self) -> String {
        self.program_hex.lock().map(|g| g.clone()).unwrap_or_default()
    }

    fn set_program_id(&mut self, hex: String) -> String {
        match chain::program_id_from_hex(&hex) {
            Ok(words) => {
                let pda = chain::counter_pda(&words);
                let normalised = hex.trim().trim_start_matches("0x").to_string();
                match self.program_hex.lock() {
                    Ok(mut g) => {
                        *g = normalised.clone();
                        if let Ok(mut c) = self.last_count.lock() {
                            *c = None;
                        }
                        ok(json!({ "programId": normalised, "pda": pda }))
                    }
                    Err(_) => err("module state is poisoned"),
                }
            }
            Err(e) => err(e),
        }
    }

    fn counter_address(&mut self) -> String {
        match self.program_words() {
            Ok(w) => ok(json!({ "pda": chain::counter_pda(&w) })),
            Err(e) => err(e),
        }
    }

    fn counter(&mut self) -> String {
        let words = match self.program_words() {
            Ok(w) => w,
            Err(e) => return err(e),
        };
        match self.with(|r| r.counter(&words)) {
            Ok(v) => {
                self.observe(&v);
                ok(json!({
                    "pda": v.pda,
                    "count": v.count,
                    "owner": v.owner_hex,
                    "lastUpdated": v.last_updated,
                    "blockHeight": v.block_height,
                }))
            }
            Err(e @ ReadError::NotInitialised) => err_kind("not_initialised", e),
            Err(e @ ReadError::NotOurs(_)) => err_kind("not_ours", e),
            Err(e) => err_kind("transport", e),
        }
    }

    fn block_height(&mut self) -> String {
        match self.with(|r| r.block_height()) {
            Ok(h) => ok(json!({ "blockHeight": h })),
            Err(e) => err(e),
        }
    }
}

#[cfg(not(test))]
#[no_mangle]
pub extern "Rust" fn logos_module_install() {
    install::<CounterReader>();
}
