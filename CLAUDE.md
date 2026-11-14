# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

> **Template notice:** This file describes the template repository itself. In a project derived from this template, tell the user it should be replaced with project-specific guidance.

## About This Repository

A minimal full-stack starter template: a React app in `app/`, bundled with Vite, and a Rust server in `server/`, built on axum, that serves the built app from the directory in `PUBLIC_DIR`. Its only application code is placeholder pages in `app/src/pages/`, routed by `app/src/router.tsx` — real pages and server routes are added by projects derived from it.

Each of `app/` (pnpm) and `server/` (Cargo) is a self-contained project, so run its commands from inside that directory. The root holds only repo-wide config, with dprint formatting everything outside `app/`.

## Gotchas

- `lefthook run pre-commit` skips every job when nothing is staged, even ones that ignore the staged file list — pass `--all-files` to run it outside an actual commit.
- The pre-commit hook fixes files in place, and the run fails if any file changes. Report that failure and leave the fixes for the user to review and re-stage before committing again.
- The app's Vitest 100% coverage threshold applies to the whole run, not per file — a single test file can fail on source that other tests cover, so run the full suite (`pnpm vitest run`).
- The server's test command is `cargo tarpaulin`, not `cargo test` — only it applies the 100% coverage threshold from `server/tarpaulin.toml`.
- `.gitignore` ignores every dotfile and dotdir (`.*`) — a new one needs an explicit `!` exception there or it silently won't be tracked.
