# Repository Guidelines

## Project Structure & Module Organization
- `src/main.rs` — CLI entry (brute‑forces encrypted PDFs via `lopdf`).
- `Cargo.toml` — dependencies and build profiles; optimized `release` enabled.
- Future modules live under `src/` (e.g., `src/bruteforce.rs`, `src/cli.rs`). Prefer small, single‑purpose modules.

## Build, Test, and Development Commands
- Build (debug): `cargo build`
- Build (release): `cargo build --release`
- Lint: `cargo clippy --all-targets -- -D warnings`
- Format: `cargo fmt --all`
- Run help: `cargo run -- --help`
- Example run: `cargo run -- -i test-data/test-pdf.pdf --min 1 --max 4 -d -a -t 4`

## Coding Style & Naming Conventions
- Use `rustfmt` defaults; run `cargo fmt` before every PR.
- Keep functions under ~50 lines; extract helpers in `src/*`. 
- Modules: snake_case (`brute_force.rs`), types: UpperCamelCase, functions/vars: snake_case, constants: SCREAMING_SNAKE_CASE.
- Prefer `Result<T, E>` returns and early errors; no `unwrap()` in library‑like code.

## Testing Guidelines
- Framework: Rust built‑in test harness.
- Place unit tests in the same file under `#[cfg(test)] mod tests { ... }`.
- Add integration tests in `tests/` (create if missing). Name as `tests/{feature}_it.rs`.

## Commit & Pull Request Guidelines
- Use Conventional Commits: `feat:`, `fix:`, `perf:`, `refactor:`, `test:`, `docs:`, `chore:`.
- Commits: small, focused, formatted code only passes CI (clippy + fmt).
- PRs must include: concise description, rationale, CLI examples, and performance notes (attempts/sec changes where relevant).
- Link issues; add screenshots or terminal snippets for UX/CLI behavior.

## Security & Operational Notes
- Use only with PDFs you own or have permission to test.
- Avoid committing real protected documents; keep large files out of Git.
- Be explicit with limits: validate `--min/--max` and `--threads` as in `main.rs`.

## Agent‑Specific Instructions
- Respect this file’s scope across the repo.
- Make minimal, surgical changes; prefer adding modules over large diffs.
- When editing code, also update examples and commands above.
