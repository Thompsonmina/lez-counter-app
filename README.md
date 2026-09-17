# lez-counter-app

Two Logos Basecamp modules that read a SPEL program's state off the LEZ public
testnet and put it on screen. Built to learn the path end to end, so the
interesting part is the seam, not the app: the counter it reads holds a single
number.

- **`counter_reader`** — Rust `cdylib` core. JSON-RPC to the sequencer, PDA
  derivation, Borsh decode. No Logos types in the part worth testing, so
  `cargo test` runs with no Nix.
- **`counter_ui`** — `ui_qml` view, **QML-only**: no `.rep`, no C++ backend, no
  code generator. Between the two modules there is **no hand-written C++ at
  all**.
- **`lez_counter`** — the SPEL program the other two read: the thing that
  actually lives on chain.

Read-only by construction: no wallet, no key material, no transactions.

## Versions

Everything here is one release set.

| | |
|---|---|
| logos-module-builder | **0.2.6** |
| release set | **v0.2.1-r.2** |
| logos-protocol | **0.2** |
| Basecamp | **0.2.2** (what it was verified on); 0.2.3 is same-line |
| logoscore / lgpm | 0.2.2 / 1.0.0-dev |
| Qt | 6.9.2 |
| LEZ sequencer | **v0.2.4** (`testnet.lez.logos.co`) |
| `wallet` | v0.2.4, commit `47eba256479f6f785acbd138834340703cd03401` |

**Do not load these on Basecamp 0.3.0-rc.2.** `logos-protocol` goes 0.2 → 0.9 in
that wave, and a same-MAJOR rule governs whether a `.lgx` loads at all. That is a
separate migration, not an upgrade.

## The chain side

```
program_id   6beee6fdb01ea14baf382309d7a84c9dee32a60148407aae653b28cb4b4f1e55
counter PDA  E44VUcNZTfHkN3GEEcm3JKvCoYXZcsD3dQAKDpdykS8e
sequencer    https://testnet.lez.logos.co
```

The PDA is derived from the program id, so **a redeploy changes the program id
and therefore the address**, orphaning the old account. The reader treats both
as configuration (`set_program_id`, `set_endpoint`) rather than constants.

The program itself is in [`lez_counter/`](lez_counter/) — a SPEL program with
two instructions, `initialize` and `increment`. Note that its guest binary is a
build artifact and is not committed, so building it yourself produces a
different program id unless the toolchain matches exactly.

## Build

```bash
# core
cd counter_reader
nix build "github:logos-co/logos-module-builder/0.2.6#rust-sdk-src" -o logos-rust-sdk-src
(cd rust-lib && cargo generate-lockfile)
git add -A                      # nix only sees git-tracked files
nix build .#lgx -o result-lgx   # ~4.5 MB

# view — no compiler in the loop
cd ../counter_ui
git add -A
nix build .#lgx -o result-lgx   # ~9 KB
```

`nix build .#lidl -o result-lidl` in `counter_reader` prints the contract the
builder derived from the Rust trait — there is no hand-written `.lidl`.

## Run

**Headless**, proves the core with no display:

```bash
export LOGOSCORE_CONFIG_DIR=$PWD/run/logoscore-cfg
logoscore -D -m "$PWD/run/modules" &
logoscore load-module counter_reader
logoscore call counter_reader counter
logoscore stop
```

**Standalone**, the fast edit loop — it takes a plugin *directory*, so it can
render exactly what Basecamp has installed:

```bash
cd counter_ui && nix run . -- --modules-dir ../run/modules --load counter_reader
```

**Basecamp**, the real host:

```bash
lgpm --modules-dir    ./bcdata/modules \
     install --file counter_reader/result-lgx/*.lgx --allow-unsigned
lgpm --ui-plugins-dir ./bcdata/plugins \
     install --file counter_ui/result-lgx/*.lgx --allow-unsigned
LogosBasecamp --user-dir "$PWD/bcdata"
```

`counter_ui` declares `dependencies: ["counter_reader"]`, so the host loads the
core by itself — on a fresh user-dir, with no interaction.
