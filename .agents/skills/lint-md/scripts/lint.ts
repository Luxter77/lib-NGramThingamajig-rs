#!/usr/bin/env bun

/**
 * Comprehensive lint script for app-terminal-linux.
 * Runs markdownlint with --fix, then scans all markdown files for issues
 * markdownlint does not catch: typographic problems (em/en dashes, curly
 * quotes, bold, asterisk italics), YAML frontmatter validity, absolute-path
 * links without protocol, wikilink balance, and unescaped patterns.
 */

/// <reference types="bun-types" />

import { glob } from "glob";
import { readFileSync } from "fs";
import { resolve } from "path";
import { fileURLToPath } from "url";
import { dirname } from "path";
// @ts-ignore - Bun provides built-in yaml module at runtime
import { parse as parseYAML } from "yaml";

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);

function findProjectRoot(startDir: string): string {
  let current = startDir;
  while (current !== "/") {
    try {
      if (
        readFileSync(resolve(current, "AGENTS.md"), "utf-8").length > 0 ||
        readFileSync(resolve(current, ".git"), "utf-8").length > 0
      ) {
        return current;
      }
    } catch {
      // File doesn't exist, continue searching
    }
    current = resolve(current, "..");
  }
  return startDir;
}

const ROOT = findProjectRoot(__dirname);

// Files excluded from typographic checks (they contain intentional examples)
const TYPOGRAPHIC_EXCLUDE = [
  ".agents/skills/lint-md/SKILL.md",
  "AGENTS.md",
  "CLAUDE.md",
];

interface LintIssue {
  file: string;
  line: number;
  column: number;
  rule: string;
  message: string;
  severity: "error" | "warning";
}

const issues: LintIssue[] = [];

function addIssue(
  file: string,
  line: number,
  column: number,
  rule: string,
  message: string,
  severity: "error" | "warning" = "error"
) {
  issues.push({ file, line, column, rule, message, severity });
}

function checkFrontmatter(file: string, content: string) {
  const frontmatterMatch = content.match(/^---\n([\s\S]*?)\n--/);
  if (!frontmatterMatch) {
    return;
  }

  const frontmatterContent = frontmatterMatch[1];
  const frontmatterStartLine = 1;

  try {
    parseYAML(frontmatterContent);
  } catch (error) {
    if (error instanceof Error) {
      const lineMatch = error.message.match(/line (\d+)/i);
      const lineNum = lineMatch ? parseInt(lineMatch[1]) + frontmatterStartLine : frontmatterStartLine + 1;
      addIssue(file, lineNum, 1, "YML001", `Invalid YAML frontmatter: ${error.message}`);
    } else {
      addIssue(file, frontmatterStartLine + 1, 1, "YML001", `Invalid YAML frontmatter: ${String(error)}`);
    }
    return;
  }

  const frontmatterLines = frontmatterContent.split("\n");
  const seenKeys = new Map<string, number>();
  for (let i = 0; i < frontmatterLines.length; i++) {
    const line = frontmatterLines[i].trim();
    if (!line || line.startsWith("#")) continue;
    const keyMatch = line.match(/^([a-zA-Z_][a-zA-Z0-9_-]*):/);
    if (keyMatch) {
      const key = keyMatch[1];
      const fileLineNum = i + frontmatterStartLine + 1;
      if (seenKeys.has(key)) {
        const firstLine = seenKeys.get(key)!;
        addIssue(file, fileLineNum, 1, "YML001", `Duplicate frontmatter key: "${key}" (first seen at line ${firstLine})`);
      } else {
        seenKeys.set(key, fileLineNum);
      }
    }
  }
}

function checkFile(file: string, content: string) {
  const isTypoExcluded = TYPOGRAPHIC_EXCLUDE.includes(file);

  checkFrontmatter(file, content);

  const lines = content.split("\n");

  const wikiOpens = (content.match(/\[\[/g) || []).length;
  const wikiCloses = (content.match(/\]\]/g) || []).length;
  if (wikiOpens !== wikiCloses) {
    addIssue(
      file,
      1,
      1,
      "WIK001",
      `Unbalanced wikilink brackets: ${Math.abs(wikiOpens - wikiCloses)} ${wikiOpens > wikiCloses ? "unclosed" : "extra closing"} [[]]`,
      "warning"
    );
  }

  const mdLinks = content.match(/(?<!!)\[([^\]]+)\]\(([^)]+)\)/g) || [];
  const mdImages = content.match(/!\[([^\]]+)\]\(([^)]+)\)/g) || [];
  const totalMdCloses = (content.match(/\]\(/g) || []).length;
  const totalMdOpens = mdLinks.length + mdImages.length;
  if (totalMdOpens !== totalMdCloses) {
    addIssue(
      file,
      1,
      1,
      "WIK002",
      `Unbalanced markdown link/image brackets: ${Math.abs(totalMdOpens - totalMdCloses)} ${totalMdOpens > totalMdCloses ? "unclosed" : "extra closing"} []() / ![]()`
    );
  }

  const rootLinkRegex = /\[([^\]]+)\]\((\/[^)]+)\)/g;
  let match;
  while ((match = rootLinkRegex.exec(content)) !== null) {
    const lineNum = content.substring(0, match.index).split("\n").length;
    const column = match.index - content.lastIndexOf("\n", match.index - 1);
    addIssue(
      file,
      lineNum,
      column,
      "WIK003",
      `Markdown link uses absolute path without protocol: [${match[1]}](/${match[2]}) - use full URL (https://...) or relative path`
    );
  }

  const wikiPathRegex = /\[\[[^\]]*\/[^\]]*\]\]/g;
  while ((match = wikiPathRegex.exec(content)) !== null) {
    const lineNum = content.substring(0, match.index).split("\n").length;
    const column = match.index - content.lastIndexOf("\n", match.index - 1);
    addIssue(
      file,
      lineNum,
      column,
      "WIK004",
      `Wikilink uses path-like format: ${match[0]} - use just the page name without directory components`,
      "warning"
    );
  }

  const aoPatternRegex = /(?<!\\)#AO\d+/g;
  while ((match = aoPatternRegex.exec(content)) !== null) {
    const lineNum = content.substring(0, match.index).split("\n").length;
    const column = match.index - content.lastIndexOf("\n", match.index - 1);
    addIssue(
      file,
      lineNum,
      column,
      "ESC001",
      `Unescaped identifier: ${match[0]} - escape as \\\\${match[0]} to prevent tag interpretation`
    );
  }

  const hexColorRegex = /(?<!\\)#([0-9a-fA-F]{6})\b/g;
  while ((match = hexColorRegex.exec(content)) !== null) {
    const lineNum = content.substring(0, match.index).split("\n").length;
    const column = match.index - content.lastIndexOf("\n", match.index - 1);
    addIssue(
      file,
      lineNum,
      column,
      "ESC002",
      `Unescaped hex color: ${match[0]} - escape as \\\\${match[0]} to prevent tag interpretation`
    );
  }

  if (!isTypoExcluded) {
    lines.forEach((line, idx) => {
      const lineNum = idx + 1;

      const emDashRegex = /—/g;
      while ((match = emDashRegex.exec(line)) !== null) {
        addIssue(
          file,
          lineNum,
          match.index + 1,
          "TYP001",
          "Em dash (—) found - use standard dash (-) instead, or better, reword the entire sentence."
        );
      }

      const enDashRegex = /–/g;
      while ((match = enDashRegex.exec(line)) !== null) {
        addIssue(
          file,
          lineNum,
          match.index + 1,
          "TYP002",
          "En dash (–) found - use standard dash (-) instead, or better, reword the entire sentence."
        );
      }

      const curlyDoubleRegex = /[“”]/g;
      while ((match = curlyDoubleRegex.exec(line)) !== null) {
        addIssue(
          file,
          lineNum,
          match.index + 1,
          "TYP003",
          "Curly/smart double quote found - use straight quote (\") instead"
        );
      }

      const curlySingleRegex = /[‘’]/g;
      while ((match = curlySingleRegex.exec(line)) !== null) {
        addIssue(
          file,
          lineNum,
          match.index + 1,
          "TYP004",
          "Curly/smart single quote found - use straight quote (') instead"
        );
      }

      const boldRegex = /\*\*[^*]+\*\*/g;
      while ((match = boldRegex.exec(line)) !== null) {
        addIssue(
          file,
          lineNum,
          match.index + 1,
          "TYP005",
          "Bold (**text**) found - use headings, lists, callouts, or wikilinks instead"
        );
      }

      const asteriskItalicRegex = /(?<!\*)\*[^*]+\*(?!\*)/g;
      while ((match = asteriskItalicRegex.exec(line)) !== null) {
        const beforeMatch = line.substring(0, match.index).trimStart();
        if (beforeMatch.length === 0 || beforeMatch.startsWith("- ") || beforeMatch.startsWith("* ")) {
          continue;
        }
        addIssue(
          file,
          lineNum,
          match.index + 1,
          "TYP006",
          "Asterisk italics (*text*) found - use underscores (_text_) instead"
        );
      }
    });
  }
}

async function runMarkdownlint() {
  console.log("Running markdownlint --fix...");
  const { $ } = await import("bun");
  try {
    const result = await $`bun x markdownlint-cli2 --fix --dot ".agents/**/*.md"`.cwd(ROOT).quiet().nothrow();
    const stdout = result.stdout.toString();
    const stderr = result.stderr.toString().trimEnd();
    if (stdout) {
      console.log(stdout);
    }
    if (stderr) {
      const label = result.exitCode === 1 ? "markdownlint reported issues" : "markdownlint command failed";
      console.error(`${label}:\n${stderr.split("\n").map((line) => `  ${line}`).join("\n")}`);
    } else if (result.exitCode !== 0) {
      console.error(`markdownlint exited with code ${result.exitCode} without stderr output.`);
    }
    return result.exitCode === 0;
  } catch (error) {
    console.error("markdownlint failed:", error);
    return false;
  }
}

async function findMarkdownFiles() {
  const pattern = ".agents/**/*.md";
  const ignores = [
    "**/node_modules/**",
    "**/.git/**",
    "**/.obsidian/**",
    "**/vendor/**",
    ".agents/skills/lint-md/**/*.md",
  ];
  const files = await glob(pattern, {
    cwd: ROOT,
    ignore: ignores,
    absolute: true,
    dot: true,
  });
  return files;
}

async function main() {
  console.log("═══════════════════════════════════════");
  console.log("  app-terminal-linux Lint Check");
  console.log("═══════════════════════════════════════\n");

  const markdownlintOk = await runMarkdownlint();
  console.log("");

  console.log("Scanning for typographic and structural issues...");
  const files = await findMarkdownFiles();
  console.log(`Found ${files.length} markdown files to check\n`);

  for (const file of files) {
    const content = readFileSync(file, "utf-8");
    const relativePath = file.replace(ROOT + "/", "");
    checkFile(relativePath, content);
  }

  console.log("═══════════════════════════════════════");
  console.log("  Lint Results");
  console.log("═══════════════════════════════════════\n");

  if (issues.length === 0) {
    console.log("✓ No typographic issues found!");
  } else {
    const byFile = new Map<string, LintIssue[]>();
    for (const issue of issues) {
      if (!byFile.has(issue.file)) {
        byFile.set(issue.file, []);
      }
      byFile.get(issue.file)!.push(issue);
    }

    for (const [file, fileIssues] of byFile) {
      console.log(`\n${file}:`);
      for (const issue of fileIssues) {
        const prefix = issue.severity === "error" ? "✗" : "⚠";
        console.log(`  ${prefix} Line ${issue.line}:${issue.column} [${issue.rule}] ${issue.message}`);
      }
    }

    console.log(`\nTotal: ${issues.length} issue(s) found`);
  }

  console.log("\n═══════════════════════════════════════");
  const errors = issues.filter((i) => i.severity === "error").length;
  if (!markdownlintOk || errors > 0) {
    console.log("  LINT FAILED");
    console.log("═══════════════════════════════════════");
    process.exit(1);
  } else {
    console.log("  LINT PASSED");
    console.log("═══════════════════════════════════════");
    process.exit(0);
  }
}

main().catch((err) => {
  console.error("Fatal error:", err);
  process.exit(1);
});
