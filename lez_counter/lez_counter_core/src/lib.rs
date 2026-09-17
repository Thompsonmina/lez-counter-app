//! lez_counter — shared types and handler logic.
//!
//! This crate is the **schema**. It is compiled three times over: into the
//! risc0 guest that runs on chain, into the host-side CLI, and (the whole point
//! of this exercise) into the Logos module that reads the account back. All
//! three therefore agree on the Borsh layout by construction rather than by
//! comment — which matters, because Borsh has no field names on the wire and no
//! version tag: field ORDER is the encoding.
//!
//! Nothing here knows about risc0, the sequencer, or Logos. The handlers are
//! plain functions over plain values, so `cargo test -p lez_counter_core` runs
//! them with no chain and no zkVM in the loop.

use borsh::{BorshDeserialize, BorshSerialize};
use spel_framework_macros::account_type;

/// The PDA seed for the single counter account this program owns.
///
/// Shared so the reader derives the same address the program writes to. A
/// second spelling of this string anywhere is a bug waiting to happen.
pub const COUNTER_SEED: &str = "counter";

/// The one account this program owns.
///
/// `#[account_type]` is a marker consumed by `spel generate-idl`; it is a
/// pass-through at compile time. The generator scans path-dependency crates for
/// it, which is why this can live here rather than being trapped in the guest.
#[account_type]
#[derive(BorshSerialize, BorshDeserialize, Debug, Clone, PartialEq, Eq)]
pub struct Counter {
    /// Times `increment` has successfully applied.
    pub count: u64,
    /// The account that ran `initialize`. 32 raw bytes, as account ids are.
    pub owner: [u8; 32],
    /// Block height reported by the caller on the last successful write.
    ///
    /// There is no clock on chain, so this is caller-supplied and is a height
    /// the sequencer can sanity-check — never a trusted timestamp.
    pub last_updated: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CounterError {
    /// `by` was zero: a write that changes nothing should not cost a
    /// transaction, and silently accepting it hides a caller's bug.
    ZeroIncrement,
    /// `count` would exceed u64. Saturating here would silently freeze the
    /// counter at the top, which reads identically to "nobody is calling".
    Overflow,
    /// Someone other than the recorded owner tried to write.
    NotOwner,
    /// `now` went backwards. Heights are monotonic; a lower one means the
    /// caller is confused or replaying, and neither should be recorded.
    StaleHeight { seen: u64, last: u64 },
}

impl core::fmt::Display for CounterError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            CounterError::ZeroIncrement => write!(f, "increment must be greater than zero"),
            CounterError::Overflow => write!(f, "counter would overflow u64"),
            CounterError::NotOwner => write!(f, "only the owner may increment this counter"),
            CounterError::StaleHeight { seen, last } => {
                write!(f, "height {seen} is not after the last write at {last}")
            }
        }
    }
}

/// The initial state written by `initialize`.
pub fn new_counter(owner: [u8; 32], now: u64) -> Counter {
    Counter { count: 0, owner, last_updated: now }
}

/// Apply an increment, or explain why not.
///
/// Takes `&mut` and returns unit rather than returning a new Counter: the
/// caller's account is the thing being mutated, and handing back a fresh value
/// invites writing it to the wrong account.
pub fn apply_increment(
    state: &mut Counter,
    caller: [u8; 32],
    by: u64,
    now: u64,
) -> Result<(), CounterError> {
    if state.owner != caller {
        return Err(CounterError::NotOwner);
    }
    if by == 0 {
        return Err(CounterError::ZeroIncrement);
    }
    if now < state.last_updated {
        return Err(CounterError::StaleHeight { seen: now, last: state.last_updated });
    }
    let next = state.count.checked_add(by).ok_or(CounterError::Overflow)?;
    state.count = next;
    state.last_updated = now;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const ALICE: [u8; 32] = [1u8; 32];
    const BOB: [u8; 32] = [2u8; 32];

    #[test]
    fn a_new_counter_starts_at_zero_and_records_its_owner() {
        let c = new_counter(ALICE, 100);
        assert_eq!(c.count, 0);
        assert_eq!(c.owner, ALICE);
        assert_eq!(c.last_updated, 100);
    }

    #[test]
    fn increment_advances_count_and_height() {
        let mut c = new_counter(ALICE, 100);
        apply_increment(&mut c, ALICE, 5, 101).unwrap();
        assert_eq!(c.count, 5);
        assert_eq!(c.last_updated, 101);
    }

    #[test]
    fn only_the_owner_may_increment() {
        let mut c = new_counter(ALICE, 100);
        assert_eq!(apply_increment(&mut c, BOB, 1, 101), Err(CounterError::NotOwner));
        // and the state is untouched
        assert_eq!(c.count, 0);
        assert_eq!(c.last_updated, 100);
    }

    #[test]
    fn a_zero_increment_is_refused_rather_than_silently_accepted() {
        let mut c = new_counter(ALICE, 100);
        assert_eq!(apply_increment(&mut c, ALICE, 0, 101), Err(CounterError::ZeroIncrement));
        assert_eq!(c.last_updated, 100);
    }

    #[test]
    fn overflow_is_refused_rather_than_saturating() {
        let mut c = new_counter(ALICE, 100);
        c.count = u64::MAX;
        assert_eq!(apply_increment(&mut c, ALICE, 1, 101), Err(CounterError::Overflow));
        assert_eq!(c.count, u64::MAX);
    }

    #[test]
    fn a_height_that_goes_backwards_is_refused() {
        let mut c = new_counter(ALICE, 100);
        apply_increment(&mut c, ALICE, 1, 105).unwrap();
        assert_eq!(
            apply_increment(&mut c, ALICE, 1, 104),
            Err(CounterError::StaleHeight { seen: 104, last: 105 })
        );
        // the same height is allowed: two writes can land in one block
        apply_increment(&mut c, ALICE, 1, 105).unwrap();
        assert_eq!(c.count, 2);
    }

    /// The layout the reader will decode. If this test changes, every account
    /// already on chain decodes differently — see the module docs.
    #[test]
    fn borsh_layout_is_count_then_owner_then_height() {
        let c = Counter { count: 1, owner: [0xAB; 32], last_updated: 2 };
        let bytes = borsh::to_vec(&c).unwrap();
        assert_eq!(bytes.len(), 8 + 32 + 8);
        assert_eq!(&bytes[0..8], &1u64.to_le_bytes());
        assert_eq!(&bytes[8..40], &[0xAB; 32]);
        assert_eq!(&bytes[40..48], &2u64.to_le_bytes());
        assert_eq!(Counter::try_from_slice(&bytes).unwrap(), c);
    }
}
