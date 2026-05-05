# RISK — cr-ikfo2u (cascade-chain label race)

## What This Patch Could Break

- `src/event.rs` (event parser: cascade-updated filter) now treats
  cascade-marked `bead.updated` events as dispatch-relevant. If Gas City
  stops including labels or cascade metadata in update payloads, a
  position-1 bead whose create event races the label write could still
  wait.
- `src/dispatch.rs` (dispatcher: duplicate side-effect guard) now skips a
  side-effecting action when an equivalent dispatch record is already in
  redb. A corrupt or unreadable archived record would surface as an error
  instead of risking a duplicate sling.
- `src/state.rs` (redb state: action lookup) scans dispatch records to
  detect duplicate start, advance, and completion actions. The table is
  small today; sustained high-volume cascade traffic could justify an
  indexed action-key table.
- `flake.nix` (Nix source: fixture fileset) no longer uses
  `craneLib.cleanCargoSource` directly. The fileset includes Cargo
  sources plus `tests/fixtures`, which should be rechecked if future
  tests need more non-Rust fixtures.

## Test Coverage

- `cargo test` passes.
- `nix develop -c cargo fmt` passes.
- `ORCHESTRATOR_BIN=target/debug/orchestrator ORCHESTRATOR_CODEX_PROVIDER_MODE=shim ... tests/scripts/orchestrator-isolated-gc-test.sh` passes in isolated GC_HOME.
- `nix flake check` passes for x86_64-linux; Nix reports aarch64-linux
  omitted on this host.
- `nix run .#integration-live` passes with `gpt-5.4-mini` in an isolated
  test city.

## Cross-Rig Effects

- No Criopolis live beads were created for testing.
- The fix is contained to `LiGoldragon/orchestrator`; it does not edit
  Gas City, Criopolis city files, prompts, or `pack.toml`.
- Runtime behavior changes only for the orchestrator daemon's event
  handling and redb dispatch-history reads.

## Second-Reviewer Focus

- Review `src/event.rs` (event parser: cascade-updated filter) first:
  it is the root-cause fix for created events arriving before labels.
- Review `src/dispatch.rs` (dispatcher: duplicate side-effect guard):
  it prevents create/update pairs from double-slinging the same bead.
- Review `tests/fixtures/real-bead-show.json` (fixture: real gc output)
  against current `gc bd show --json` output.
- Review `tests/scripts/orchestrator-isolated-gc-test.sh` (harness:
  Codex shim process name) because the shim must look like `codex` to
  avoid false restart counts.
