# AGENT.md — Repository Rules

This file is the working contract for humans and AI agents in this repository. Read it before changing code, docs, commits, or releases.

## 1. Working language

- **English is the only working language of the repository.** Source code, identifiers, comments, documentation, ADRs, commit messages, PR titles and descriptions, issues, test names, fixtures, and UI source strings are written in English.
- Localized translation catalogs are the only exception. A catalog may contain the target language because that is product data, not repository prose.
- Do not add a second working language to source or docs. Do not write Spanish comments, Spanish commit messages, or bilingual duplicate documents.
- When a user asks for a change in another language, implement it in English and answer the user in the language they used.

## 2. Repository structure

The project is a Cargo workspace, edition 2024, resolver 3.

```text
.
├── Cargo.toml
├── crates/
│   ├── mdview-model/
│   ├── mdview-markdown/
│   ├── mdview-syntax/
│   ├── mdview-workspace/
│   ├── mdview-media/
│   ├── mdview-theme/
│   ├── mdview-editor/
│   ├── mdview-render/
│   ├── mdview-ui/
│   ├── mdview-i18n/
│   └── mdview-app/
├── xtask/
├── docs/
├── tests/
└── AGENT.md
```

Rules:

- **One feature boundary per crate.** A crate owns one coherent responsibility and exposes a small public API.
- **Keep the dependency direction one-way.** Core crates (`mdview-model`, `mdview-markdown`, `mdview-syntax`, `mdview-workspace`, `mdview-media`) must not depend on egui or on the app shell. UI crates may depend on core crates, never the reverse. `mdview-app` orchestrates I/O, windows, settings, jobs, and updates.
- **No filesystem or network I/O in UI crates.** Use the app's job/command layer.
- **No `unwrap`/`expect`/panic in library paths** unless the invariant is documented and tested. Return typed errors.
- **No webview, embedded browser, or JavaScript runtime.** This includes Electron, Tauri, Wails, NW.js, WebView2, and JS engines used to render content.
- **A new crate is justified only by a real boundary**, not by a single type or a small helper. If an existing crate owns the responsibility, extend that crate.
- **Platform-specific code belongs in the narrowest crate that needs it.** Gate it with `cfg` and keep the public interface cross-platform.
- **`xtask` owns repository automation** (fixtures, packaging, release checks, file-length checks). Do not put build logic in application runtime code.

## 3. Commit rules

Use Conventional Commits with a **required scope**:

```text
<type>(<scope>): <summary>

<body>

<footer>
```

### 3.1 Summary line

- Format: `type(scope): imperative summary`.
- `type` is lowercase.
- `scope` is lowercase and names the crate or area that changed.
- The summary is imperative, specific, and no longer than 72 characters.
- Do not end the summary with a period.
- Do not use vague summaries such as `fix stuff`, `updates`, `wip`, or `misc`.

### 3.2 Body

- Leave one blank line after the summary.
- Explain **what changed and why**. Include constraints, tradeoffs, and user-visible behavior when relevant.
- Wrap lines at 72 characters.
- Reference issues at the end: `Refs: #123` or `Closes: #123`.
- If the change is breaking, add `BREAKING CHANGE:` in the footer and explain the migration.
- For UI changes, include the test plan and, when useful, the snapshot names.

### 3.3 Types

| Type | Use |
|---|---|
| `feat` | A user-visible feature or capability. |
| `fix` | A bug fix. |
| `docs` | Documentation only. |
| `refactor` | Internal restructuring with no behavior change. |
| `test` | Tests or fixtures only. |
| `perf` | Performance improvement. |
| `build` | Build system, packaging, or dependencies. |
| `ci` | CI/CD configuration. |
| `chore` | Maintenance that is not one of the above. |
| `style` | Formatting only. |
| `revert` | Reverts a previous commit; reference it in the body. |

Do not use `feat` for refactors, docs, dependency bumps, or tests. The type must describe the actual change.

### 3.4 Scopes

Use the crate name without the `mdview-` prefix:

`app`, `ui`, `editor`, `render`, `model`, `markdown`, `syntax`, `workspace`, `media`, `theme`, `i18n`, `xtask`, `repo`, `ci`, `docs`, `deps`.

Use `repo` for workspace-wide policy, `ci` for pipelines, `docs` for documentation, and `deps` only for dependency updates that do not change behavior. If a change spans several crates, choose the scope of the primary boundary and list the others in the body.

### 3.5 Examples

```text
feat(editor): add virtualized line rendering

Render only the visible line range plus an overscan window. This keeps
scroll and typing responsive on large files while preserving the existing
selection and undo model.

Refs: #42
```

```text
fix(workspace): follow external renames by file identity

Match the platform file identity before falling back to path comparison so
an open tab keeps its document after a rename in Finder, Explorer, or
Nautilus. Ambiguous cases remain in the missing-file state.

Closes: #108
```

```text
docs(roadmap): translate the roadmap to English
```

### 3.6 Commit hygiene

- **One logical change per commit.** Do not mix a refactor with a feature, or a dependency bump with a behavioral change.
- The repository must build, format, and pass tests at every commit.
- Add or update tests and documentation in the same commit as the behavior they describe.
- Do not commit commented-out code, dead code, temporary debug output, secrets, credentials, local paths, or generated build output.
- `Cargo.lock` is committed for the application. `target/` is ignored.
- Generated files, vendored sources, snapshots, and fixtures are committed only when they are required for a reproducible build or test. State why in the commit body.
- Never rewrite published history or force-push a shared branch without explicit approval.

## 4. File length limit

**Hard limit: every hand-written code file must be 500 lines or fewer.**

- This applies to `.rs` files under `crates/**` and `xtask/**`, including test modules and fixtures written by hand.
- The limit counts physical lines, including blank lines and comments. Do not compress code to game the limit; split by responsibility.
- `Cargo.lock`, generated files, vendored code, binary assets, snapshots, and data fixtures are exempt. Hand-written test fixtures are not exempt.
- If a file approaches the limit, extract a cohesive submodule. Do not create a `misc.rs` or `utils.rs` dumping ground.
- `lib.rs` should be a thin public facade. Put implementation in named modules and re-export the stable API.
- A pull request that adds a file above 500 lines must be split before review. An existing oversized file may be split in a dedicated `refactor(scope):` commit.
- `cargo xtask check-file-length` is the local gate. Hosted CI/CD is intentionally not configured before `1.0.0`; it is added as part of the release gate, not during feature development.

## 5. Code and architecture rules

- **No new dependency without justification.** Explain its role, license, binary-size impact, maintenance status, and alternative in the PR. Prefer the standard library and existing workspace dependencies.
- **Pin `craft-libs` to one revision** for all `craft-*` crates. Do not mix tags, branches, and revisions. Commit the lockfile and build with `--locked`.
- **Pin `egui` to the exact version required by the selected `craft-ui` revision.**
- **Tokens, not literal colors.** UI, syntax highlighting, and Markdown rendering use the theme token layer. Literal colors are allowed only inside theme definitions and deliberate content assets.
- **English identifiers and comments.** No abbreviations that obscure meaning.
- **Errors are data.** Use typed errors with context; do not swallow failures. User-facing errors must be actionable.
- **Security defaults are closed.** No script execution in HTML/SVG, no external resources inside SVG, no path traversal, no private-network image fetches without an explicit setting.
- **Tests are part of the feature.** Add unit tests for logic, integration tests for filesystem/watcher behavior, and render snapshots for visible UI changes.
- **Accessibility is not optional.** New controls need keyboard access, a focus state, an accessible label, and sufficient contrast.
- **No dates, durations, or effort estimates in roadmap or planning documents.** Organize work by phases, milestones, dependencies, and acceptance criteria.

## 6. CI/CD policy

Do not add, enable, or run hosted CI/CD before `1.0.0`.

- Verify every change locally with `cargo fmt`, strict Clippy, `cargo test --workspace`, `cargo xtask check-file-length`, and a release build.
- Keep packaging scripts in `xtask` so they can run on the developer machine.
- Hosted CI/CD, signing, notarization, store publishing, and automated release promotion are part of the `1.0.0` release gate. They are not part of the feature-development loop.
- Do not push tags or trigger release workflows before the local feature set is operational.

## 7. Pull requests

- The PR title follows the same format as the commit summary.
- The description states the problem, the approach, the user-visible effect, the test plan, and any limitations.
- Link the issue and the roadmap milestone.
- UI changes include before/after snapshots or a short screen recording.
- Do not merge with failing required checks, unresolved data-loss risks, or an undocumented scope reduction.
- Keep the diff focused. Move unrelated cleanups to a separate PR.

## 8. Definition of done for a change

A change is complete when:

- It builds with `cargo build --locked`.
- `cargo fmt --check` and `cargo clippy --workspace --all-targets -- -D warnings` pass.
- Relevant unit, integration, and snapshot tests pass.
- New behavior is documented and, when user-visible, reflected in the roadmap support matrix.
- No file exceeds the 500-line limit.
- The commit message follows this document.
- No unrelated changes are included.

## 9. Roadmap progress tracking

`ROADMAP.md` is the source of truth for scope and progress.

- When a functionality is implemented **and tested**, update its row in the milestone's `Progress` table in the same commit.
- Use `DONE` only when the stated evidence exists. Use `DONE (Linux; macOS/Windows pending)` when the implementation and Linux verification are complete but cross-platform verification is pending.
- Use `IN PROGRESS`, `BLOCKED`, or `PENDING` for everything else. Do not mark a feature done just because it compiles.
- Mark the smallest meaningful unit: a deliverable or acceptance criterion, not an entire milestone, unless every criterion for that milestone is satisfied.
- Record the evidence in the table: test name, command, manual OS check, or snapshot name.
- If a later change regresses a marked feature, change it back to `IN PROGRESS` or `BLOCKED` in the same commit.
- Do not silently reduce scope. If a criterion cannot be met, document the deviation and update the roadmap before claiming the milestone is complete.

## 10. Conflict resolution

If a direct user request conflicts with these rules, ask before bypassing a rule. Do not silently weaken the 500-line limit, the English-only policy, the crate boundaries, or the no-webview constraint.
