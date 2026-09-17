//! The on-chain schema of the `lez_counter` program, for readers.
//!
//! Borsh has no field names on the wire and no version tag: **field ORDER is the
//! encoding**. This declaration and the program's must not drift, which is why
//! the layout is pinned by a test below rather than by a comment.
//!
//! WHY THIS IS A MODULE AND NOT A SHARED CRATE. The builder stages ONLY the
//! crate named by `codegen.rust.crate`, plus the SDK:
//!     cp -r ${rustCrateDir} $out/rust-lib
//!     cp -r ${rustSdk}      $out/logos-rust-sdk-src
//! (mkLogosModule.nix). So a Logos Rust module's crate can hold no local path
//! dependency but the SDK — not even a sibling crate inside the same module
//! directory. A **git** dependency does work (`allowBuiltinFetchGit = true`,
//! vendored from Cargo.lock), so the real fix is publishing the program repo
//! and depending on `lez_counter_core` by git. Until then this is a copy, and
//! the layout test below is the only thing standing between it and drift.

use borsh::{BorshDeserialize, BorshSerialize};

/// The PDA seed of the single counter account the program owns.
pub const COUNTER_SEED: &str = "counter";

/// The one account `lez_counter` owns. 48 bytes: 8 + 32 + 8.
#[derive(BorshSerialize, BorshDeserialize, Debug, Clone, PartialEq, Eq)]
pub struct Counter {
    /// Times `increment` has successfully applied.
    pub count: u64,
    /// The account that ran `initialize`. 32 raw bytes, as account ids are.
    pub owner: [u8; 32],
    /// Block height reported by the caller on the last successful write.
    pub last_updated: u64,
}

/// The exact encoded size. A reader can reject a wrong-sized account before
/// decoding rather than interpreting arbitrary bytes as a plausible Counter.
pub const COUNTER_LEN: usize = 8 + 32 + 8;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn borsh_layout_is_count_then_owner_then_height() {
        let c = Counter { count: 1, owner: [0xAB; 32], last_updated: 2 };
        let bytes = borsh::to_vec(&c).unwrap();
        assert_eq!(bytes.len(), COUNTER_LEN);
        assert_eq!(&bytes[0..8], &1u64.to_le_bytes());
        assert_eq!(&bytes[8..40], &[0xAB; 32]);
        assert_eq!(&bytes[40..48], &2u64.to_le_bytes());
        assert_eq!(Counter::try_from_slice(&bytes).unwrap(), c);
    }

    /// The bytes the live testnet actually returned for the deployed program's
    /// counter PDA (E44VUcNZTfHkN3GEEcm3JKvCoYXZcsD3dQAKDpdykS8e), so a drift in
    /// this struct fails here rather than in a running panel.
    #[test]
    fn decodes_the_real_on_chain_account() {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&0u64.to_le_bytes());                 // count = 0
        bytes.extend_from_slice(&hex32("4375c879e0ea5171299794ea45f164b4849383fd554aa4cc69f67b060f82b163"));
        bytes.extend_from_slice(&10090u64.to_le_bytes());             // last_updated
        let c = Counter::try_from_slice(&bytes).unwrap();
        assert_eq!(c.count, 0);
        assert_eq!(c.last_updated, 10090);
    }

    fn hex32(s: &str) -> [u8; 32] {
        let s = format!("{s:0>64}");
        let mut out = [0u8; 32];
        for i in 0..32 {
            out[i] = u8::from_str_radix(&s[i * 2..i * 2 + 2], 16).unwrap();
        }
        out
    }
}
