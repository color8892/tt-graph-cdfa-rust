import * as fs from "fs";
import * as path from "path";
import * as vscode from "vscode";
import { resolveAnalyzerPath, runAnalyzer } from "./analyzer";
import { renderGraphHtml } from "./graphView";
import { findGraphNode } from "./resultLookup";
import { AnalyzerResult, CdfaDiagnostic, SourceLocation } from "./schema";

let diagnosticCollection: vscode.DiagnosticCollection;
let lastResult: AnalyzerResult | undefined;
let currentPanel: vscode.WebviewPanel | undefined = undefined;

export function activate(context: vscode.ExtensionContext): void {
  diagnosticCollection = vscode.languages.createDiagnosticCollection("tt-graph-cdfa");
  context.subscriptions.push(diagnosticCollection);

  context.subscriptions.push(
    vscode.commands.registerCommand("ttGraphCdfa.analyzeCurrentFile", () => analyzeCurrentFile(context)),
    vscode.commands.registerCommand("ttGraphCdfa.analyzeWorkspaceExample", () => analyzeWorkspaceExample(context)),
    vscode.commands.registerCommand("ttGraphCdfa.clearDiagnostics", clearDiagnostics),
  );
}

export function deactivate(): void {
  diagnosticCollection?.dispose();
}

async function analyzeCurrentFile(context: vscode.ExtensionContext): Promise<void> {
  const editor = vscode.window.activeTextEditor;
  if (!editor) {
    vscode.window.showErrorMessage("Open a C++ file before running TT Graph CDFA analysis.");
    return;
  }
  await analyzeFile(editor.document.uri.fsPath, context);
}

async function analyzeWorkspaceExample(context: vscode.ExtensionContext): Promise<void> {
  const workspaceRoot = getWorkspaceRoot();
  if (!workspaceRoot) {
    vscode.window.showErrorMessage("Open the tt-graph-cdfa-rust workspace first.");
    return;
  }
  const examplePath = path.join(workspaceRoot, "examples", "paper_program1", "program1.cpp");
  await analyzeFile(examplePath, context);
}

async function analyzeFile(sourcePath: string, context: vscode.ExtensionContext): Promise<AnalyzerResult | undefined> {
  const workspaceRoot = getWorkspaceRoot();
  if (!workspaceRoot) {
    vscode.window.showErrorMessage("Open a workspace before running TT Graph CDFA analysis.");
    return undefined;
  }

  try {
    const configuredPath = vscode.workspace
      .getConfiguration("ttGraphCdfa")
      .get<string>("analyzerPath");
    const analyzerPath = resolveAnalyzerPath(configuredPath, workspaceRoot);
    const result = await runAnalyzer({ analyzerPath, workspaceRoot, sourcePath });
    lastResult = result;
    publishDiagnostics(result);

    try {
      showGraphWebview(result, context);
    } catch (webviewError) {
      vscode.window.showWarningMessage(`Failed to display graph view: ${String(webviewError)}`);
    }

    vscode.window.showInformationMessage(
      `TT Graph CDFA found ${result.diagnostics.length} diagnostics.`,
    );
    return result;
  } catch (error) {
    vscode.window.showErrorMessage(String(error));
    return undefined;
  }
}

function showGraphWebview(result: AnalyzerResult, context: vscode.ExtensionContext): void {
  const columnToShowIn = vscode.window.activeTextEditor
    ? vscode.window.activeTextEditor.viewColumn === vscode.ViewColumn.One
      ? vscode.ViewColumn.Two
      : vscode.ViewColumn.One
    : vscode.ViewColumn.One;

  if (currentPanel) {
    currentPanel.reveal(columnToShowIn);
    currentPanel.webview.html = renderGraphHtml(result, graphHtmlOptions(currentPanel.webview, context));
    return;
  }

  currentPanel = vscode.window.createWebviewPanel(
    "ttGraphCdfaView",
    "TT Graph CDFA Visualizer",
    columnToShowIn,
    {
      enableScripts: true,
      retainContextWhenHidden: true,
      localResourceRoots: [
        vscode.Uri.joinPath(context.extensionUri, "node_modules", "mermaid", "dist"),
        vscode.Uri.joinPath(context.extensionUri, "media"),
      ],
    }
  );

  currentPanel.webview.html = renderGraphHtml(result, graphHtmlOptions(currentPanel.webview, context));

  currentPanel.webview.onDidReceiveMessage(
    async (message) => {
      switch (message.type) {
        case "openNode":
          const nodeId = message.nodeId;
          const node = findGraphNode(lastResult, nodeId);
          if (node && node.source) {
            await openLocation(node.source);
          }
          break;
      }
    },
    undefined,
    context.subscriptions,
  );

  currentPanel.onDidDispose(
    () => {
      currentPanel = undefined;
    },
    null,
    context.subscriptions,
  );
}

function publishDiagnostics(result: AnalyzerResult): void {
  diagnosticCollection.clear();
  const byFile = new Map<string, vscode.Diagnostic[]>();

  for (const diagnostic of result.diagnostics) {
    const location = diagnostic.first;
    const uriPath = normalizeDiagnosticFile(result.source.path, location.file);
    const diagnostics = byFile.get(uriPath) ?? [];
    diagnostics.push(toVsCodeDiagnostic(diagnostic));
    byFile.set(uriPath, diagnostics);
  }

  for (const [filePath, diagnostics] of byFile) {
    diagnosticCollection.set(vscode.Uri.file(filePath), diagnostics);
  }
}

function toVsCodeDiagnostic(diagnostic: CdfaDiagnostic): vscode.Diagnostic {
  const location = diagnostic.first;
  const line = Math.max(0, location.line - 1);
  const column = Math.max(0, location.column - 1);
  const range = new vscode.Range(line, column, line, column + 1);
  const item = new vscode.Diagnostic(range, diagnostic.message, vscode.DiagnosticSeverity.Warning);
  item.source = "TT Graph CDFA";
  item.code = diagnostic.cca_type;
  return item;
}

async function openLocation(location: SourceLocation): Promise<void> {
  const filePath = normalizeDiagnosticFile(location.file, location.file);
  if (!fs.existsSync(filePath)) {
    vscode.window.showWarningMessage(`Source file not found: ${filePath}`);
    return;
  }
  const document = await vscode.workspace.openTextDocument(vscode.Uri.file(filePath));
  const editor = await vscode.window.showTextDocument(document);
  const position = new vscode.Position(Math.max(0, location.line - 1), Math.max(0, location.column - 1));
  editor.selection = new vscode.Selection(position, position);
  editor.revealRange(new vscode.Range(position, position), vscode.TextEditorRevealType.InCenter);
}

function normalizeDiagnosticFile(sourcePath: string, filePath: string): string {
  if (path.isAbsolute(filePath)) {
    return filePath;
  }
  const workspaceRoot = getWorkspaceRoot();
  if (workspaceRoot) {
    return path.join(workspaceRoot, filePath);
  }
  return path.resolve(path.dirname(sourcePath), filePath);
}

function clearDiagnostics(): void {
  diagnosticCollection.clear();
  lastResult = undefined;
  if (currentPanel) {
    currentPanel.dispose();
  }
}

function graphHtmlOptions(webview: vscode.Webview, context: vscode.ExtensionContext) {
  const mermaidScriptUri = webview.asWebviewUri(
    vscode.Uri.joinPath(
      context.extensionUri,
      "node_modules",
      "mermaid",
      "dist",
      "mermaid.esm.min.mjs",
    ),
  );
  const graphStylesUri = webview.asWebviewUri(
    vscode.Uri.joinPath(context.extensionUri, "media", "graphView.css"),
  );

  return {
    mermaidScriptUri: mermaidScriptUri.toString(),
    graphStylesUri: graphStylesUri.toString(),
    nonce: createNonce(),
    webviewCspSource: webview.cspSource,
  };
}

function createNonce(): string {
  const possible = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
  let text = "";
  for (let index = 0; index < 32; index++) {
    text += possible.charAt(Math.floor(Math.random() * possible.length));
  }
  return text;
}

// @ts-ignore (unused in this file but exported by design)
function getWorkspaceRoot(): string | undefined {
  return vscode.workspace.workspaceFolders?.[0]?.uri.fsPath;
}
