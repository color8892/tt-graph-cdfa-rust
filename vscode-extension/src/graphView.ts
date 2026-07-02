import { AnalyzerResult } from "./schema";

export interface GraphHtmlOptions {
  mermaidScriptUri: string;
  graphStylesUri: string;
  nonce: string;
  webviewCspSource: string;
}

export function renderGraphHtml(result: AnalyzerResult, options: GraphHtmlOptions): string {
  const renderedGraph = renderMermaidGraph(result);

  return `<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <meta http-equiv="Content-Security-Policy" content="default-src 'none'; img-src ${options.webviewCspSource} data:; style-src ${options.webviewCspSource}; script-src ${options.webviewCspSource} 'nonce-${options.nonce}';">
  <link rel="stylesheet" href="${options.graphStylesUri}">
</head>
<body>
  <h1>TT Graph CDFA: ${escapeHtml(result.source.path)}</h1>
  <div class="graph-container">
    <pre class="mermaid">
${renderedGraph.code}
    </pre>
  </div>
  
  ${renderSummaryTable(result)}

  <script nonce="${options.nonce}" type="module">
    import mermaid from '${options.mermaidScriptUri}';
    const nodeIdByMermaidId = ${JSON.stringify(renderedGraph.nodeIdByMermaidId)};
    
    const isDark = document.body.classList.contains('vscode-dark') || 
                   document.body.classList.contains('vscode-high-contrast');
                   
    mermaid.initialize({
      startOnLoad: true,
      securityLevel: 'loose',
      theme: isDark ? 'dark' : 'neutral',
      flowchart: {
        useMaxWidth: true,
        htmlLabels: true,
        curve: 'basis'
      }
    });

    const vscode = acquireVsCodeApi();
    window.nodeClicked = function(mermaidNodeId) {
      const nodeId = nodeIdByMermaidId[mermaidNodeId] || mermaidNodeId;
      vscode.postMessage({ type: "openNode", nodeId: nodeId });
    };
  </script>
</body>
</html>`;
}

export function renderMarkdown(result: AnalyzerResult): string {
  const mermaidCode = renderMermaid(result, false);
  const sourceName = result.source.path.split(/[/\\]/).pop() || result.source.path;
  
  let md = `# TT Graph CDFA Analysis: ${sourceName}\n\n`;
  md += `**Source File**: \`${result.source.path}\`  \n`;
  md += `**Language**: \`${result.source.language}\`  \n\n`;
  
  md += `## Anomaly Diagnostics\n\n`;
  if (result.diagnostics.length === 0) {
    md += `No concurrent dataflow anomalies detected.  \n\n`;
  } else {
    md += `| Anomaly Type | Variable | First Endpoint | Second Endpoint |\n`;
    md += `| --- | --- | --- | --- |\n`;
    for (const diag of result.diagnostics) {
      md += `| **${diag.cca_type}** | \`${diag.variable}\` | \`${diag.first.node}\` (line ${diag.first.line}) | \`${diag.second.node}\` (line ${diag.second.line}) |\n`;
    }
    md += `\n`;
  }
  
  md += `## Task-Transaction Graph\n\n`;
  md += `\`\`\`mermaid\n`;
  md += mermaidCode;
  md += `\`\`\`\n`;
  
  return md;
}

export function renderMermaid(result: AnalyzerResult, includeClickHandlers: boolean = true): string {
  return renderMermaidGraph(result, includeClickHandlers).code;
}

export function renderMermaidGraph(
  result: AnalyzerResult,
  includeClickHandlers: boolean = true,
): { code: string; nodeIdByMermaidId: Record<string, string> } {
  let mermaid = "flowchart TD\n";
  const nodeIdByMermaidId: Record<string, string> = {};
  const mermaidIdByNodeId = new Map<string, string>();

  result.graph.nodes.forEach((node, index) => {
    const mermaidId = `node_${index}`;
    mermaidIdByNodeId.set(node.id, mermaidId);
    nodeIdByMermaidId[mermaidId] = node.id;
  });

  // Render nodes
  for (const node of result.graph.nodes) {
    const id = mermaidIdByNodeId.get(node.id) ?? safeMermaidId(node.id);
    const label = escapeMermaidLabel(node.label);
    if (node.node_type === "Control") {
      // Diamond shape for control
      mermaid += `  ${id}{"${label}"}\n`;
      mermaid += `  class ${id} control\n`;
    } else if (node.node_type === "Block") {
      // Rect shape for block
      mermaid += `  ${id}["${label}"]\n`;
      mermaid += `  class ${id} block\n`;
    } else {
      // Rect shape for activity
      mermaid += `  ${id}["${label}"]\n`;
      mermaid += `  class ${id} activity\n`;
    }
  }

  // Render edges
  let linkIndex = 0;
  const linkStyles: string[] = [];
  
  for (const edge of result.graph.edges) {
    const from = mermaidIdByNodeId.get(edge.from) ?? safeMermaidId(edge.from);
    const to = mermaidIdByNodeId.get(edge.to) ?? safeMermaidId(edge.to);
    const cleanType = escapeMermaidEdgeLabel(edge.type.replace(/:/g, "-"));
    if (edge.type === "sequence") {
      mermaid += `  ${from} --> ${to}\n`;
    } else if (edge.type === "branch") {
      mermaid += `  ${from} -->|"branch"| ${to}\n`;
    } else if (edge.type === "scope") {
      mermaid += `  ${from} -.->|"scope"| ${to}\n`;
    } else if (edge.type.startsWith("cca:")) {
      mermaid += `  ${from} -.->|"${cleanType}"| ${to}\n`;
      linkStyles.push(`  linkStyle ${linkIndex} stroke:#d73a49,stroke-width:2px`);
    } else {
      mermaid += `  ${from} -->|"${cleanType}"| ${to}\n`;
    }
    linkIndex++;
  }

  // Click bindings for interactive jumping
  if (includeClickHandlers) {
    for (const node of result.graph.nodes) {
      const id = mermaidIdByNodeId.get(node.id) ?? safeMermaidId(node.id);
      mermaid += `  click ${id} nodeClicked\n`;
    }
  }

  // Link styles
  if (linkStyles.length > 0) {
    mermaid += "\n" + linkStyles.join("\n") + "\n";
  }

  return { code: mermaid, nodeIdByMermaidId };
}

function renderSummaryTable(result: AnalyzerResult): string {
  if (result.diagnostics.length === 0) {
    return `<div class="empty-diagnostics">No concurrent dataflow anomalies detected.</div>`;
  }

  const rows = result.diagnostics.map((diagnostic) => `<tr>
    <td><span class="diagnostic-type">${escapeHtml(diagnostic.cca_type)}</span></td>
    <td><code>${escapeHtml(diagnostic.variable)}</code></td>
    <td><code>${escapeHtml(diagnostic.first.node)}</code> (line ${diagnostic.first.line})</td>
    <td><code>${escapeHtml(diagnostic.second.node)}</code> (line ${diagnostic.second.line})</td>
  </tr>`).join("\n");
  
  return `<table>
    <thead>
      <tr>
        <th>Anomaly Type</th>
        <th>Variable</th>
        <th>First Endpoint</th>
        <th>Second Endpoint</th>
      </tr>
    </thead>
    <tbody>
      ${rows}
    </tbody>
  </table>`;
}

function escapeHtml(value: string): string {
  return value
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;");
}

function escapeMermaidLabel(value: string): string {
  return value
    .replace(/&/g, "&amp;")
    .replace(/\\/g, "\\\\")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "#quot;")
    .replace(/\[/g, "&#91;")
    .replace(/\]/g, "&#93;")
    .replace(/\{/g, "&#123;")
    .replace(/\}/g, "&#125;")
    .replace(/\|/g, "&#124;")
    .replace(/\r?\n/g, "<br/>");
}

function escapeMermaidEdgeLabel(value: string): string {
  return escapeMermaidLabel(value).replace(/<br\/>/g, " ");
}

function safeMermaidId(value: string): string {
  const safe = value.replace(/[^A-Za-z0-9_]/g, "_");
  return /^[A-Za-z_]/.test(safe) ? safe : `node_${safe}`;
}
