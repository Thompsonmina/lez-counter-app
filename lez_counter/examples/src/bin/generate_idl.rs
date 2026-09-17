/// Generate IDL JSON for the lez_counter program.
///
/// Usage:
///   cargo run --bin generate_idl > lez_counter-idl.json

spel_framework::generate_idl!("../methods/guest/src/bin/lez_counter.rs");
