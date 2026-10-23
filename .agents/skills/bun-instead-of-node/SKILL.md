---
name: bun-instead-of-node
description: "Skill for using Bun runtime instead of Node.js. Use in the following situations: (1) Creating new JavaScript/TypeScript projects, (2) Working with package.json or dependencies, (3) Running scripts or starting dev servers, (4) Converting Node.js projects to Bun, (5) Tasks containing 'bun', 'bun x', 'bun.lockb' keywords"
---
# Bun

Use Bun as the default runtime instead of Node.js.

## Installation

```bash
# macOS / Linux
brew install oven-sh/bun/bun
```

## Command Mapping

| Node.js / npm           | Bun                                    |
| ----------------------- | -------------------------------------- |
| `npm install`           | `bun install`                          |
| `npm add <pkg>`         | `bun add <pkg>`                        |
| `npm add -D <pkg>`      | `bun add -d <pkg>`                     |
| `npm remove <pkg>`      | `bun remove <pkg>`                     |
| `npm run <script>`      | `bun run <script>` or `bun <script>`   |
| `npx <cmd>`             | `bun x <cmd>`                          |
| `node <file>`           | `bun <file>`                           |
| `npm init`              | `bun init`                             |
| `npm create <template>` | `bun create <template>`                |

## Project Initialization

```bash
# New project
bun init

# Using templates
bun create next-app my-app
bun create vite my-app
```

## Package Management

```bash
# Install (creates bun.lockb)
bun install

# Add dependencies
bun add express zod
bun add -d typescript @types/node  # devDependencies

# Remove
bun remove lodash
```

> lockfile: `bun.lockb` (binary). Do not add to `.gitignore`.

## Running Scripts

```bash
# package.json scripts
bun run dev
bun run build

# Run file directly (TypeScript supported)
bun index.ts
bun src/server.ts

# Shorthand (run can be omitted)
bun dev
```

## Precautions

### 1. Verify Package Name Before Installing

```bash
# Typo or similar package - typosquatting risk
bun add expres
bun add lodassh

# Verify exact package name before installing
bun add express
bun add lodash
```

Verify package name is exact before running `bun add`. Malicious packages with similar names may be registered on npm registry.

### 2. Do Not Run Untrusted Scripts

Avoid executing remote scripts or unverified files directly. Only run scripts within your local project.

## GitHub Actions

```yaml
- uses: oven-sh/setup-bun@v{N} # Check latest version: gh api repos/oven-sh/setup-bun/releases/latest --jq '.tag_name'
- run: bun install
- run: bun test
```
