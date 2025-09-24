# pdf-pwbf

A fast, multi-threaded CLI for brute‑forcing passwords on encrypted PDF files using `lopdf`, `rayon`, and `clap`. Shows a live progress bar and stops as soon as the correct password is found.

Warning and ethics: Only use on PDFs you own or have explicit permission to test. Brute‑forcing is computationally expensive and may run for a long time.

## Install
- Prereq: Rust toolchain (stable) with Cargo.
- Build release binary:
  - `cargo build --release`
  - Binary at `target/release/pdf-pwbf`

## Usage
```
pdf-pwbf --input <PDF> [--min N] [--max N] [-d] [-a] [-s] [-t N]
```
Flags

| Flag | Type | Default | Description |
|---|---|---:|---|
| `-i, --input <PDF>` | path | — | Path to password‑protected PDF (required) |
| `--min <N>` | integer | 6 | Minimum password length |
| `--max <N>` | integer | 6 | Maximum password length |
| `-d, --digit` | boolean | false | Include digits 0‑9 in charset |
| `-a, --alphabet` | boolean | false | Include A‑Z and a‑z in charset |
| `-s, --symbol` | boolean | false | Include common symbols (e.g., `!@#$...`) |
| `-t, --threads <N>` | integer | 1 | Number of worker threads |

Notes
- Provide at least one candidate set via `--digit` and/or `--alphabet`; `--symbol` is optional.
- The tool authenticates the password and reports success; it does not write a decrypted copy.

### Examples
- 4‑digit PIN (fast check):
  - `cargo run -- -i test-data/test-pdf.pdf --min 4 --max 4 -d -t 4`
- 1–5 length alphanumeric:
  - `cargo run -- -i test-data/test-pdf.pdf --min 1 --max 5 -d -a -t 8`
- Include symbols 1–3 length:
  - `cargo run -- -i test-data/test-pdf.pdf --min 1 --max 3 -d -a -s -t 6`

## How it works
- Computes total attempts across length range and shows a progress bar (`indicatif`).
- Splits the keyspace into chunks and searches in parallel with a Rayon thread pool.
- Uses `lopdf` to test candidate passwords via `authenticate_password`.

## Performance & Limits
- Search space grows as `|charset|^length`. Keep `--max` realistic.
- More threads increases CPU usage; set `--threads` according to cores.
- Very large spaces are rejected to avoid overflow/meaningless runs.

## Development
- Format: `cargo fmt --all`
- Lint: `cargo clippy --all-targets -- -D warnings`
- Build: `cargo build` or `cargo build --release`
- Tests (add as needed): `cargo test`

## Project Layout
- `src/main.rs` — CLI and brute‑force engine
- `Cargo.toml` — dependencies and release profile

For contributor norms, see `AGENTS.md`.
