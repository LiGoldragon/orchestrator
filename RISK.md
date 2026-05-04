# RISK — cr-qp7bha (orchestrator Rust repository)

## What This Patch Could Break

- A live cascade could stall if `gc events --after` changes its JSON
  line shape. The parser currently accepts bead-created and bead-closed
  lines and skips non-bead lines without `subject`; see
  `src/event.rs:79` (event batch parser).
- A malformed cascade bead now fails the daemon rather than guessing.
  Invalid `cascade_position`, missing `gc.routed_to` on a dispatched
  bead, or a `cascade_next` bead without a target are surfaced as errors;
  see `src/bead.rs:82` (position parser) and `src/dispatch.rs:124`
  (closed-bead transition).
- The daemon reads every bead-created/bead-closed event through
  `gc bd show --json` before filtering by labels, matching the bead's
  acceptance criteria. High event volume could make this noisy until gc
  exposes a label-filtered event stream; see `src/gc.rs:42`
  (bead lookup wrapper) and `src/bead.rs:47` (label filter).
- Cross-prefix event subjects can appear in the city stream. Missing
  local bead records are skipped and recorded, not fatal; see
  `src/dispatch.rs:171` (missing-bead skip path).

## Test Coverage

- `cargo test` passes: event parsing/sorting, cascade metadata parsing,
  dispatch decisions, order-tracking skip, redb cursor persistence, and
  rkyv dispatch-record round trip.
- `nix develop -c cargo fmt --check` passes.
- `nix flake check` passes for `x86_64-linux`; Nix reports
  `aarch64-linux` omitted as incompatible on this host.
- Live smoke run passed:
  `target/debug/orchestrator --city /home/li/Criopolis --state target/orchestrator-smoke-3.redb --once`.
  It initialized a local ignored state DB, skipped ordinary beads, and
  did not mutate Criopolis.

## Cross-Rig Effects

- Repository created and pushed:
  `LiGoldragon/orchestrator` (GitHub: new Rust daemon repo).
- City files were not edited. Mayor still owns `pack.toml`,
  prompts, and the eventual lifecycle wiring; see
  `ARCHITECTURE.md:66` (runtime wiring boundary).
- Default state path is `<city>/.gc/orchestrator.redb`; mayor should
  confirm this is the right live path before registering the daemon.

## Deployment Story

The binary is lifecycle-neutral:

```sh
orchestrator --city /home/li/Criopolis
```

The clean first wiring is a custom Gas City provider or supervisor entry
that runs the binary with explicit `--city /home/li/Criopolis`. systemd
is also viable, but it moves lifecycle observation outside gc. The repo
does not edit city `pack.toml`; that remains mayor-authored.

## Second-Reviewer Focus

- Review `src/dispatch.rs:90` (decision table) first: it is the routing
  contract.
- Review `src/state.rs:50` (cursor advance) and `src/state.rs:63`
  (dispatch record persistence): these are the restart-safety points.
- Review `src/gc.rs:80` (gc command boundary): all effects cross there.
- Decide whether skip records for non-cascade beads should remain in
  redb or be reduced to stderr-only logging once live volume is known.
