# Contributing

Thanks for helping improve `tt-graph-cdfa-rust`. This project is a research
reproduction and engineering prototype, so changes should preserve both code
correctness and reproducibility.

## Development Setup

Install a recent Rust toolchain. The default feature set uses LLVM libclang for
the C++ frontend.

On Windows, set `LIBCLANG_PATH` to the directory that contains `libclang.dll`.
For core-library work that does not need the C++ frontend, use
`--no-default-features`.

## Before Sending Changes

Run the most relevant checks first, then the broader checks when reasonable:

```powershell
cargo fmt
cargo test
cargo test --no-default-features
cargo clippy --all-targets
```

If a change touches JSON output, add or update tests that parse the JSON with
`serde_json` and check the intended schema fields. Do not rely only on string
contains assertions for structured output.

If a change touches paper reproduction behavior, document the affected command
and expected output in `README.md` or `PAPER_REPRODUCTION.md`.

Before release work, also run:

```powershell
cargo package
```

## Scope Guidelines

- Keep changes small and focused.
- Preserve existing CLI command names, default paths, and exit-code behavior
  unless the change explicitly targets the CLI contract.
- Do not update benchmark budgets or reproduction claims without explaining the
  environment and rationale.
- Do not commit generated build artifacts, local editor state, secrets, or
  machine-specific configuration.

## Release Guidelines

- Use version tags in the form `vMAJOR.MINOR.PATCH`.
- Keep crates.io publishing manual; do not add automatic publishing on every
  tag without an explicit maintainer decision.
- Update `CHANGELOG.md` before cutting a release.
- Confirm JSON schema and reproduction command changes are documented.

## Review Expectations

Useful pull requests include:

- a concise summary of the behavior change
- tests or a clear reason tests are not applicable
- commands run and their results
- any known reproduction, benchmark, or platform caveats
