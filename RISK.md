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

## cr-8g3x09 Integration Harness

Run the isolated Gas City cascade test with:

```sh
nix flake check
nix build .#checks.x86_64-linux.orchestrator-integration
```

The flake check wires the integration check through
`flake.nix:96` (orchestrator integration check). The Rust harness at
`tests/integration_cascade.rs:53` (isolated cascade test) invokes
`tests/scripts/orchestrator-isolated-gc-test.sh:1` (isolated Gas City
shell harness). The fixture at `tests/fixtures/deterministic-city.toml:49`
(Codex Spark model default) sets `model = "gpt-5.3-codex-spark"`,
`effort = "low"`, and unrestricted permission mode so the new Gas City
Codex provider schema emits the expected Codex CLI flags.

The environment contract is the safety boundary. The script keeps
`HOME` pointed at the real platform home so Codex subscription auth can
be read, while `GC_HOME`, `XDG_RUNTIME_DIR`, `DOLT_ROOT_PATH`, `TMPDIR`,
and the city root are all under a throwaway test root; see
`tests/scripts/orchestrator-isolated-gc-test.sh:145` (isolated env
wrapper). Host lifecycle commands are shadowed by local no-op shims at
`tests/scripts/orchestrator-isolated-gc-test.sh:86` (host command
shims), and the supervisor starts as a child process at
`tests/scripts/orchestrator-isolated-gc-test.sh:187` (isolated
supervisor start). A full container is not used because real Codex
sessions need normal access to subscription auth and network; the test
isolates Gas City state instead of hiding the whole process tree.

The integration path could break if Gas City changes Codex provider
option names, `gc sling` metadata, bd JSON output, or supervisor status
text. The shim mode checks that `gpt-5.3-codex-spark`, low effort, and
unrestricted permission flags are actually passed to Codex before it
runs the deterministic local test agent; see
`tests/scripts/orchestrator-isolated-gc-test.sh:116` (Codex argument
checks). The live-cost path still uses a research-preview Codex model,
so manual runs without shim mode may spend subscription/API budget.

Second-reviewer focus for this harness:

- `tests/scripts/orchestrator-isolated-gc-test.sh:145` (isolated env
  wrapper) should be checked first for leaks into live `~/.gc`.
- `tests/scripts/orchestrator-isolated-gc-test.sh:326` (cascade bead
  creation) should be checked against the cascade metadata contract.
- `tests/scripts/orchestrator-isolated-gc-test.sh:452` (restart
  idempotence assertion) should be checked for double-dispatch coverage.
- `flake.nix:96` (integration check inputs) should be checked whenever
  `gascity-nix` changes its package or runtime dependency surface.
