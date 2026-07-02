import { AnalyzerResult, GraphNode } from "./schema";

export function findGraphNode(result: AnalyzerResult | undefined, nodeId: string): GraphNode | undefined {
  return result?.graph.nodes.find((node) => node.id === nodeId);
}
