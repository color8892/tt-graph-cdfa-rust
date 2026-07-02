import * as assert from "assert";
import * as path from "path";
import { defaultAnalyzerPath, diagnosticsCommandForPath } from "../src/analyzer";
import { renderGraphHtml, renderMermaid, renderMermaidGraph } from "../src/graphView";
import { findGraphNode } from "../src/resultLookup";
import { AnalyzerResult, parseAnalyzerResult } from "../src/schema";

function run(): void {
  testAnalyzerPath();
  testCommandSelection();
  testJsonParsing();
  testMermaidRendering();
  testMermaidEscapingAndSafeIds();
  testGraphHtmlUsesLocalMermaidAndCsp();
  testLatestResultLookupUsesUpdatedResult();
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
  assert.match(mermaid, /click node_0 nodeClicked/);
}

function testMermaidEscapingAndSafeIds(): void {
  const result = sampleResult();
  result.graph.nodes[0].id = "1 Act/with:bad[id]";
  result.graph.nodes[0].label = "Act 1 [x] \"quoted\" | pipe <tag> & value\nnext";
  result.graph.edges[0].from = "1 Act/with:bad[id]";
  result.graph.edges[0].type = "cca:WriteRead|bad\"<edge>";

  const rendered = renderMermaidGraph(result);

  assert.match(
    rendered.code,
    /node_0\["Act 1 &#91;x&#93; #quot;quoted#quot; &#124; pipe &lt;tag&gt; &amp; value<br\/>next"\]/,
  );
  assert.match(
    rendered.code,
    /node_0 -\.->\|"cca-WriteRead&#124;bad#quot;&lt;edge&gt;"\| node_1/,
  );
  assert.strictEqual(rendered.nodeIdByMermaidId.node_0, "1 Act/with:bad[id]");
  assert.doesNotMatch(rendered.code, /1 Act\/with:bad\[id\] -\.->/);
  assert.doesNotMatch(rendered.code, /<tag>/);
  assert.doesNotMatch(rendered.code, /"<edge>"/);
}

function testGraphHtmlUsesLocalMermaidAndCsp(): void {
  const html = renderGraphHtml(sampleResult(), {
    mermaidScriptUri: "vscode-resource://extension/node_modules/mermaid/dist/mermaid.esm.min.mjs",
    graphStylesUri: "vscode-resource://extension/media/graphView.css",
    nonce: "testNonce",
    webviewCspSource: "vscode-resource://extension",
  });

  assert.match(html, /Content-Security-Policy/);
  assert.match(html, /style-src vscode-resource:\/\/extension;/);
  assert.match(html, /media\/graphView\.css/);
  assert.match(html, /nonce="testNonce"/);
  assert.match(html, /nodeIdByMermaidId/);
  assert.match(html, /mermaid\.esm\.min\.mjs/);
  assert.doesNotMatch(html, /<style>/);
  assert.doesNotMatch(html, /style="/);
  assert.doesNotMatch(html, /unsafe-inline/);
  assert.doesNotMatch(html, /cdn\.jsdelivr\.net/);
  assert.doesNotMatch(html, /https:\/\/.*mermaid/);
}

function testLatestResultLookupUsesUpdatedResult(): void {
  let latest: AnalyzerResult | undefined = sampleResult();
  assert.strictEqual(findGraphNode(latest, "Act1")?.source?.line, 10);

  latest = sampleResult();
  latest.graph.nodes[0].source = { file: "examples/program2.cpp", line: 42, column: 1 };

  assert.strictEqual(findGraphNode(latest, "Act1")?.source?.file, "examples/program2.cpp");
  assert.strictEqual(findGraphNode(latest, "Act1")?.source?.line, 42);
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
