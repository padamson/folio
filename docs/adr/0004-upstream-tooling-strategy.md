# ADR 0004: Upstream tooling strategy

**Status:** Proposed

**Date:** 2026-07-18

**Related Documents:**
- [Roadmap](../roadmap.md)
- [ADR-0002: Server and channel strategy](0002-server-and-channel-strategy.md)

---

## Context and problem statement

Folio sits inside a portfolio of my own tools (playwright-rust, panschema) and
alongside mature third-party tools (osxphotos, osxmetadata) that cover parts of
its problem space. Four integration decisions were pending on resume:

1. Which version source for `playwright-rs` — crates.io release or git `main`?
   (The working tree carried a stale 0.6.1 → 0.7.0 bump; published is 0.14.1
   with an unreleased 0.15 on `main`.)
2. How to adopt panschema, which is pre-crates.io with its useful writers
   (Rust codegen, HTML docs, JSON Schema) only on `main`?
3. Should the Apple Photos bridge (ADR-0002) be built on a Rust port of
   osxphotos?
4. Should the macOS metadata harvest be built on a Rust port of osxmetadata?

## Decision drivers

1. **Dogfooding intent** — folio exists partly as a proving ground for
   playwright-rust and should now serve the same role for panschema; bugs get
   reported and missing features implemented upstream as necessary.
2. **Value-driven schedule** — the backlog comes first. Folio has already
   absorbed one multi-month tool detour (playwright-rust); another one is not
   justified before the archive is trustworthy.
3. **Maintenance treadmills are real** — osxphotos reads the undocumented
   Photos SQLite database via reverse engineering accumulated over a decade,
   and is currently broken for shared albums on macOS 26 while its maintainer
   catches up. Porting it means inheriting that treadmill.
4. **Rust-first, with sanctioned edges** — the core stays Rust; Python is an
   accepted supporting language for platform-edge scripts.

## Decision outcome

Four decisions, one table:

| Tool | Decision | Mechanics |
|------|----------|-----------|
| playwright-rust | Pin git `main`, active-iteration dogfood mode | `playwright-rs = { git = "...", branch = "main" }` with a `[patch.crates-io]` path override for local co-editing. The migration from 0.6.1 crosses the single-crate merge, 0.14's `#[non_exhaustive]` option structs, and 0.15's sync locators + `f64` mouse coordinates; adopt the 0.15-era agent docs (`CLAUDE_SNIPPET.md` / `playwright-rs-usage` skill). Additionally: `playwright-rs` moves to dev-dependencies — the runtime browser-open in `folio-core::preview` is replaced by the `webbrowser` crate, so users of the CLI never need Node or Playwright browsers. |
| panschema | Adopt at story 002; CLI install pinned to `main` | Local shell alias to `panschema/target/debug/panschema`; CI resolves `main` HEAD via `gh api .../commits/main --jq .sha` and runs `cargo install --git ... --rev ${SHA} --locked --force` (the established consumer pattern). `panschema.toml` with a `path:` schema source and a `[generate.folio]` block (`rust`, `html`, `json_schema`); generated output committed; `panschema verify` in CI. Known gaps folio will likely hit and report upstream: JSON Schema is scalar-only, Postgres DDL skips inheritance and multivalued slots. |
| osxphotos | Wrap the CLI as an adapter; do not port | The bridge shells out to `osxphotos export --album ... --sidecar XMP` (its XMP output lands directly in folio's native format) and `osxphotos import` for the outbound direction. A Rust port would rebuild a maintained decade of schema archaeology against a target Apple moves yearly — and unlike playwright-rust, it fills no ecosystem gap worth owning. Risk containment: pin the archivist Mac's macOS upgrades to osxphotos compatibility, the same discipline as any pinned tool. |
| osxmetadata | Reference spec, not a port | What folio needs — reading Finder tags, comments, and download origins (`kMDItemWhereFroms`) at ingest — is a small native module on the `xattr` + `plist` crates (Finder tags are a binary plist in one xattr). osxmetadata's CLI serves as the audit tool and test oracle. Writing Finder comments requires AppleScript; that path stays read-only in Rust. If the module ever wants to be a published `osxmetadata-rs` crate, extract it after it has proven itself here — not before. |

**Trade-offs accepted:**

- Tracking two `main` branches means absorbing upstream breakage promptly;
  that is the point of active-iteration mode, and `Cargo.lock` / the SHA pin
  keep builds reproducible between deliberate updates.
- Git pins skip validation of *published* artifacts; do occasional crates.io
  spot-checks when releases ship.
- The Apple Photos bridge is macOS-hosted by construction; that machine is
  already the archive's operational home.

## Consequences

- Folio consumes playwright-rust as a library and panschema as a CLI, both
  tracking `main` (cargo-vet git exemptions apply to the library dependency if
  folio adopts the supply-chain posture).
- The stale 0.7.0 bump in the working tree is discarded and redone against
  `main` (resume checklist in the roadmap).
- `folio-core` loses its heaviest runtime dependency; E2E tests keep the full
  playwright-rust surface as dev-dependencies.
- Python enters the repo only at the channel edges (osxphotos invocations),
  consistent with the supporting-language policy.

## Validation

- [ ] Fresh `cargo build` of the workspace against playwright-rust `main`;
      E2E suite green with the sync-locator API.
- [ ] `folio ingest` runs on a machine with no Node installed.
- [ ] `panschema generate` produces compiling Rust types from the story 002
      schema; `panschema verify` wired into CI.
- [ ] Bridge round-trip through the osxphotos CLI on the current macOS,
      documented with the exact pinned versions.
