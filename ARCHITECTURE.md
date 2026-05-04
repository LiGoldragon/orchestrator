# ARCHITECTURE — orchestrator

The orchestrator daemon is the deterministic cascade dispatcher for
Criopolis. It watches Gas City events, reads typed bead metadata, and
routes the next bead in a cascade through `gc sling`.

## Role

Criopolis cascade chains are ordinary beads with metadata:

- `gc.routed_to` names the target agent.
- `cascade_position` names the bead's position in the chain.
- `cascade_next` names the next bead id.
- `cascade_final = "true"` marks the final bead.
- `cascade_id` names the chain for final notification.
- label `cascade-chain` marks beads the daemon may act on.

The orchestrator starts a chain when a position-1 bead is created,
advances a chain when a cascade bead closes with `cascade_next`, and
notifies mayor when a final cascade bead closes.

## Boundaries

Owns:

- redb-backed event cursor state.
- redb-backed dispatch records for restart introspection.
- rkyv-archived dispatch records.
- typed parsing of `gc events` JSON Lines and `gc bd show --json`.
- deterministic calls to `gc sling` and `gc mail send --notify mayor`.

Does not own:

- Criopolis `pack.toml` registration.
- Criopolis agent prompts.
- order files or health checks inside the city repo.
- Gas City event storage or bead schema.

## Code Map

```
src/
├── lib.rs          — module entry + re-exports
├── main.rs         — binary entrypoint
├── command_line.rs — CLI options
├── error.rs        — crate Error enum
├── identifiers.rs  — typed ids and names
├── event.rs        — event JSON Lines parsing
├── bead.rs         — cascade bead view over bead JSON
├── gc.rs           — typed wrapper around gc CLI calls
├── state.rs        — redb cursor and dispatch storage
├── dispatch.rs     — cascade decision and side effects
└── orchestrator.rs — top-level daemon loop
```

## Invariants

- Bead close is the completion source.
- The stored cursor is a Gas City event sequence number.
- The daemon advances the cursor only after processing an event.
- Non-`cascade-chain` beads are ignored.
- `order-tracking` and `gc:order-tracking` beads are ignored.
- Cursor and dispatch state are redb-backed.
- Dispatch records are archived with rkyv.

## Runtime Shape

The daemon runs outside the LLM agent loop. Mayor wires it into the
city lifecycle separately. The command line accepts a city directory and
state database path so the same binary can run under a custom Gas City
provider, a supervisor-managed service, or systemd.

## Cross-Cutting Context

- Project-wide architecture: `~/git/criome/ARCHITECTURE.md`.
- Workspace contract: `~/git/lore/AGENTS.md`.
- Cascade design source:
  `/home/li/Criopolis/_intake/reports/orchestrator-design-v3.md`.

## Status

Initial Rust daemon implementation for `cr-qp7bha` (orchestrator Rust
repository).
