# Repository Guidelines

## Project Structure and Module Organization

- `desktop/` contains the Rust desktop app, tests, and platform documentation.
- `extension/entrypoints/content.ts` runs the browser extension gamepad loop, manages focus, and activates selected controls.
- `extension/entrypoints/background.ts` handles browser tab actions such as back, next tab, and previous tab.
- `extension/lib/spatial.ts` finds visible, enabled candidates and chooses the nearest target by direction.
- `extension/lib/gamepad.ts` reads real browser gamepad state and provides controlled input frames to E2E builds.
- `extension/tests/e2e/` contains Playwright tests against local pages loaded with WXT.
- `extension/wxt.config.ts` defines the extension manifest; `extension/tsconfig.json` enables strict TypeScript checking.
- `extension/README.md` documents the browser extension. WXT generates the unpacked extension under `extension/.output/chrome-mv3`.
- Root `package.json` and `.releaserc.json` configure semantic-release for the desktop app.

## Build, Test, and Development Commands

- `make web-install` installs the extension dependencies.
- `make web-dev` starts WXT development mode.
- `make web-typecheck` runs TypeScript without emitting files.
- `make web-lint`, `make web-format`, and `make web-format-check` run Oxlint and Oxfmt.
- `make web-build` creates a production extension build; `make web-package` creates its ZIP.
- `make web-test` builds the WXT `e2e` mode and runs Playwright against Chromium; run `npx playwright install chromium` from `extension/` once to install its browser.
- `make desktop-run`, `make desktop-check`, `make desktop-lint`, `make desktop-format`, and `make desktop-test` run the Rust app workflows.
- `make desktop-build` creates the Linux x64 archive and Windows x64 executable in `desktop/dist`; install the Windows Rust target, MinGW, and CMake first.
- `make desktop-linux-package` creates the Linux archive; `make desktop-windows` builds the Windows executable.

## Coding Style and Naming Conventions

Use strict TypeScript and preserve the existing two-space indentation, single quotes, and semicolons in `extension/`. Prefer explicit types at browser API and message boundaries. Use `camelCase` for variables and functions; keep module filenames lowercase. Run the configured Oxfmt and Oxlint commands for finished changes.

## Testing Guidelines

Always follow Red → Green for behavior changes: first add a focused test that fails for the missing behavior and run it, then implement the smallest change that makes it pass, and finally run the relevant test suite.

Before submitting, run the relevant extension format, lint, typecheck, and build commands, plus the desktop format, lint, check, and test commands. E2E tests use deterministic standard-mapped input frames and verify focus movement, activation, pause/resume, browser history, and tab switching. Physical controller behavior still needs manual validation; automated extension coverage targets Chromium and standard mappings.

## Commits and Pull Requests

This checkout has no Git metadata, so no project-specific commit convention can be verified. Use a short imperative commit subject. A pull request should describe the user-visible change, list validation performed, link a related issue when available, and include screenshots for visible UI changes.

<!-- gitnexus:start -->
# GitNexus — Code Intelligence

This project is indexed by GitNexus as **Gamepad-Navigator** (338 symbols, 754 relationships, 28 execution flows).

> Index stale? Run `node .gitnexus/run.cjs analyze --index-only` from the project root — it auto-selects an available runner. No `.gitnexus/run.cjs` yet? Bootstrap with `npx`, `bunx`, or `pnpm dlx` — e.g. `bunx gitnexus@latest analyze` (npm 11 npx crash; #1939).

## Always Do

- **MUST run impact analysis before editing.** Use `impact({target: "symbolName", direction: "upstream"})` (MCP) or `node .gitnexus/run.cjs impact "symbolName" --direction upstream --repo .` (CLI fallback); report callers, processes, and risk. Never substitute grep for graph analysis.
- **MUST analyze graph changes before committing.** Use `detect_changes({scope: "all"})` (MCP) or `node .gitnexus/run.cjs detect-changes --scope all --repo .` (CLI fallback). `partial: true` or `truncated: true` is not a clean check — a zero means unseen, not unaffected; re-run it. For regression review: `detect_changes({scope: "compare", base_ref: "main"})` or `node .gitnexus/run.cjs detect-changes --scope compare --base-ref "main" --repo .`.
- **MUST warn the user** if impact analysis returns HIGH or CRITICAL risk before proceeding with edits.
- **MUST treat `risk: UNKNOWN` as unresolved, not as low.** An empty caller set is not evidence the symbol is unused — it can also mean the callers are not resolvable by the index (plain-object property access, dynamic dispatch, cross-language calls). `impact` pairs `UNKNOWN` with a `riskNote` saying so. Confirm with a text search before treating the symbol as safe to change or delete; do not proceed on the strength of a zero.
- When exploring unfamiliar code, use `query({search_query: "concept"})` to find execution flows instead of grepping. It returns process-grouped results ranked by relevance.
- When you need full context on a specific symbol — callers, callees, which execution flows it participates in — use `context({name: "symbolName"})`.
- For security review, `explain({target: "fileOrSymbol"})` lists taint findings (source→sink flows; needs `analyze --pdg`).

## Never Do

- NEVER edit a function, class, or method before MCP/CLI impact analysis.
- NEVER ignore HIGH or CRITICAL risk warnings from impact analysis, and never read `UNKNOWN` as an all-clear — it means the walk could not answer, which is the one verdict that requires confirming by other means.
- NEVER rename symbols with find-and-replace — use `rename` which understands the call graph.
- NEVER commit before MCP/CLI graph change analysis.

## Resources

| Resource | Use for |
| --- | --- |
| `gitnexus://repo/Gamepad-Navigator/context` | Codebase overview, check index freshness |
| `gitnexus://repo/Gamepad-Navigator/clusters` | All functional areas |
| `gitnexus://repo/Gamepad-Navigator/processes` | All execution flows |
| `gitnexus://repo/Gamepad-Navigator/process/{name}` | Step-by-step execution trace |

## CLI

| Task | Read this skill file |
| --- | --- |
| Understand architecture / "How does X work?" | `.claude/skills/gitnexus-exploring/SKILL.md` |
| Blast radius / "What breaks if I change X?" | `.claude/skills/gitnexus-impact-analysis/SKILL.md` |
| Trace bugs / "Why is X failing?" | `.claude/skills/gitnexus-debugging/SKILL.md` |
| Rename / extract / split / refactor | `.claude/skills/gitnexus-refactoring/SKILL.md` |
| Tools, resources, schema reference | `.claude/skills/gitnexus-guide/SKILL.md` |
| Index, status, clean, wiki CLI commands | `.claude/skills/gitnexus-cli/SKILL.md` |

<!-- gitnexus:end -->
