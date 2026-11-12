# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

> **Template notice:** This file describes the template repository itself. In a project derived from this template, tell the user it should be replaced with project-specific guidance.

## About This Repository

A minimal full-stack starter template: a React app in `app/`, bundled with Vite, and a Node.js Fastify server in `server/` that serves the built app from the directory in `PUBLIC_DIR`. Its only application code is placeholder pages in `app/src/pages/`, routed by `app/src/router.tsx` — real pages and server routes are added by projects derived from it.

## Gotchas

- `lefthook run pre-commit` skips every job when nothing is staged, even ones that ignore the staged file list — pass `--all-files` to run it outside an actual commit.
- The pre-commit hook fixes files in place, and the run fails if any file changes. Report that failure and leave the fixes for the user to review and re-stage before committing again.
- Each package runs its own Vitest suite, and its 100% coverage threshold applies to that whole run, not per file — a single test file can fail on source that other tests cover, so run the full suite (`pnpm --recursive test`).
- In `server/`, `tsconfig.json` is for type checking, tests included; `tsconfig.build.json` only builds the server during `build`.
- `.gitignore` ignores every dotfile and dotdir (`.*`) — a new one needs an explicit `!` exception there or it silently won't be tracked.
