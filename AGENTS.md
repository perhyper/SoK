# SoK Repository Instructions

## Documentation Ownership

- Keep `README.md` user-facing: project purpose, installation, usage, outputs, and concise architecture only.
- Do not copy internal review notes, implementation prompts, or agent-facing constraints into `README.md`.
- Put runtime agent behavior in `structure-of-knowledge/SKILL.md`.
- Put detailed research, collaboration, and output rules in `structure-of-knowledge/references/`.
- Put machine-verifiable contracts in `specs/` and enforce them in the CLI where appropriate.
- Link to the source of truth instead of duplicating the same rule across documents.

## Repository Boundaries

- Keep `structure-of-knowledge/` portable and free of Rust sources or build artifacts.
- Keep the Rust crate in `cli/`; installed skills contain only the compiled CLI at `bin/sok`.

## Validation

- Run `make test` after changing skill instructions, references, specifications, or CLI behavior.
- Run `make vet` after changing Rust code.
- Run `make test-install` after changing installation or distribution behavior.
- Run `git diff --check` before committing.
