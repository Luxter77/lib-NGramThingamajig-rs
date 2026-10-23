# lib-NGramThingamajig-rs

lib-NGramThingamajig-rs es un byte-level n-gram sequence model written in rust.

Project-specific rules, formatting guidelines, and skill definitions live in `.agents/`.

See `.agents/rules/` for agent communication style, date/time formats, typography conventions, writing style, and git file-move rules.

See `.agents/skills/` for mode-specific instructions:

- `bun-instead-of-node` for use Bun for all runtime operations.
- `caveman` for ultra-compressed response mode.
- `dt-technical-writer` for Distilled Technical writing.
- `lint-md` for markdownlint workflow.
- `obsidian-markdown` for Obsidian Flavored Markdown syntax.

## Markdown linting

`.markdownlint-cli2.jsonc` configures markdownlint-cli2.

Run `bun x markdownlint-cli2 --fix "**/*.md"` from the project root.

See `.agents/skills/lint-md/SKILL.md` for the full workflow.

## VS Code

Recommended extensions in `.vscode/extensions.json`.

Settings enable on-type markdownlint auto-fix.
