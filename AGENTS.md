# Agent instructions — orchestrator

You **MUST** read AGENTS.md at `github:ligoldragon/lore` — the
workspace contract.

## Repo role

The **orchestrator daemon** watches Gas City bead events for
`cascade-chain` work and dispatches the next bead in each cascade.

It owns only the deterministic cascade dispatcher. It does not own
Criopolis agent prompts, city `pack.toml`, order definitions, or council
synthesis.

## Style

- Rust style canon: `~/git/lore/rust/style.md`.
- Nix packaging canon: `~/git/lore/rust/nix-packaging.md`.
- Methods on types, no free functions outside `main` and small private
  helpers.
- Typed newtypes at boundaries.
- `Error` is one `thiserror`-derived enum for the crate.
- Edition 2024.
- Tests live under `tests/`.

## Process

- Commit per logical change.
- Push immediately after every commit.
- Co-author commits with:
  `Co-Authored-By: Codex CLI <noreply@anthropic.com>`.
