import { AnalyzerResult } from "./schema";

export function renderGraphHtml(result: AnalyzerResult): string {
  const mermaidCode = renderMermaid(result);

  return `<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <style>
    body {
      font-family: var(--vscode-font-family);
      color: var(--vscode-foreground);
      padding: 16px;
      background: var(--vscode-editor-background);
    }
    h1 {
      font-size: 16px;
      margin: 0 0 16px;
      border-bottom: 1px solid var(--vscode-panel-border);
      padding-bottom: 8px;
    }
    .graph-container {
      width: 100%;
      border: 1px solid var(--vscode-panel-border);
      background: var(--vscode-editor-background);
      border-radius: 4px;
      padding: 12px;
      box-sizing: border-box;
      overflow: auto;
    }
    
    /* Custom styles for Mermaid nodes that respect VS Code themes */
    .node.activity rect, .node.activity polygon {
      fill: #cfe8ff !important;
      stroke: #1f6feb !important;
      stroke-width: 1.5px !important;
    }
    .node.control polygon {
      fill: #f2c94c !important;
      stroke: #8a6d00 !important;
      stroke-width: 1.5px !important;
    }
    .node.block rect {
      fill: #b7e4c7 !important;
      stroke: #2d6a4f !important;
      stroke-width: 1.5px !important;
    }
    
    /* Dark theme styling override */
    .vscode-dark .node.activity rect, .vscode-dark .node.activity polygon,
    .vscode-high-contrast .node.activity rect, .vscode-high-contrast .node.activity polygon {
      fill: #1f3a60 !important;
      stroke: #58a6ff !important;
    }
    .vscode-dark .node.control polygon,
    .vscode-high-contrast .node.control polygon {
      fill: #6e5600 !important;
      stroke: #f2c94c !important;
    }
    .vscode-dark .node.block rect,
    .vscode-high-contrast .node.block rect {
      fill: #1b382b !important;
      stroke: #3fb950 !important;
    }
    
    .node text {
      fill: var(--vscode-editor-foreground, #111) !important;
      font-weight: 500 !important;
    }
    
    table {
      border-collapse: collapse;
      margin-top: 24px;
      width: 100%;
    }
    th, td {
      border-bottom: 1px solid var(--vscode-panel-border);
      padding: 8px 10px;
      text-align: left;
      font-size: 12px;
    }
    th {
      font-weight: 600;
      background: var(--vscode-panel-background);
    }
    tr:hover {
      background: var(--vscode-list-hoverBackground);
    }
  </style>
</head>
<body>
  <h1>TT Graph CDFA: ${escapeHtml(result.source.path)}</h1>
  <div class="graph-container">
    <pre class="mermaid">
${mermaidCode}
    </pre>
  </div>
  
  ${renderSummaryTable(result)}

  <script type="module">
    import mermaid from 'https://cdn.jsdelivr.net/npm/mermaid@10/dist/mermaid.esm.min.mjs';
    
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
    window.nodeClicked = function(nodeId) {
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
  let mermaid = "flowchart TD\n";

  // Render nodes
  for (const node of result.graph.nodes) {
    const label = node.label;
    if (node.node_type === "Control") {
      // Diamond shape for control
      mermaid += `  ${node.id}{" ${label} "}\n`;
      mermaid += `  class ${node.id} control\n`;
    } else if (node.node_type === "Block") {
      // Rect shape for block
      mermaid += `  ${node.id}[" ${label} "]\n`;
      mermaid += `  class ${node.id} block\n`;
    } else {
      // Rect shape for activity
      mermaid += `  ${node.id}[" ${label} "]\n`;
      mermaid += `  class ${node.id} activity\n`;
    }
  }

  // Render edges
  let linkIndex = 0;
  const linkStyles: string[] = [];
  
  for (const edge of result.graph.edges) {
    const cleanType = edge.type.replace(/:/g, "-");
    if (edge.type === "sequence") {
      mermaid += `  ${edge.from} --> ${edge.to}\n`;
    } else if (edge.type === "branch") {
      mermaid += `  ${edge.from} -->|branch| ${edge.to}\n`;
    } else if (edge.type === "scope") {
      mermaid += `  ${edge.from} -.->|scope| ${edge.to}\n`;
    } else if (edge.type.startsWith("cca:")) {
      mermaid += `  ${edge.from} -.->|${cleanType}| ${edge.to}\n`;
      linkStyles.push(`  linkStyle ${linkIndex} stroke:#d73a49,stroke-width:2px`);
    } else {
      mermaid += `  ${edge.from} -->|${cleanType}| ${edge.to}\n`;
    }
    linkIndex++;
  }

  // Click bindings for interactive jumping
  if (includeClickHandlers) {
    for (const node of result.graph.nodes) {
      mermaid += `  click ${node.id} call nodeClicked()\n`;
    }
  }

  // Link styles
  if (linkStyles.length > 0) {
    mermaid += "\n" + linkStyles.join("\n") + "\n";
  }

  return mermaid;
}

function renderSummaryTable(result: AnalyzerResult): string {
  if (result.diagnostics.length === 0) {
    return `<div style="margin-top: 16px; font-size: 12px; color: var(--vscode-descriptionForeground);">No concurrent dataflow anomalies detected.</div>`;
  }

  const rows = result.diagnostics.map((diagnostic) => `<tr>
    <td><span style="color: #d73a49; font-weight: 600;">${escapeHtml(diagnostic.cca_type)}</span></td>
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
