const fs = require("fs");
const path = require("path");

const repoRoot = path.resolve(__dirname, "..");
const docFiles = [
  "README.md",
  "BENCHMARK.md",
  "CHANGELOG.md",
  "CONTRIBUTING.md",
  "PAPER_REPRODUCTION.md",
  "SECURITY.md",
  path.join("docs", "artifact-schema.md"),
  path.join("docs", "paper-mapping.md"),
  path.join("vscode-extension", "README.md"),
  path.join(".github", "PULL_REQUEST_TEMPLATE.md"),
];

const linkPattern = /!?\[[^\]]*]\(([^)]+)\)/g;
const missing = [];

for (const relativeFile of docFiles) {
  const filePath = path.join(repoRoot, relativeFile);
  if (!fs.existsSync(filePath)) {
    missing.push(`${relativeFile}: documentation file is missing`);
    continue;
  }

  const content = fs.readFileSync(filePath, "utf8");
  for (const match of content.matchAll(linkPattern)) {
    const rawTarget = normalizeMarkdownTarget(match[1]);
    if (!rawTarget || shouldSkip(rawTarget)) {
      continue;
    }

    const [targetPath] = rawTarget.split("#", 1);
    if (!targetPath) {
      continue;
    }

    const decodedTarget = decodeURIComponent(targetPath);
    const resolvedTarget = path.resolve(path.dirname(filePath), decodedTarget);
    if (!isInsideRepo(resolvedTarget) || !fs.existsSync(resolvedTarget)) {
      missing.push(`${relativeFile}: missing link target ${rawTarget}`);
    }
  }
}

if (missing.length > 0) {
  console.error("Documentation link check failed:");
  for (const item of missing) {
    console.error(`- ${item}`);
  }
  process.exit(1);
}

console.log(`Checked links in ${docFiles.length} documentation files`);

function normalizeMarkdownTarget(value) {
  let target = value.trim();
  if (target.startsWith("<") && target.endsWith(">")) {
    target = target.slice(1, -1);
  }
  return target.split(/\s+/, 1)[0];
}

function shouldSkip(target) {
  return (
    target.startsWith("#") ||
    /^[a-z][a-z0-9+.-]*:/i.test(target)
  );
}

function isInsideRepo(targetPath) {
  const relative = path.relative(repoRoot, targetPath);
  return relative === "" || (!relative.startsWith("..") && !path.isAbsolute(relative));
}
