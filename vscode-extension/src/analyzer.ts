import * as cp from "child_process";
import * as fs from "fs";
import * as path from "path";
import { AnalyzerResult, parseAnalyzerResult } from "./schema";

export interface AnalyzerRunOptions {
  analyzerPath: string;
  workspaceRoot: string;
  sourcePath: string;
}

export function defaultAnalyzerPath(workspaceRoot: string, platform: NodeJS.Platform = process.platform): string {
  const binaryName = platform === "win32" ? "tt-graph-cdfa-rust.exe" : "tt-graph-cdfa-rust";
  return path.join(workspaceRoot, "target", "debug", binaryName);
}

export function resolveAnalyzerPath(configuredPath: string | undefined, workspaceRoot: string): string {
  const candidate = configuredPath && configuredPath.trim().length > 0
    ? configuredPath
    : defaultAnalyzerPath(workspaceRoot);
  if (!fs.existsSync(candidate)) {
    throw new Error(`Analyzer binary not found at ${candidate}. Build it with: cargo build`);
  }
  return candidate;
}

export function diagnosticsCommandForPath(sourcePath: string): string {
  const baseName = path.basename(sourcePath).toLowerCase();
  return baseName.includes("plain") ? "diagnostics-cpp-implicit" : "diagnostics-cpp";
}

export function runAnalyzer(options: AnalyzerRunOptions): Promise<AnalyzerResult> {
  const command = diagnosticsCommandForPath(options.sourcePath);
  return new Promise((resolve, reject) => {
    cp.execFile(
      options.analyzerPath,
      [command, options.sourcePath],
      { cwd: options.workspaceRoot, maxBuffer: 10 * 1024 * 1024 },
      (error, stdout, stderr) => {
        let result: AnalyzerResult;
        try {
          result = parseAnalyzerResult(stdout);
        } catch (parseError) {
          reject(new Error(`${String(parseError)}${stderr ? `\n${stderr}` : ""}`));
          return;
        }

        if (error || result.error) {
          const detailedError = result.error
            ? `${result.error}${stderr.trim() ? `\n\nClang Diagnostics:\n${stderr.trim()}` : ""}`
            : (stderr.trim() || error?.message || "Analyzer failed");
          reject(new Error(detailedError));
          return;
        }

        resolve(result);
      },
    );
  });
}
