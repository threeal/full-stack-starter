# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

> **Template notice:** This file describes the template repository itself. In a project derived from this template, tell the user it should be replaced with project-specific guidance.

## About This Repository

A minimal full-stack starter template: a Rust app at the root, built on axum, that builds the React web frontend in `web/`, bundled with Vite, and embeds it in its binary. Its only application code is an example click counter: an in-memory counter API under `/api` in `src/lib.rs`, and placeholder pages in `web/src/pages/` that call it, routed by `web/src/router.tsx` — real pages and server routes are added by projects derived from it.

Run Cargo commands from the root and pnpm commands from inside `web/`. The web frontend only runs through the root app, since `build.rs` builds it with pnpm on every Cargo command; running the Vite dev server on its own isn't supported. dprint formats everything outside `web/`, which Prettier formats instead.

## Gotchas

- `lefthook run pre-commit` skips every job when nothing is staged, even ones that ignore the staged file list — pass `--all-files` to run it outside an actual commit.
- The pre-commit hook fixes files in place, and the run fails if any file changes. Report that failure and leave the fixes for the user to review and re-stage before committing again.
- The web frontend's Vitest 100% coverage threshold applies to the whole run, not per file — a single test file can fail on source that other tests cover, so run the full suite (`pnpm vitest run`).
- The root's test command is `cargo tarpaulin`, not `cargo test` — only it applies the 100% coverage threshold from `tarpaulin.toml`.
- `build.rs` reruns only when a path in its `WEB_INPUTS` changes — a new file in `web/` outside those, such as another config file, must be added there or the embedded web frontend goes stale.
- `.gitignore` ignores every dotfile and dotdir (`.*`) — a new one needs an explicit `!` exception there or it silently won't be tracked.
