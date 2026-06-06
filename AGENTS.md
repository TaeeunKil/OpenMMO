# Agent Guidelines

## Python

- When running Python in this repository, use the project virtual environment at `.venv`.
- Prefer `.venv\Scripts\python.exe` for direct Python commands.
- Prefer `uv pip install ...` for installing Python packages into the active project environment.

## Pre-Commit Validation

- Run validation only once, immediately before making a commit, not after every task.
- For frontend changes, run `npm run check` and `npm run lint`.
- For Rust changes, run `cargo fmt` and `cargo check`.
