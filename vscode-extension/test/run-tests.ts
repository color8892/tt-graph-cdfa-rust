import * as assert from "assert";
import * as path from "path";
import { defaultAnalyzerPath, diagnosticsCommandForPath } from "../src/analyzer";
import { renderMermaid } from "../src/graphView";
import { AnalyzerResult, parseAnalyzerResult } from "../src/schema";

function run(): void {
  testAnalyzerPath();
  testCommandSelection();
  testJsonParsing();
  testMermaidRendering();
  console.log("VS Code extension unit tests passed");
}

function testAnalyzerPath(): void {
  assert.strictEqual(
    defaultAnalyzerPath("/repo", "linux"),
    path.join("/repo", "target", "debug", "tt-graph-cdfa-rust"),
  );
  assert.strictEqual(
    defaultAnalyzerPath("C:\\repo", "win32"),
    path.join("C:\\repo", "target", "debug", "tt-graph-cdfa-rust.exe"),
  );
}

function testCommandSelection(): void {
  assert.strictEqual(diagnosticsCommandForPath("program1.cpp"), "diagnostics-cpp");
  assert.strictEqual(
    diagnosticsCommandForPath("program1_plain.cpp"),
    "diagnostics-cpp-implicit",
  );
}

function testJsonParsing(): void {
  const result = parseAnalyzerResult(JSON.stringify(sampleResult()));
  assert.strictEqual(result.schema_version, 1);
  assert.strictEqual(result.diagnostics[0].cca_type, "WriteRead");
  assert.throws(() => parseAnalyzerResult("{"), /valid JSON/);
}

function testMermaidRendering(): void {
  const mermaid = renderMermaid(sampleResult());
  assert.match(mermaid, /flowchart TD/);
  assert.match(mermaid, /Act1/);
  assert.match(mermaid, /cca-WriteRead/);
  assert.match(mermaid, /click Act1 call nodeClicked\(\)/);
}

function sampleResult(): AnalyzerResult {
  return {
    schema_version: 1,
    source: { path: "examples/program1.cpp", language: "cpp" },
    graph: {
      nodes: [
        {
          id: "Act1",
          node_type: "Activity",
          control_type: null,
          label: "Act1",
          source: { file: "examples/program1.cpp", line: 10, column: 3 },
        },
        {
          id: "Act2",
          node_type: "Activity",
          control_type: null,
          label: "Act2",
          source: { file: "examples/program1.cpp", line: 20, column: 3 },
        },
      ],
      edges: [{ from: "Act1", to: "Act2", type: "cca:WriteRead" }],
    },
    diagnostics: [
      {
        severity: "warning",
        cca_type: "WriteRead",
        variable: "v",
        message: "Concurrent dataflow anomaly WriteRead on variable v",
        first: { node: "Act1", file: "examples/program1.cpp", line: 10, column: 3 },
        second: { node: "Act2", file: "examples/program1.cpp", line: 20, column: 3 },
      },
    ],
  };
}

run();
