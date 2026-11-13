# Structure of Knowledge (SoK)

SoK helps you investigate a field by connecting its central questions, ideas, and
methods to the evidence behind them. Use it to enter an unfamiliar discipline,
evaluate a collection of documents, or plan what to read and learn next.

The research skill guides an agent through source review and writing for a
particular reader. A local Rust CLI checks the supporting records and produces
a self-contained HTML report and reusable JSON. The project's educational
foundations are described in the [blueprint](https://github.com/perhyper/SoK/blob/main/BLUEPRINT.md).

## Install

On macOS or Linux, install the skill and CLI with Rust, Cargo, `curl`, and `tar`:

```bash
curl -fsSL https://raw.githubusercontent.com/perhyper/SoK/main/install.sh | sh
```

For a specific release, pass `--ref <tag>` to the installer. From a local
checkout, run `make install`.

## Start a research task

Give the agent a question, an audience, and the result you want. For example:

> Use the structure-of-knowledge skill to introduce computational neuroscience
> to a technical reader. How do researchers decide between competing models of
> the same behavior? Explain what experimental evidence can distinguish them,
> then suggest a focused reading path.

For a curriculum, include the learner's starting point and available time.
For a document review, say whether the analysis should draw on outside scholarship.

## Outputs

| File | What it contains |
|---|---|
| `report.html` | A self-contained report to read or share. |
| `sok-report.json` | The structured report for reuse in other tools. |
| `sources.csv` | The sources consulted, their roles, and how to access them. |
| `reviewed-evidence.jsonl` | Evidence linking individual claims to reviewed sources. |

To build the report from a completed research workspace containing `report.md`,
`sources.csv`, and `reviewed-evidence.jsonl`, run:

```bash
SOK_CLI="${CODEX_HOME:-$HOME/.codex}/skills/structure-of-knowledge/bin/sok"
"$SOK_CLI" export-json --stage final --report report.md \
  --sources sources.csv --evidence reviewed-evidence.jsonl --output sok-report.json
"$SOK_CLI" validate-report --input sok-report.json --strict
"$SOK_CLI" render-html --input sok-report.json --output report.html
```

For other commands, including scaffolding a new investigation and exporting a
graph across reports, see the [CLI guide](structure-of-knowledge/references/cli-workflow.md).

## License

[MIT](LICENSE).
