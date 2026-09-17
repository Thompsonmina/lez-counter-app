//! lez_counter — the on-chain guest.
//!
//! Deliberately thin: every handler forwards to `lez_counter_core` and does
//! nothing but marshal accounts. The logic is tested in that crate with no
//! zkVM in the loop; what is left here is the part only a real deploy can
//! exercise, and there is as little of it as possible.
//!
//! NOTE on the shape of the `SpelOutput::execute` call: the `#[lez_program]`
//! macro rewrites it to
//!   `execute_with_claims(&[state.account.clone(), ..], &__claims_<fn>(..), ..)`
//! — it reads the ACCOUNT PARAMETERS BY NAME at the call site, and derives each
//! account's claim from its `#[account(..)]` constraints. So the parameters
//! must be passed through directly. Hoisting one into a local
//! (`let acc = state.account.clone(); .. execute(vec![acc])`) silently drops
//! the claim, and an `init` account that is never claimed makes the whole
//! transaction fail — which the sequencer discards without recording, so it
//! looks exactly like a transaction that was never sent. Mutating
//! `state.account.data` before the call is fine: the rewrite clones after.

#![no_main]

use nssa_core::account::Data;
use spel_framework::prelude::*;

risc0_zkvm::guest::entry!(main);

#[lez_program]
mod lez_counter {
    #[allow(unused_imports)]
    use super::*;

    use lez_counter_core::{apply_increment, new_counter, Counter};

    /// Create the counter. Its address is the PDA of the literal seed
    /// `"counter"`, so this program owns exactly one and the reader can derive
    /// its address knowing only the program id.
    #[instruction]
    pub fn initialize(
        #[account(init, pda = literal("counter"))] state: AccountWithMetadata,
        #[account(signer)] owner: AccountWithMetadata,
        now: u64,
    ) -> SpelResult {
        let mut state = state;
        let counter = new_counter(*owner.account_id.value(), now);
        state.account.data = encode(&counter)?;
        Ok(SpelOutput::execute(vec![state, owner], vec![]))
    }

    /// Add `by` to the counter. Refusals come from the core crate, so the rule
    /// and its unit test live in the same place.
    #[instruction]
    pub fn increment(
        #[account(mut, pda = literal("counter"))] state: AccountWithMetadata,
        #[account(signer)] caller: AccountWithMetadata,
        by: u64,
        now: u64,
    ) -> SpelResult {
        let mut state = state;
        let mut counter = Counter::try_from_slice(state.account.data.as_ref())
            .map_err(|e| SpelError::custom(2, format!("counter is not decodable: {e}")))?;

        apply_increment(&mut counter, *caller.account_id.value(), by, now)
            .map_err(|e| SpelError::custom(3, e.to_string()))?;

        state.account.data = encode(&counter)?;
        Ok(SpelOutput::execute(vec![state, caller], vec![]))
    }

    fn encode(counter: &Counter) -> Result<Data, SpelError> {
        let bytes = borsh::to_vec(counter)
            .map_err(|e| SpelError::custom(1, format!("borsh encode failed: {e}")))?;
        Data::try_from(bytes).map_err(|_| SpelError::custom(1, "counter does not fit in an account"))
    }
}
