export interface AnalyzerResult {
  schema_version: number;
  source: AnalyzerSource;
  graph: AnalyzerGraph;
  diagnostics: CdfaDiagnostic[];
  error?: string;
}

export interface AnalyzerSource {
  path: string;
  language: string;
}

export interface AnalyzerGraph {
  nodes: GraphNode[];
  edges: GraphEdge[];
}

export interface GraphNode {
  id: string;
  node_type: "Activity" | "Control" | "Block";
  control_type: "And" | "Xor" | "Loop" | null;
  label: string;
  source: SourceLocation | null;
}

export interface GraphEdge {
  from: string;
  to: string;
  type: string;
}

export interface CdfaDiagnostic {
  severity: "warning";
  cca_type: string;
  variable: string;
  message: string;
  first: DiagnosticEndpoint;
  second: DiagnosticEndpoint;
}

export interface DiagnosticEndpoint extends SourceLocation {
  node: string;
}

export interface SourceLocation {
  file: string;
  line: number;
  column: number;
}

export function parseAnalyzerResult(raw: string): AnalyzerResult {
  let parsed: unknown;
  try {
    parsed = JSON.parse(raw);
  } catch (error) {
    throw new Error(`Analyzer did not return valid JSON: ${String(error)}`);
  }

  if (!isObject(parsed)) {
    throw new Error("Analyzer JSON must be an object");
  }
  if (parsed.schema_version !== 1) {
    throw new Error(`Unsupported analyzer schema_version: ${String(parsed.schema_version)}`);
  }
  if (!isObject(parsed.source) || typeof parsed.source.path !== "string") {
    throw new Error("Analyzer JSON is missing source.path");
  }
  if (!isObject(parsed.graph) || !Array.isArray(parsed.graph.nodes) || !Array.isArray(parsed.graph.edges)) {
    throw new Error("Analyzer JSON is missing graph nodes or edges");
  }
  if (!Array.isArray(parsed.diagnostics)) {
    throw new Error("Analyzer JSON is missing diagnostics array");
  }

  return parsed as unknown as AnalyzerResult;
}

function isObject(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null;
}
