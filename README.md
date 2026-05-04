# orchestrator

`orchestrator` is the Criopolis cascade dispatcher.

It watches Gas City bead events, filters to beads labeled
`cascade-chain`, and dispatches the next step of a cascade:

- position-1 bead created: `gc sling <cascade_target_agent> <bead> --no-formula`
- cascade bead closed with `cascade_next`: sling the next bead with `--no-formula`
- final cascade bead closed: notify mayor with cascade completion mail

The daemon stores its event cursor in redb and records dispatches as
rkyv archives for restart-safe introspection.

## Usage

```sh
orchestrator --city /home/li/Criopolis
```

Useful development run:

```sh
orchestrator --city /home/li/Criopolis --once
```

The default state database is:

```text
<city>/.gc/orchestrator.redb
```

Mayor owns city lifecycle wiring. This repository only ships the daemon
binary and library.

## Cascade Bead Contract

Every orchestrated bead carries:

- label `cascade-chain`
- metadata `cascade_target_agent`
- metadata `cascade_position`
- metadata `cascade_next` when another bead follows
- metadata `cascade_final = "true"` on the final bead
- metadata `cascade_id` for completion mail

`gc.routed_to` is the live routing stamp written by raw `gc sling`
dispatch. Cascade definitions use `cascade_target_agent` so later steps
stay invisible to ordinary agent work queries until the orchestrator
advances the chain.

The daemon ignores beads labeled `order-tracking` or
`gc:order-tracking`.

## Development

```sh
cargo test
nix flake check
```
