//! Reading `lez_counter`'s state from a LEZ sequencer.
//!
//! Free of every Logos type, so `cargo test` reaches it with no SDK, no Nix and
//! no generated scaffold.
//!
//! A read on LEZ is plain JSON-RPC over HTTPS plus a Borsh decode — no zkVM, no
//! circuits, no wallet, no key material. That is the whole reason this module
//! is small.

use std::time::Duration;

use borsh::BorshDeserialize;
use crate::schema::{Counter, COUNTER_LEN, COUNTER_SEED};
use serde_json::{json, Value};

pub const DEFAULT_SEQUENCER_URL: &str = "https://testnet.lez.logos.co";
const DEFAULT_TIMEOUT_SECS: u64 = 20;

/// PDA derivation, matching `AccountId::for_public_pda` in
/// `lee/state_machine/core/src/program/mod.rs` (LEZ v0.2.4):
///
///   account_id = sha256( prefix32 || program_id32 || seed32 )
///
/// A **single** seed is used RAW, zero-padded to 32 bytes — it is NOT hashed.
/// (`compute_pda`: `let combined = if seeds.len() == 1 { *seeds[0] } else {
/// sha256(concat) }`.) Hashing it instead yields a perfectly plausible address
/// that points at nothing; the test below is against the address the chain
/// actually uses, which is the only way to catch that.
const PDA_PREFIX: &[u8; 32] = b"/LEE/v0.2/AccountId/PDA/\x00\x00\x00\x00\x00\x00\x00\x00";

#[derive(Debug)]
pub enum ReadError {
    BadUrl(String),
    BadProgramId(String),
    Transport(String),
    Rpc { code: i64, message: String },
    Malformed(String),
    /// The account exists but no program owns it — it was never initialised.
    NotInitialised,
    /// An account of the wrong size, or owned by a different program.
    NotOurs(String),
}

impl std::fmt::Display for ReadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ReadError::BadUrl(s) => write!(f, "bad sequencer URL: {s}"),
            ReadError::BadProgramId(s) => write!(f, "bad program id: {s}"),
            ReadError::Transport(s) => write!(f, "transport: {s}"),
            ReadError::Rpc { code, message } => write!(f, "sequencer error {code}: {message}"),
            ReadError::Malformed(s) => write!(f, "malformed response: {s}"),
            ReadError::NotInitialised => write!(
                f,
                "the counter account does not exist yet — run `initialize` against this program"
            ),
            ReadError::NotOurs(s) => write!(f, "account is not this program's counter: {s}"),
        }
    }
}

/// A program id as the 8 little-endian u32 words the sequencer speaks, parsed
/// from the 64-char hex ImageID the tooling prints.
pub fn program_id_from_hex(hex: &str) -> Result<[u32; 8], ReadError> {
    let hex = hex.trim().trim_start_matches("0x");
    if hex.len() != 64 || !hex.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(ReadError::BadProgramId(
            "expected 64 hex characters (the ImageID)".into(),
        ));
    }
    let mut words = [0u32; 8];
    for (i, w) in words.iter_mut().enumerate() {
        let mut b = [0u8; 4];
        for (j, byte) in b.iter_mut().enumerate() {
            *byte = u8::from_str_radix(&hex[i * 8 + j * 2..i * 8 + j * 2 + 2], 16)
                .map_err(|e| ReadError::BadProgramId(e.to_string()))?;
        }
        *w = u32::from_le_bytes(b);
    }
    Ok(words)
}

fn program_id_bytes(words: &[u32; 8]) -> [u8; 32] {
    let mut out = [0u8; 32];
    for (i, w) in words.iter().enumerate() {
        out[i * 4..i * 4 + 4].copy_from_slice(&w.to_le_bytes());
    }
    out
}

/// Derive the counter's PDA for a given program id, as base58.
///
/// Shares `COUNTER_SEED` with the program rather than repeating the literal —
/// a second spelling of that string anywhere is a bug waiting to happen.
pub fn counter_pda(program_id: &[u32; 8]) -> String {
    // Single seed: raw bytes, zero-padded to 32. Not hashed.
    let mut seed = [0u8; 32];
    let s = COUNTER_SEED.as_bytes();
    assert!(s.len() <= 32, "a PDA seed must fit in 32 bytes");
    seed[..s.len()].copy_from_slice(s);

    let mut buf = [0u8; 96];
    buf[0..32].copy_from_slice(PDA_PREFIX);
    buf[32..64].copy_from_slice(&program_id_bytes(program_id));
    buf[64..96].copy_from_slice(&seed);
    base58(&sha256(&buf))
}

/// What a reader shows: the decoded counter plus the provenance that makes it
/// trustworthy.
#[derive(Debug, Clone)]
pub struct CounterView {
    pub pda: String,
    pub count: u64,
    pub owner_hex: String,
    pub last_updated: u64,
    pub block_height: u64,
}

pub struct Reader {
    url: String,
    http: reqwest::blocking::Client,
}

impl Reader {
    pub fn new(url: &str) -> Result<Self, ReadError> {
        let parsed = url::Url::parse(url).map_err(|e| ReadError::BadUrl(e.to_string()))?;
        let loopback = matches!(
            parsed.host_str(),
            Some("localhost") | Some("127.0.0.1") | Some("::1")
        );
        // https, or plain http only against a local sequencer. A read is not
        // secret, but an answer read off the wire is an answer an attacker
        // chose, and callers treat it as chain state.
        if parsed.scheme() != "https" && !(parsed.scheme() == "http" && loopback) {
            return Err(ReadError::BadUrl("the sequencer URL must use https".into()));
        }
        let http = reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(DEFAULT_TIMEOUT_SECS))
            .build()
            .map_err(|e| ReadError::Transport(e.to_string()))?;
        Ok(Self { url: parsed.to_string(), http })
    }

    pub fn url(&self) -> &str {
        &self.url
    }

    fn call(&self, method: &str, params: Value) -> Result<Value, ReadError> {
        let body = json!({ "jsonrpc": "2.0", "id": 1, "method": method, "params": params });
        let text = self
            .http
            .post(&self.url)
            .json(&body)
            .send()
            .and_then(|r| r.text())
            .map_err(|e| ReadError::Transport(e.to_string()))?;
        parse_envelope(&text)
    }

    pub fn block_height(&self) -> Result<u64, ReadError> {
        let v = self.call("getLastBlockId", json!([]))?;
        as_u64(&v).ok_or_else(|| ReadError::Malformed(format!("getLastBlockId returned {v}")))
    }

    /// Fetch and decode the counter owned by `program_id`.
    pub fn counter(&self, program_id: &[u32; 8]) -> Result<CounterView, ReadError> {
        let pda = counter_pda(program_id);
        let v = self.call("getAccount", json!([pda]))?;
        let view = decode_account(&v, program_id, &pda)?;
        Ok(CounterView { block_height: self.block_height().unwrap_or(0), ..view })
    }
}

/// Turn a `getAccount` result into a CounterView, checking provenance first.
///
/// Split out from the transport so it can be tested against the bytes the live
/// chain actually returned.
pub fn decode_account(
    v: &Value,
    program_id: &[u32; 8],
    pda: &str,
) -> Result<CounterView, ReadError> {
    let obj = v
        .as_object()
        .ok_or_else(|| ReadError::Malformed("account is not an object".into()))?;

    let owner: Vec<u64> = obj
        .get("program_owner")
        .and_then(|o| o.as_array())
        .map(|a| a.iter().filter_map(|x| x.as_u64()).collect())
        .unwrap_or_default();

    // An unknown account comes back as a ZERO RECORD, not an error,
    // so "we got a record" proves nothing. All-zero program_owner means nobody
    // owns it — it was never initialised.
    if owner.iter().all(|w| *w == 0) {
        return Err(ReadError::NotInitialised);
    }
    // Belongs to some program, but not ours. Decoding it anyway would turn 48
    // arbitrary bytes into a plausible-looking Counter.
    let expect: Vec<u64> = program_id.iter().map(|w| *w as u64).collect();
    if owner != expect {
        return Err(ReadError::NotOurs(format!(
            "owned by program {owner:?}, expected {expect:?}"
        )));
    }

    let data: Vec<u8> = obj
        .get("data")
        .and_then(|d| d.as_array())
        .map(|a| a.iter().filter_map(|x| x.as_u64()).map(|b| b as u8).collect())
        .unwrap_or_default();

    if data.len() != COUNTER_LEN {
        return Err(ReadError::NotOurs(format!(
            "expected {COUNTER_LEN} bytes of data, got {}",
            data.len()
        )));
    }

    let c = Counter::try_from_slice(&data)
        .map_err(|e| ReadError::Malformed(format!("borsh decode failed: {e}")))?;

    Ok(CounterView {
        pda: pda.to_string(),
        count: c.count,
        owner_hex: c.owner.iter().map(|b| format!("{b:02x}")).collect(),
        last_updated: c.last_updated,
        block_height: 0,
    })
}

pub fn parse_envelope(text: &str) -> Result<Value, ReadError> {
    let v: Value = serde_json::from_str(text)
        .map_err(|e| ReadError::Malformed(format!("{e} (body: {})", truncate(text, 200))))?;
    let obj = v
        .as_object()
        .ok_or_else(|| ReadError::Malformed("not a JSON object".into()))?;
    if let Some(err) = obj.get("error").filter(|e| !e.is_null()) {
        return Err(ReadError::Rpc {
            code: err.get("code").and_then(|c| c.as_i64()).unwrap_or(0),
            message: err
                .get("message")
                .and_then(|m| m.as_str())
                .unwrap_or("unspecified error")
                .to_string(),
        });
    }
    obj.get("result")
        .cloned()
        .ok_or_else(|| ReadError::Malformed("response carries neither result nor error".into()))
}

fn as_u64(v: &Value) -> Option<u64> {
    v.as_u64().or_else(|| v.as_str().and_then(|s| s.parse().ok()))
}

fn truncate(s: &str, n: usize) -> String {
    if s.len() <= n { s.to_string() } else { format!("{}…", &s[..n]) }
}

// ── minimal sha256 + base58, so the module has no extra dependencies ────────

fn sha256(input: &[u8]) -> [u8; 32] {
    const K: [u32; 64] = [
        0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4,
        0xab1c5ed5, 0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe,
        0x9bdc06a7, 0xc19bf174, 0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f,
        0x4a7484aa, 0x5cb0a9dc, 0x76f988da, 0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7,
        0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967, 0x27b70a85, 0x2e1b2138, 0x4d2c6dfc,
        0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85, 0xa2bfe8a1, 0xa81a664b,
        0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070, 0x19a4c116,
        0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
        0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7,
        0xc67178f2,
    ];
    let mut h: [u32; 8] = [
        0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab,
        0x5be0cd19,
    ];
    let mut msg = input.to_vec();
    let bitlen = (input.len() as u64) * 8;
    msg.push(0x80);
    while msg.len() % 64 != 56 {
        msg.push(0);
    }
    msg.extend_from_slice(&bitlen.to_be_bytes());

    for chunk in msg.chunks(64) {
        let mut w = [0u32; 64];
        for i in 0..16 {
            w[i] = u32::from_be_bytes([chunk[i * 4], chunk[i * 4 + 1], chunk[i * 4 + 2], chunk[i * 4 + 3]]);
        }
        for i in 16..64 {
            let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
            let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
            w[i] = w[i - 16]
                .wrapping_add(s0)
                .wrapping_add(w[i - 7])
                .wrapping_add(s1);
        }
        let (mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut hh) =
            (h[0], h[1], h[2], h[3], h[4], h[5], h[6], h[7]);
        for i in 0..64 {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let ch = (e & f) ^ ((!e) & g);
            let t1 = hh
                .wrapping_add(s1)
                .wrapping_add(ch)
                .wrapping_add(K[i])
                .wrapping_add(w[i]);
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let maj = (a & b) ^ (a & c) ^ (b & c);
            let t2 = s0.wrapping_add(maj);
            hh = g; g = f; f = e;
            e = d.wrapping_add(t1);
            d = c; c = b; b = a;
            a = t1.wrapping_add(t2);
        }
        for (i, v) in [a, b, c, d, e, f, g, hh].iter().enumerate() {
            h[i] = h[i].wrapping_add(*v);
        }
    }
    let mut out = [0u8; 32];
    for (i, v) in h.iter().enumerate() {
        out[i * 4..i * 4 + 4].copy_from_slice(&v.to_be_bytes());
    }
    out
}

fn base58(input: &[u8]) -> String {
    const ALPHABET: &[u8] = b"123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz";
    let mut digits: Vec<u8> = vec![0];
    for &byte in input {
        let mut carry = byte as usize;
        for d in digits.iter_mut() {
            carry += (*d as usize) << 8;
            *d = (carry % 58) as u8;
            carry /= 58;
        }
        while carry > 0 {
            digits.push((carry % 58) as u8);
            carry /= 58;
        }
    }
    let leading_zeros = input.iter().take_while(|b| **b == 0).count();
    let mut out: Vec<u8> = vec![ALPHABET[0]; leading_zeros];
    out.extend(digits.iter().rev().map(|d| ALPHABET[*d as usize]));
    String::from_utf8(out).expect("base58 alphabet is ascii")
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The program deployed on the public testnet on 2026-09-15.
    const PROGRAM_HEX: &str =
        "6beee6fdb01ea14baf382309d7a84c9dee32a60148407aae653b28cb4b4f1e55";
    /// The PDA the spel CLI resolved for it — our derivation must agree.
    const EXPECTED_PDA: &str = "E44VUcNZTfHkN3GEEcm3JKvCoYXZcsD3dQAKDpdykS8e";
    const OWNER_HEX: &str =
        "4375c879e0ea5171299794ea45f164b4849383fd554aa4cc69f67b060f82b163";

    fn program_words() -> [u32; 8] {
        program_id_from_hex(PROGRAM_HEX).unwrap()
    }

    #[test]
    fn program_id_hex_round_trips_to_the_sequencer_words() {
        // The words the sequencer reports for this program.
        assert_eq!(
            program_words(),
            [4259769963, 1268850352, 153303215, 2639046871, 27669230, 2927247432, 3408411493, 1428049739]
        );
    }

    #[test]
    fn rejects_a_malformed_program_id() {
        assert!(program_id_from_hex("abc").is_err());
        assert!(program_id_from_hex(&"z".repeat(64)).is_err());
    }

    #[test]
    fn sha256_matches_known_vectors() {
        let e = sha256(b"");
        assert_eq!(
            e.iter().map(|b| format!("{b:02x}")).collect::<String>(),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
        let a = sha256(b"abc");
        assert_eq!(
            a.iter().map(|b| format!("{b:02x}")).collect::<String>(),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    /// The whole point: our PDA derivation must produce the same address the
    /// chain actually stores the counter at.
    #[test]
    fn derives_the_pda_the_chain_uses() {
        assert_eq!(counter_pda(&program_words()), EXPECTED_PDA);
    }

    fn account_json(owner: &[u32; 8], data: Vec<u8>) -> Value {
        json!({
            "program_owner": owner.iter().map(|w| *w as u64).collect::<Vec<_>>(),
            "balance": 0,
            "data": data,
            "nonce": 0
        })
    }

    fn real_counter_bytes() -> Vec<u8> {
        let mut b = Vec::new();
        b.extend_from_slice(&0u64.to_le_bytes());
        let h = format!("{OWNER_HEX:0>64}");
        for i in 0..32 {
            b.push(u8::from_str_radix(&h[i * 2..i * 2 + 2], 16).unwrap());
        }
        b.extend_from_slice(&10090u64.to_le_bytes());
        b
    }

    #[test]
    fn decodes_the_account_the_chain_returned() {
        let v = account_json(&program_words(), real_counter_bytes());
        let view = decode_account(&v, &program_words(), EXPECTED_PDA).unwrap();
        assert_eq!(view.count, 0);
        assert_eq!(view.last_updated, 10090);
        assert!(view.owner_hex.ends_with("0f82b163"));
    }

    #[test]
    fn an_unknown_account_is_not_initialised_rather_than_malformed() {
        // The zero record an unknown account returns.
        let v = account_json(&[0; 8], vec![]);
        assert!(matches!(
            decode_account(&v, &program_words(), EXPECTED_PDA),
            Err(ReadError::NotInitialised)
        ));
    }

    #[test]
    fn refuses_an_account_owned_by_another_program() {
        let other = [1u32; 8];
        let v = account_json(&other, real_counter_bytes());
        assert!(matches!(
            decode_account(&v, &program_words(), EXPECTED_PDA),
            Err(ReadError::NotOurs(_))
        ));
    }

    #[test]
    fn refuses_a_wrong_sized_account_before_decoding() {
        let v = account_json(&program_words(), vec![0u8; 40]);
        assert!(matches!(
            decode_account(&v, &program_words(), EXPECTED_PDA),
            Err(ReadError::NotOurs(_))
        ));
    }

    #[test]
    fn reads_an_rpc_error_and_a_result() {
        assert!(parse_envelope(r#"{"jsonrpc":"2.0","id":1,"result":8550}"#).is_ok());
        match parse_envelope(r#"{"jsonrpc":"2.0","id":1,"error":{"code":-32601,"message":"Method not found"}}"#) {
            Err(ReadError::Rpc { code, .. }) => assert_eq!(code, -32601),
            other => panic!("expected Rpc error, got {other:?}"),
        }
    }

    #[test]
    fn refuses_a_non_https_sequencer_unless_loopback() {
        assert!(Reader::new("http://evil.example.com").is_err());
        assert!(Reader::new("https://testnet.lez.logos.co").is_ok());
        assert!(Reader::new("http://127.0.0.1:3040").is_ok());
    }
}
