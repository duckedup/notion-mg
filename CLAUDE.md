# notion-mg

A CLI tool and SDK for the Notion API, designed for AI agent consumption. Ships as both a
binary (`src/main.rs`) and a library (`src/lib.rs`).

## Layout

- `src/api/` — Notion API types and `NotionClient` methods, one module per resource
- `src/cli/` — clap subcommands, one module per resource, each mirroring its `api/` counterpart
- `src/client/` — HTTP client, error mapping, and the generic `paginate` helper
- `src/output/` — `OutputFormat` dispatch and the `PrettyPrint` trait implementations
- `tests/` — integration tests against a `wiremock` mock server

Adding a resource means touching all four: a type + client methods in `api/`, a `Subcommand` in
`cli/`, a `PrettyPrint` impl in `output/pretty.rs`, and a variant in `cli/mod.rs`.

## Commands

```bash
just check   # fmt + clippy -D warnings + test — run before every commit
just test
just lint
```

## Comments and documentation

Comments must be **additive**: they earn their place by saying something the code cannot.

Write a comment when it explains *why* — a non-obvious constraint, an API quirk, a deliberate
tradeoff, a workaround and what would let it go away. Prefer one terse line over a paragraph.

Do not write a comment that restates the code. No `// increment the counter`, no doc block
narrating what a six-line function plainly does, no section banners, no `# Arguments` /
`# Returns` boilerplate that repeats the signature. A well-named function with an obvious body
needs no doc comment at all.

Doc comments (`///`) belong on public API surface where the contract is not self-evident:
error conditions, panics, units, ownership, or behavior a caller could not infer from the
signature. Keep them to a sentence or two.

The same rule governs prose. Do not add README or CLAUDE.md sections that restate what a
`--help` output or a function signature already says.

## Issue tracking

Issues live in [beads](https://github.com/steveyegge/beads) (`bd`), backed by a Dolt database in
`.beads/`. The database itself is gitignored; it syncs to the `refs/dolt/data` ref on the GitHub
remote, so issue history never lands on `main`.

```bash
bd ready               # unblocked work
bd create "title" -t feature -p 1
bd show <id>
bd close <id>
bd dolt push           # publish to refs/dolt/data
bd dolt pull           # fetch teammates' issues
```

Run `bd dolt pull` after checking out a branch and `bd dolt push` before opening a PR.
