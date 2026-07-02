use std::collections::{BTreeSet, HashMap, HashSet};

pub mod bench;
pub mod bench_corpus;
#[cfg(feature = "clang")]
pub mod clang_frontend;
pub mod cli_support;
pub mod diagnostics;
pub mod export;
pub mod figures;
pub mod format_helper;
pub mod interactive;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NodeType {
    Activity,
    Control,
    Block,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ControlType {
    And,
    Xor,
    Loop,
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum OperationType {
    Write,
    Read,
    Kill,
}

pub fn parse_operation_type(value: &str) -> Option<OperationType> {
    match value.to_ascii_lowercase().as_str() {
        "read" => Some(OperationType::Read),
        "write" => Some(OperationType::Write),
        "kill" => Some(OperationType::Kill),
        _ => None,
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum CcaType {
    WriteWrite,
    WriteRead,
    WriteKill,
    ReadKill,
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Operation {
    pub variable: String,
    pub op: OperationType,
}

impl Operation {
    pub fn new(variable: impl Into<String>, op: OperationType) -> Self {
        Self {
            variable: variable.into(),
            op,
        }
    }
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct CcaEntry {
    pub variable: String,
    pub first_node: String,
    pub second_node: String,
}

impl CcaEntry {
    pub fn new(
        variable: impl Into<String>,
        first_node: impl Into<String>,
        second_node: impl Into<String>,
    ) -> Self {
        Self {
            variable: variable.into(),
            first_node: first_node.into(),
            second_node: second_node.into(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TTNode {
    pub node_id: String,
    pub node_type: NodeType,
    pub control_type: Option<ControlType>,
    pub operations: HashSet<Operation>,
    pub sequence_arc: Option<String>,
    pub branch_arc: Vec<String>,
    pub scope_arc: Option<String>,
    pub d_opn_set: HashMap<(String, OperationType), HashSet<String>>,
    pub cca_sets: HashMap<CcaType, BTreeSet<CcaEntry>>,
}

impl TTNode {
    pub fn activity(node_id: impl Into<String>, scope_arc: impl Into<String>) -> Self {
        Self::new(node_id, NodeType::Activity, None, Some(scope_arc.into()))
    }

    pub fn control(
        node_id: impl Into<String>,
        control_type: ControlType,
        scope_arc: Option<String>,
    ) -> Self {
        Self::new(node_id, NodeType::Control, Some(control_type), scope_arc)
    }

    pub fn block(node_id: impl Into<String>, scope_arc: impl Into<String>) -> Self {
        Self::new(node_id, NodeType::Block, None, Some(scope_arc.into()))
    }

    fn new(
        node_id: impl Into<String>,
        node_type: NodeType,
        control_type: Option<ControlType>,
        scope_arc: Option<String>,
    ) -> Self {
        let mut cca_sets = HashMap::new();
        for cca_type in [
            CcaType::WriteWrite,
            CcaType::WriteRead,
            CcaType::WriteKill,
            CcaType::ReadKill,
        ] {
            cca_sets.insert(cca_type, BTreeSet::new());
        }

        Self {
            node_id: node_id.into(),
            node_type,
            control_type,
            operations: HashSet::new(),
            sequence_arc: None,
            branch_arc: Vec::new(),
            scope_arc,
            d_opn_set: HashMap::new(),
            cca_sets,
        }
    }

    pub fn with_operations(mut self, operations: Vec<Operation>) -> Self {
        self.operations = operations.into_iter().collect();
        self
    }

    pub fn with_sequence_arc(mut self, sequence_arc: impl Into<String>) -> Self {
        self.sequence_arc = Some(sequence_arc.into());
        self
    }

    pub fn with_branch_arc(mut self, branch_arc: Vec<String>) -> Self {
        self.branch_arc = branch_arc;
        self
    }

    fn is_nop_node(&self) -> bool {
        self.node_type == NodeType::Activity
            || (self.node_type == NodeType::Control
                && matches!(
                    self.control_type,
                    Some(ControlType::Xor) | Some(ControlType::Loop)
                ))
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StrategyResult {
    pub entries: BTreeSet<(CcaType, CcaEntry)>,
    pub touched_and_nodes: Vec<String>,
    pub summary_blocks_updated: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DetectionResult {
    pub summary_entries: BTreeSet<(CcaType, CcaEntry)>,
    pub direct_entries: BTreeSet<(CcaType, CcaEntry)>,
    pub touched_and_nodes: Vec<String>,
    pub summary_blocks_updated: Vec<String>,
}

impl DetectionResult {
    pub fn matches_direct_scan(&self) -> bool {
        self.summary_entries == self.direct_entries
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DeletionResult {
    pub removed_operation: bool,
    pub touched_and_nodes: Vec<String>,
    pub summary_blocks_updated: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DeletionVerificationResult {
    pub deletion: DeletionResult,
    pub recomputed_removed_operation: bool,
    pub summary_matches_recomputed: bool,
}

impl DeletionVerificationResult {
    pub fn matches_recomputed_state(&self) -> bool {
        self.summary_matches_recomputed
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TTGraphError {
    MissingNode { node_id: String },
    InvalidGraph { errors: Vec<String> },
}

impl std::fmt::Display for TTGraphError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TTGraphError::MissingNode { node_id } => {
                write!(formatter, "missing TT Graph node `{node_id}`")
            }
            TTGraphError::InvalidGraph { errors } => {
                write!(formatter, "invalid TT Graph: {}", errors.join("; "))
            }
        }
    }
}

impl std::error::Error for TTGraphError {}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TTGraph {
    pub nodes: HashMap<String, TTNode>,
    children_index: HashMap<String, Vec<String>>,
}

impl TTGraph {
    pub fn new(mut nodes: HashMap<String, TTNode>) -> Self {
        for node in nodes.values_mut() {
            node.d_opn_set.clear();
        }

        let mut children_index: HashMap<String, Vec<String>> = HashMap::new();
        for node in nodes.values() {
            if let Some(scope) = &node.scope_arc {
                children_index
                    .entry(scope.clone())
                    .or_default()
                    .push(node.node_id.clone());
            }
        }

        let mut graph = Self {
            nodes,
            children_index,
        };
        graph.rebuild_all_d_opn_sets();
        graph.recompute_all_cca_sets();
        graph
    }

    /// Validates the graph structure. Returns `Ok(())` if the graph is structurally valid,
    /// or a list of error messages if there are invalid node references.
    pub fn validate(&self) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();
        for (node_id, node) in &self.nodes {
            if let Some(scope_id) = &node.scope_arc
                && !self.nodes.contains_key(scope_id)
            {
                errors.push(format!(
                    "Node `{node_id}` has scope_arc pointing to non-existent node `{scope_id}`"
                ));
            }
            if let Some(seq_id) = &node.sequence_arc
                && !self.nodes.contains_key(seq_id)
            {
                errors.push(format!(
                    "Node `{node_id}` has sequence_arc pointing to non-existent node `{seq_id}`"
                ));
            }
            for branch_id in &node.branch_arc {
                if !self.nodes.contains_key(branch_id) {
                    errors.push(format!(
                        "Node `{node_id}` has branch_arc pointing to non-existent node `{branch_id}`"
                    ));
                }
            }
        }
        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    /// Inserts a data-flow operation into a node and runs both summary and direct scan
    /// detection strategies, verifying that the results are identical.
    ///
    /// # Preconditions
    /// The target node must exist in the graph.
    ///
    /// # Panics
    /// Panics if `tnode_id` is not present in the graph.
    pub fn insert_operation(
        &mut self,
        tnode_id: &str,
        variable: &str,
        op: OperationType,
    ) -> DetectionResult {
        let mut baseline = self.clone();
        let summary = self.insert_operation_summary_only(tnode_id, variable, op);
        let direct = baseline.insert_operation_direct_only(tnode_id, variable, op);

        DetectionResult {
            summary_entries: summary.entries,
            direct_entries: direct.entries,
            touched_and_nodes: summary.touched_and_nodes,
            summary_blocks_updated: summary.summary_blocks_updated,
        }
    }

    /// Inserts a data-flow operation without panicking on invalid input.
    pub fn try_insert_operation(
        &mut self,
        tnode_id: &str,
        variable: &str,
        op: OperationType,
    ) -> Result<DetectionResult, TTGraphError> {
        self.validate_for_operation(tnode_id)?;
        Ok(self.insert_operation(tnode_id, variable, op))
    }

    /// Deletes a data-flow operation from a node and propagates the deletion
    /// up the scope hierarchy, updating summaries and recomputing CCA sets.
    ///
    /// # Preconditions
    /// The target node must exist in the graph.
    ///
    /// # Panics
    /// Panics if `tnode_id` is not present in the graph.
    pub fn delete_operation(
        &mut self,
        tnode_id: &str,
        variable: &str,
        op: OperationType,
    ) -> DeletionResult {
        let removed_operation = self
            .nodes
            .get_mut(tnode_id)
            .expect("TNode must exist")
            .operations
            .remove(&Operation::new(variable, op));

        if !removed_operation {
            return DeletionResult {
                removed_operation,
                touched_and_nodes: Vec::new(),
                summary_blocks_updated: Vec::new(),
            };
        }

        let mut touched_and_nodes = Vec::new();
        let mut summary_blocks_updated = Vec::new();
        let mut current_id = tnode_id.to_string();

        loop {
            let scope_id = self.nodes[&current_id].scope_arc.clone();
            let Some(scope_id) = scope_id else {
                break;
            };

            current_id = scope_id;
            if self.nodes[&current_id].node_type == NodeType::Block {
                self.remove_d_opn(&current_id, variable, op, tnode_id);
                summary_blocks_updated.push(current_id.clone());

                if let Some(parent_id) = self.nodes[&current_id].scope_arc.clone()
                    && self.is_and_control(&parent_id)
                {
                    self.recompute_and_cca_sets(&parent_id);
                    touched_and_nodes.push(parent_id);
                }
            }
        }

        DeletionResult {
            removed_operation,
            touched_and_nodes,
            summary_blocks_updated,
        }
    }

    /// Deletes a data-flow operation and checks the resulting summaries against
    /// a full recomputation baseline.
    ///
    /// This is intended for verification and reproduction paths. It keeps
    /// `delete_operation` lightweight while exposing the stronger invariant
    /// needed to defend deletion as an extension of the insertion algorithm.
    ///
    /// # Preconditions
    /// The target node must exist in the graph.
    ///
    /// # Panics
    /// Panics if `tnode_id` is not present in the graph.
    pub fn delete_operation_with_recompute_check(
        &mut self,
        tnode_id: &str,
        variable: &str,
        op: OperationType,
    ) -> DeletionVerificationResult {
        let mut recomputed = self.clone();
        let deletion = self.delete_operation(tnode_id, variable, op);
        let recomputed_removed_operation = recomputed
            .nodes
            .get_mut(tnode_id)
            .expect("TNode must exist")
            .operations
            .remove(&Operation::new(variable, op));

        if recomputed_removed_operation {
            recomputed.rebuild_all_d_opn_sets();
            recomputed.recompute_all_cca_sets();
        }

        DeletionVerificationResult {
            summary_matches_recomputed: deletion.removed_operation == recomputed_removed_operation
                && self.nodes == recomputed.nodes
                && self.children_index == recomputed.children_index,
            deletion,
            recomputed_removed_operation,
        }
    }

    /// Deletes a data-flow operation without panicking on invalid input.
    pub fn try_delete_operation(
        &mut self,
        tnode_id: &str,
        variable: &str,
        op: OperationType,
    ) -> Result<DeletionResult, TTGraphError> {
        self.validate_for_operation(tnode_id)?;
        Ok(self.delete_operation(tnode_id, variable, op))
    }

    /// Deletes a data-flow operation without panicking on invalid input, then
    /// checks the mutated graph against a full recomputation baseline.
    pub fn try_delete_operation_with_recompute_check(
        &mut self,
        tnode_id: &str,
        variable: &str,
        op: OperationType,
    ) -> Result<DeletionVerificationResult, TTGraphError> {
        self.validate_for_operation(tnode_id)?;
        Ok(self.delete_operation_with_recompute_check(tnode_id, variable, op))
    }

    /// Inserts a data-flow operation into a node using only the summary-based
    /// strategy (Algorithm 1) which incrementally updates `d_OPN_set` summaries.
    ///
    /// # Preconditions
    /// The target node must exist in the graph.
    ///
    /// # Panics
    /// Panics if `tnode_id` is not present in the graph.
    pub fn insert_operation_summary_only(
        &mut self,
        tnode_id: &str,
        variable: &str,
        op: OperationType,
    ) -> StrategyResult {
        self.nodes
            .get_mut(tnode_id)
            .expect("TNode must exist")
            .operations
            .insert(Operation::new(variable, op));

        let mut entries = BTreeSet::new();
        let mut touched_and_nodes = Vec::new();
        let mut summary_blocks_updated = Vec::new();
        let mut current_id = tnode_id.to_string();

        loop {
            let scope_id = self.nodes[&current_id].scope_arc.clone();
            let Some(scope_id) = scope_id else {
                break;
            };

            current_id = scope_id;
            if self.nodes[&current_id].node_type == NodeType::Block {
                self.add_d_opn(&current_id, variable, op, tnode_id);
                summary_blocks_updated.push(current_id.clone());

                if let Some(parent_id) = self.nodes[&current_id].scope_arc.clone()
                    && self.is_and_control(&parent_id)
                {
                    touched_and_nodes.push(parent_id.clone());
                    for entry in
                        self.detect_using_d_opn_set(&parent_id, &current_id, variable, op, tnode_id)
                    {
                        entries.insert(entry);
                    }
                }
            }
        }

        StrategyResult {
            entries,
            touched_and_nodes,
            summary_blocks_updated,
        }
    }

    /// Inserts a data-flow operation into a node using only the direct scan
    /// strategy (Algorithm 2) which searches sibling subgraphs directly.
    ///
    /// # Preconditions
    /// The target node must exist in the graph.
    ///
    /// # Panics
    /// Panics if `tnode_id` is not present in the graph.
    pub fn insert_operation_direct_only(
        &mut self,
        tnode_id: &str,
        variable: &str,
        op: OperationType,
    ) -> StrategyResult {
        self.nodes
            .get_mut(tnode_id)
            .expect("TNode must exist")
            .operations
            .insert(Operation::new(variable, op));

        let mut entries = BTreeSet::new();
        let mut touched_and_nodes = Vec::new();
        let mut current_id = tnode_id.to_string();

        loop {
            let scope_id = self.nodes[&current_id].scope_arc.clone();
            let Some(scope_id) = scope_id else {
                break;
            };

            current_id = scope_id;
            if self.nodes[&current_id].node_type == NodeType::Block
                && let Some(parent_id) = self.nodes[&current_id].scope_arc.clone()
                && self.is_and_control(&parent_id)
            {
                touched_and_nodes.push(parent_id.clone());
                for entry in
                    self.detect_by_direct_scan(&parent_id, &current_id, variable, op, tnode_id)
                {
                    entries.insert(entry);
                }
            }
        }

        StrategyResult {
            entries,
            touched_and_nodes,
            summary_blocks_updated: Vec::new(),
        }
    }

    pub fn rebuild_all_d_opn_sets(&mut self) {
        for node in self.nodes.values_mut() {
            node.d_opn_set.clear();
        }

        let block_ids: Vec<String> = self
            .nodes
            .values()
            .filter(|node| node.node_type == NodeType::Block)
            .map(|node| node.node_id.clone())
            .collect();

        for block_id in block_ids {
            for node_id in self.reachable_nop_nodes(&block_id) {
                let operations: Vec<Operation> =
                    self.nodes[&node_id].operations.iter().cloned().collect();
                for operation in operations {
                    self.add_d_opn(&block_id, &operation.variable, operation.op, &node_id);
                }
            }
        }
    }

    pub fn recompute_all_cca_sets(&mut self) {
        let and_ids: Vec<String> = self
            .nodes
            .values()
            .filter(|node| {
                node.node_type == NodeType::Control && node.control_type == Some(ControlType::And)
            })
            .map(|node| node.node_id.clone())
            .collect();

        for and_id in and_ids {
            self.recompute_and_cca_sets(&and_id);
        }
    }

    pub fn reachable_nop_nodes(&self, block_id: &str) -> BTreeSet<String> {
        let mut result = BTreeSet::new();
        if let Some(children) = self.children_index.get(block_id) {
            for child_id in children {
                let node = &self.nodes[child_id];
                if node.node_type != NodeType::Block {
                    if node.is_nop_node() {
                        result.insert(child_id.clone());
                    }
                    if node.node_type == NodeType::Control {
                        for child_block_id in &node.branch_arc {
                            result.extend(self.reachable_nop_nodes(child_block_id));
                        }
                    }
                }
            }
        }
        result
    }

    pub fn sorted_d_opn_rows(&self, block_id: &str) -> Vec<(String, OperationType, Vec<String>)> {
        let mut rows: Vec<(String, OperationType, Vec<String>)> = self.nodes[block_id]
            .d_opn_set
            .iter()
            .map(|((variable, op), node_ids)| {
                let mut nodes: Vec<String> = node_ids.iter().cloned().collect();
                nodes.sort();
                (variable.clone(), *op, nodes)
            })
            .collect();
        rows.sort_by(|left, right| (left.0.as_str(), left.1).cmp(&(right.0.as_str(), right.1)));
        rows
    }

    pub fn to_dot(&self) -> String {
        let mut output = String::from("digraph TTGraph {\n");
        output.push_str("  rankdir=TB;\n");
        output.push_str("  node [fontname=\"Consolas\"];\n");

        let mut node_ids: Vec<&String> = self.nodes.keys().collect();
        node_ids.sort();
        for node_id in &node_ids {
            let node = &self.nodes[*node_id];
            let shape = match node.node_type {
                NodeType::Activity => "box",
                NodeType::Control => "diamond",
                NodeType::Block => "folder",
            };
            let label = dot_escape(&format!(
                "{}\\n{}{}",
                node.node_id,
                node_type_label(node),
                operation_label(node)
            ));
            output.push_str(&format!(
                "  \"{}\" [shape={}, label=\"{}\"];\n",
                dot_escape(&node.node_id),
                shape,
                label
            ));
        }

        for node_id in &node_ids {
            let node = &self.nodes[*node_id];
            if let Some(sequence_arc) = &node.sequence_arc {
                output.push_str(&format!(
                    "  \"{}\" -> \"{}\" [label=\"sequence\", color=\"black\"];\n",
                    dot_escape(&node.node_id),
                    dot_escape(sequence_arc)
                ));
            }
            for branch_arc in &node.branch_arc {
                output.push_str(&format!(
                    "  \"{}\" -> \"{}\" [label=\"branch\", color=\"blue\"];\n",
                    dot_escape(&node.node_id),
                    dot_escape(branch_arc)
                ));
            }
            if let Some(scope_arc) = &node.scope_arc {
                output.push_str(&format!(
                    "  \"{}\" -> \"{}\" [label=\"scope\", style=\"dashed\", color=\"gray\"];\n",
                    dot_escape(&node.node_id),
                    dot_escape(scope_arc)
                ));
            }
        }

        for node_id in &node_ids {
            let node = &self.nodes[*node_id];
            let mut cca_types: Vec<CcaType> = node.cca_sets.keys().copied().collect();
            cca_types.sort();
            for cca_type in cca_types {
                if let Some(entries) = node.cca_sets.get(&cca_type) {
                    for entry in entries {
                        output.push_str(&format!(
                            "  \"{}\" -> \"{}\" [label=\"{:?}({})\", color=\"red\", style=\"dotted\"];\n",
                            dot_escape(&entry.first_node),
                            dot_escape(&entry.second_node),
                            cca_type,
                            dot_escape(&entry.variable)
                        ));
                    }
                }
            }
        }

        output.push_str("}\n");
        output
    }

    fn detect_using_d_opn_set(
        &mut self,
        and_id: &str,
        current_block_id: &str,
        variable: &str,
        op: OperationType,
        tnode_id: &str,
    ) -> Vec<(CcaType, CcaEntry)> {
        let mut entries = Vec::new();
        for other_block_id in self.other_blocks(and_id, current_block_id) {
            for (other_op, cca_type) in related_operations(op) {
                let node_ids = self.nodes[&other_block_id]
                    .d_opn_set
                    .get(&(variable.to_string(), other_op))
                    .cloned()
                    .unwrap_or_default();

                for other_node_id in node_ids {
                    entries.push(self.record_cca(
                        and_id,
                        cca_type,
                        variable,
                        op,
                        tnode_id,
                        &other_node_id,
                    ));
                }
            }
        }
        entries
    }

    fn detect_by_direct_scan(
        &mut self,
        and_id: &str,
        current_block_id: &str,
        variable: &str,
        op: OperationType,
        tnode_id: &str,
    ) -> Vec<(CcaType, CcaEntry)> {
        let mut entries = Vec::new();
        for other_block_id in self.other_blocks(and_id, current_block_id) {
            for other_node_id in self.reachable_nop_nodes(&other_block_id) {
                for (other_op, cca_type) in related_operations(op) {
                    if self.nodes[&other_node_id]
                        .operations
                        .contains(&Operation::new(variable, other_op))
                    {
                        entries.push(self.record_cca(
                            and_id,
                            cca_type,
                            variable,
                            op,
                            tnode_id,
                            &other_node_id,
                        ));
                    }
                }
            }
        }
        entries
    }

    fn record_cca(
        &mut self,
        and_id: &str,
        cca_type: CcaType,
        variable: &str,
        new_op: OperationType,
        new_node_id: &str,
        other_node_id: &str,
    ) -> (CcaType, CcaEntry) {
        let entry = normalize_cca_entry(cca_type, variable, new_op, new_node_id, other_node_id);
        self.nodes
            .get_mut(and_id)
            .expect("AND node must exist")
            .cca_sets
            .entry(cca_type)
            .or_default()
            .insert(entry.clone());
        (cca_type, entry)
    }

    fn recompute_and_cca_sets(&mut self, and_id: &str) {
        for entries in self
            .nodes
            .get_mut(and_id)
            .expect("AND node must exist")
            .cca_sets
            .values_mut()
        {
            entries.clear();
        }

        let branch_blocks = self.nodes[and_id].branch_arc.clone();
        let mut detected = Vec::new();
        for left_index in 0..branch_blocks.len() {
            for right_index in (left_index + 1)..branch_blocks.len() {
                let left_nodes: Vec<String> = self
                    .reachable_nop_nodes(&branch_blocks[left_index])
                    .into_iter()
                    .collect();
                let right_nodes: Vec<String> = self
                    .reachable_nop_nodes(&branch_blocks[right_index])
                    .into_iter()
                    .collect();

                for left_node_id in &left_nodes {
                    for right_node_id in &right_nodes {
                        let left_operations: Vec<Operation> = self.nodes[left_node_id]
                            .operations
                            .iter()
                            .cloned()
                            .collect();
                        let right_operations: Vec<Operation> = self.nodes[right_node_id]
                            .operations
                            .iter()
                            .cloned()
                            .collect();

                        for left_operation in &left_operations {
                            for right_operation in &right_operations {
                                if left_operation.variable != right_operation.variable {
                                    continue;
                                }
                                if let Some(cca_type) =
                                    cca_type_for_pair(left_operation.op, right_operation.op)
                                {
                                    let entry = normalize_cca_entry(
                                        cca_type,
                                        &left_operation.variable,
                                        left_operation.op,
                                        left_node_id,
                                        right_node_id,
                                    );
                                    detected.push((cca_type, entry));
                                }
                            }
                        }
                    }
                }
            }
        }

        let and_node = self.nodes.get_mut(and_id).expect("AND node must exist");
        for (cca_type, entry) in detected {
            and_node.cca_sets.entry(cca_type).or_default().insert(entry);
        }
    }

    fn add_d_opn(&mut self, block_id: &str, variable: &str, op: OperationType, node_id: &str) {
        self.nodes
            .get_mut(block_id)
            .expect("BLOCK node must exist")
            .d_opn_set
            .entry((variable.to_string(), op))
            .or_default()
            .insert(node_id.to_string());
    }

    fn remove_d_opn(&mut self, block_id: &str, variable: &str, op: OperationType, node_id: &str) {
        let block = self.nodes.get_mut(block_id).expect("BLOCK node must exist");
        let key = (variable.to_string(), op);
        let should_remove_key = if let Some(node_ids) = block.d_opn_set.get_mut(&key) {
            node_ids.remove(node_id);
            node_ids.is_empty()
        } else {
            false
        };
        if should_remove_key {
            block.d_opn_set.remove(&key);
        }
    }

    fn is_and_control(&self, node_id: &str) -> bool {
        let node = &self.nodes[node_id];
        node.node_type == NodeType::Control && node.control_type == Some(ControlType::And)
    }

    fn other_blocks(&self, and_id: &str, current_block_id: &str) -> Vec<String> {
        self.nodes[and_id]
            .branch_arc
            .iter()
            .filter(|block_id| block_id.as_str() != current_block_id)
            .cloned()
            .collect()
    }

    fn validate_for_operation(&self, tnode_id: &str) -> Result<(), TTGraphError> {
        if !self.nodes.contains_key(tnode_id) {
            return Err(TTGraphError::MissingNode {
                node_id: tnode_id.to_string(),
            });
        }
        self.validate()
            .map_err(|errors| TTGraphError::InvalidGraph { errors })
    }
}

pub fn related_operations(op: OperationType) -> Vec<(OperationType, CcaType)> {
    match op {
        OperationType::Write => vec![
            (OperationType::Write, CcaType::WriteWrite),
            (OperationType::Read, CcaType::WriteRead),
            (OperationType::Kill, CcaType::WriteKill),
        ],
        OperationType::Read => vec![
            (OperationType::Write, CcaType::WriteRead),
            (OperationType::Kill, CcaType::ReadKill),
        ],
        OperationType::Kill => vec![
            (OperationType::Write, CcaType::WriteKill),
            (OperationType::Read, CcaType::ReadKill),
        ],
    }
}

pub fn cca_type_for_pair(left: OperationType, right: OperationType) -> Option<CcaType> {
    match (left, right) {
        (OperationType::Write, OperationType::Write) => Some(CcaType::WriteWrite),
        (OperationType::Write, OperationType::Read)
        | (OperationType::Read, OperationType::Write) => Some(CcaType::WriteRead),
        (OperationType::Write, OperationType::Kill)
        | (OperationType::Kill, OperationType::Write) => Some(CcaType::WriteKill),
        (OperationType::Read, OperationType::Kill) | (OperationType::Kill, OperationType::Read) => {
            Some(CcaType::ReadKill)
        }
        _ => None,
    }
}

pub fn normalize_cca_entry(
    cca_type: CcaType,
    variable: &str,
    new_op: OperationType,
    new_node_id: &str,
    other_node_id: &str,
) -> CcaEntry {
    match cca_type {
        CcaType::WriteRead => CcaEntry::new(
            variable,
            if new_op == OperationType::Write {
                new_node_id
            } else {
                other_node_id
            },
            if new_op == OperationType::Read {
                new_node_id
            } else {
                other_node_id
            },
        ),
        CcaType::WriteKill => CcaEntry::new(
            variable,
            if new_op == OperationType::Write {
                new_node_id
            } else {
                other_node_id
            },
            if new_op == OperationType::Kill {
                new_node_id
            } else {
                other_node_id
            },
        ),
        CcaType::ReadKill => CcaEntry::new(
            variable,
            if new_op == OperationType::Read {
                new_node_id
            } else {
                other_node_id
            },
            if new_op == OperationType::Kill {
                new_node_id
            } else {
                other_node_id
            },
        ),
        CcaType::WriteWrite => {
            if new_node_id <= other_node_id {
                CcaEntry::new(variable, new_node_id, other_node_id)
            } else {
                CcaEntry::new(variable, other_node_id, new_node_id)
            }
        }
    }
}

fn node_type_label(node: &TTNode) -> String {
    match node.node_type {
        NodeType::Activity => "ACTIVITY".to_string(),
        NodeType::Block => "BLOCK".to_string(),
        NodeType::Control => match node.control_type {
            Some(control_type) => format!("CONTROL::{control_type:?}"),
            None => "CONTROL".to_string(),
        },
    }
}

fn operation_label(node: &TTNode) -> String {
    if node.operations.is_empty() {
        return String::new();
    }

    let mut operations: Vec<String> = node
        .operations
        .iter()
        .map(|operation| format!("{}:{:?}", operation.variable, operation.op))
        .collect();
    operations.sort();
    format!("\\n{}", operations.join(", "))
}

fn dot_escape(value: &str) -> String {
    value.replace('"', "\\\"")
}

#[derive(Clone, Debug)]
pub struct SyntheticGraphCase {
    pub graph: TTGraph,
    pub target_node_id: String,
    pub node_count: usize,
    pub leaf_count: usize,
    pub matching_leaf_count: usize,
}

pub fn build_synthetic_full_and_graph(depth: usize, matching_stride: usize) -> SyntheticGraphCase {
    assert!(depth >= 1, "depth must be >= 1");
    assert!(matching_stride >= 1, "matching_stride must be >= 1");

    struct Builder {
        nodes: HashMap<String, TTNode>,
        leaf_index: usize,
        matching_leaf_count: usize,
        target_node_id: String,
        depth: usize,
        matching_stride: usize,
    }

    impl Builder {
        fn build_and(
            &mut self,
            level: usize,
            scope_block_id: Option<String>,
            prefix: &str,
        ) -> String {
            let and_id = format!("And_{prefix}");
            self.nodes.insert(
                and_id.clone(),
                TTNode::control(and_id.clone(), ControlType::And, scope_block_id),
            );

            let mut branch_arc = Vec::new();
            for side in ["L", "R"] {
                let block_id = format!("B_{prefix}_{side}");
                self.nodes.insert(
                    block_id.clone(),
                    TTNode::block(block_id.clone(), and_id.clone()),
                );
                branch_arc.push(block_id.clone());

                if level == self.depth {
                    self.leaf_index += 1;
                    let act_id = format!("Act_{:05}", self.leaf_index);
                    if self.target_node_id.is_empty() {
                        self.target_node_id = act_id.clone();
                    }

                    let mut operations = vec![Operation::new(
                        format!("noise_{}", self.leaf_index % 11),
                        OperationType::Read,
                    )];
                    if self.leaf_index.is_multiple_of(self.matching_stride) {
                        operations.push(Operation::new("target", OperationType::Read));
                        self.matching_leaf_count += 1;
                    }

                    self.nodes.insert(
                        act_id.clone(),
                        TTNode::activity(act_id.clone(), block_id.clone())
                            .with_operations(operations),
                    );
                    self.nodes
                        .get_mut(&block_id)
                        .expect("block must exist")
                        .sequence_arc = Some(act_id);
                } else {
                    let child_and_id = self.build_and(
                        level + 1,
                        Some(block_id.clone()),
                        &format!("{prefix}{side}"),
                    );
                    self.nodes
                        .get_mut(&block_id)
                        .expect("block must exist")
                        .sequence_arc = Some(child_and_id);
                }
            }

            self.nodes
                .get_mut(&and_id)
                .expect("AND node must exist")
                .branch_arc = branch_arc;
            and_id
        }
    }

    let mut builder = Builder {
        nodes: HashMap::new(),
        leaf_index: 0,
        matching_leaf_count: 0,
        target_node_id: String::new(),
        depth,
        matching_stride,
    };
    builder.build_and(1, None, "Root");

    let node_count = builder.nodes.len();
    SyntheticGraphCase {
        graph: TTGraph::new(builder.nodes),
        target_node_id: builder.target_node_id,
        node_count,
        leaf_count: builder.leaf_index,
        matching_leaf_count: builder.matching_leaf_count,
    }
}

pub fn build_chain_and_graph(depth: usize, matching_stride: usize) -> SyntheticGraphCase {
    assert!(depth >= 1, "depth must be >= 1");
    assert!(matching_stride >= 1, "matching_stride must be >= 1");

    struct Builder {
        nodes: HashMap<String, TTNode>,
        leaf_index: usize,
        matching_leaf_count: usize,
        target_node_id: String,
        depth: usize,
        matching_stride: usize,
    }

    impl Builder {
        fn build_chain(
            &mut self,
            level: usize,
            scope_block_id: Option<String>,
            prefix: &str,
        ) -> String {
            let and_id = format!("AndC_{prefix}");
            self.nodes.insert(
                and_id.clone(),
                TTNode::control(and_id.clone(), ControlType::And, scope_block_id),
            );

            let left_block = format!("BC_{prefix}_L");
            let right_block = format!("BC_{prefix}_R");
            self.nodes.insert(
                left_block.clone(),
                TTNode::block(left_block.clone(), and_id.clone()),
            );
            self.nodes.insert(
                right_block.clone(),
                TTNode::block(right_block.clone(), and_id.clone()),
            );

            if level < self.depth {
                let child_and =
                    self.build_chain(level + 1, Some(left_block.clone()), &format!("{prefix}L"));
                self.nodes
                    .get_mut(&left_block)
                    .expect("left block exists")
                    .sequence_arc = Some(child_and);
            } else {
                self.attach_leaf(&left_block, true);
            }

            self.attach_leaf(&right_block, false);
            self.nodes.get_mut(&and_id).expect("AND exists").branch_arc =
                vec![left_block, right_block];
            and_id
        }

        fn attach_leaf(&mut self, block_id: &str, is_target_branch: bool) {
            self.leaf_index += 1;
            let act_id = format!("ActC_{:05}", self.leaf_index);
            if is_target_branch && self.target_node_id.is_empty() {
                self.target_node_id = act_id.clone();
            }

            let mut operations = vec![Operation::new(
                format!("noise_{}", self.leaf_index),
                OperationType::Read,
            )];
            if self.leaf_index.is_multiple_of(self.matching_stride) {
                operations.push(Operation::new("target", OperationType::Read));
                self.matching_leaf_count += 1;
            }

            self.nodes.insert(
                act_id.clone(),
                TTNode::activity(act_id.clone(), block_id.to_string()).with_operations(operations),
            );
            self.nodes
                .get_mut(block_id)
                .expect("block exists")
                .sequence_arc = Some(act_id);
        }
    }

    let mut builder = Builder {
        nodes: HashMap::new(),
        leaf_index: 0,
        matching_leaf_count: 0,
        target_node_id: String::new(),
        depth,
        matching_stride,
    };
    builder.build_chain(1, None, "Root");

    let node_count = builder.nodes.len();
    SyntheticGraphCase {
        graph: TTGraph::new(builder.nodes),
        target_node_id: builder.target_node_id,
        node_count,
        leaf_count: builder.leaf_index,
        matching_leaf_count: builder.matching_leaf_count,
    }
}

pub fn build_paper_example_graph() -> TTGraph {
    let mut nodes = HashMap::new();

    nodes.insert(
        "And1".to_string(),
        TTNode::control("And1", ControlType::And, None)
            .with_branch_arc(vec!["B1".to_string(), "B2".to_string()]),
    );
    nodes.insert(
        "B1".to_string(),
        TTNode::block("B1", "And1").with_sequence_arc("Act1"),
    );
    nodes.insert(
        "B2".to_string(),
        TTNode::block("B2", "And1").with_sequence_arc("Act3"),
    );
    nodes.insert(
        "Act1".to_string(),
        TTNode::activity("Act1", "B1")
            .with_operations(vec![
                Operation::new("v", OperationType::Read),
                Operation::new("v", OperationType::Write),
                Operation::new("i", OperationType::Write),
            ])
            .with_sequence_arc("Loop1"),
    );
    nodes.insert(
        "Loop1".to_string(),
        TTNode::control("Loop1", ControlType::Loop, Some("B1".to_string()))
            .with_operations(vec![Operation::new("i", OperationType::Read)])
            .with_branch_arc(vec!["B3".to_string()]),
    );
    nodes.insert(
        "B3".to_string(),
        TTNode::block("B3", "Loop1").with_sequence_arc("Act2"),
    );
    nodes.insert(
        "Act2".to_string(),
        TTNode::activity("Act2", "B3").with_operations(vec![
            Operation::new("v", OperationType::Read),
            Operation::new("i", OperationType::Read),
            Operation::new("i", OperationType::Write),
        ]),
    );
    nodes.insert(
        "Act3".to_string(),
        TTNode::activity("Act3", "B2")
            .with_operations(vec![
                Operation::new("v", OperationType::Read),
                Operation::new("v", OperationType::Write),
            ])
            .with_sequence_arc("Xor1"),
    );
    nodes.insert(
        "Xor1".to_string(),
        TTNode::control("Xor1", ControlType::Xor, Some("B2".to_string()))
            .with_operations(vec![Operation::new("v", OperationType::Read)])
            .with_branch_arc(vec!["B4".to_string(), "B5".to_string()]),
    );
    nodes.insert(
        "B4".to_string(),
        TTNode::block("B4", "Xor1").with_sequence_arc("Act4"),
    );
    nodes.insert(
        "B5".to_string(),
        TTNode::block("B5", "Xor1").with_sequence_arc("Act5"),
    );
    nodes.insert(
        "Act4".to_string(),
        TTNode::activity("Act4", "B4").with_operations(vec![
            Operation::new("v", OperationType::Read),
            Operation::new("i", OperationType::Write),
            Operation::new("v", OperationType::Kill),
        ]),
    );
    nodes.insert(
        "Act5".to_string(),
        TTNode::activity("Act5", "B5").with_operations(vec![
            Operation::new("v", OperationType::Read),
            Operation::new("v", OperationType::Kill),
        ]),
    );

    TTGraph::new(nodes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paper_table_2_d_opn_sets_match_program_1() {
        let graph = build_paper_example_graph();

        assert_d_opn(&graph, "B1", "v", OperationType::Read, &["Act1", "Act2"]);
        assert_d_opn(&graph, "B1", "v", OperationType::Write, &["Act1"]);
        assert_d_opn(&graph, "B1", "i", OperationType::Read, &["Act2", "Loop1"]);
        assert_d_opn(&graph, "B1", "i", OperationType::Write, &["Act1", "Act2"]);

        assert_d_opn(
            &graph,
            "B2",
            "v",
            OperationType::Read,
            &["Act3", "Act4", "Act5", "Xor1"],
        );
        assert_d_opn(&graph, "B2", "v", OperationType::Write, &["Act3"]);
        assert_d_opn(&graph, "B2", "v", OperationType::Kill, &["Act4", "Act5"]);
        assert_d_opn(&graph, "B2", "i", OperationType::Write, &["Act4"]);

        assert_d_opn(&graph, "B3", "v", OperationType::Read, &["Act2"]);
        assert_d_opn(&graph, "B3", "i", OperationType::Read, &["Act2"]);
        assert_d_opn(&graph, "B3", "i", OperationType::Write, &["Act2"]);

        assert_d_opn(&graph, "B4", "v", OperationType::Read, &["Act4"]);
        assert_d_opn(&graph, "B4", "i", OperationType::Write, &["Act4"]);
        assert_d_opn(&graph, "B4", "v", OperationType::Kill, &["Act4"]);

        assert_d_opn(&graph, "B5", "v", OperationType::Read, &["Act5"]);
        assert_d_opn(&graph, "B5", "v", OperationType::Kill, &["Act5"]);
    }

    #[test]
    fn paper_write_insertion_matches_direct_scan() {
        let mut graph = build_paper_example_graph();

        let result = graph.insert_operation("Act2", "v", OperationType::Write);

        let expected = BTreeSet::from([
            (CcaType::WriteWrite, CcaEntry::new("v", "Act2", "Act3")),
            (CcaType::WriteRead, CcaEntry::new("v", "Act2", "Act3")),
            (CcaType::WriteRead, CcaEntry::new("v", "Act2", "Act4")),
            (CcaType::WriteRead, CcaEntry::new("v", "Act2", "Act5")),
            (CcaType::WriteRead, CcaEntry::new("v", "Act2", "Xor1")),
            (CcaType::WriteKill, CcaEntry::new("v", "Act2", "Act4")),
            (CcaType::WriteKill, CcaEntry::new("v", "Act2", "Act5")),
        ]);

        assert!(result.matches_direct_scan());
        assert_eq!(result.summary_entries, expected);
        assert_eq!(result.touched_and_nodes, vec!["And1".to_string()]);
        assert_eq!(
            graph.nodes["B1"].d_opn_set[&("v".to_string(), OperationType::Write)],
            HashSet::from(["Act1".to_string(), "Act2".to_string()])
        );
    }

    #[test]
    fn read_insertion_keeps_write_read_tuple_order() {
        let mut graph = build_paper_example_graph();

        let result = graph.insert_operation("Act4", "i", OperationType::Read);

        assert!(result.matches_direct_scan());
        assert_eq!(
            result.summary_entries,
            BTreeSet::from([
                (CcaType::WriteRead, CcaEntry::new("i", "Act1", "Act4")),
                (CcaType::WriteRead, CcaEntry::new("i", "Act2", "Act4")),
            ])
        );
    }

    #[test]
    fn synthetic_graph_summary_matches_direct_scan() {
        let case = build_synthetic_full_and_graph(4, 3);
        let mut summary_graph = case.graph.clone();
        let mut direct_graph = case.graph.clone();

        let summary = summary_graph.insert_operation_summary_only(
            &case.target_node_id,
            "target",
            OperationType::Write,
        );
        let direct = direct_graph.insert_operation_direct_only(
            &case.target_node_id,
            "target",
            OperationType::Write,
        );

        assert_eq!(summary.entries, direct.entries);
        assert!(!summary.entries.is_empty());
    }

    #[test]
    fn nested_loop_xor_and_graph_summary_matches_direct_scan() {
        let mut graph = build_nested_control_flow_graph();

        let result = graph.insert_operation("ActThen", "x", OperationType::Write);

        assert!(result.matches_direct_scan());
        assert_eq!(result.touched_and_nodes, vec!["AndRoot".to_string()]);
        assert_eq!(
            result.summary_blocks_updated,
            vec![
                "B_then".to_string(),
                "B_loop".to_string(),
                "B_left".to_string()
            ]
        );
        assert_eq!(
            result.summary_entries,
            BTreeSet::from([
                (CcaType::WriteRead, CcaEntry::new("x", "ActThen", "ActRead")),
                (CcaType::WriteKill, CcaEntry::new("x", "ActThen", "ActKill")),
            ])
        );
        assert_d_opn(&graph, "B_left", "x", OperationType::Write, &["ActThen"]);
        assert_d_opn(&graph, "B_loop", "x", OperationType::Write, &["ActThen"]);
        assert_d_opn(&graph, "B_then", "x", OperationType::Write, &["ActThen"]);
    }

    #[test]
    fn delete_inserted_operation_updates_summaries_and_cca_sets() {
        let mut graph = build_paper_example_graph();
        let initial_cca_sets = graph.nodes["And1"].cca_sets.clone();
        let insert_result = graph.insert_operation("Act2", "v", OperationType::Write);
        assert!(insert_result.matches_direct_scan());
        assert!(
            graph.nodes["And1"].cca_sets[&CcaType::WriteWrite]
                .contains(&CcaEntry::new("v", "Act2", "Act3"))
        );

        let verification =
            graph.delete_operation_with_recompute_check("Act2", "v", OperationType::Write);
        let delete_result = &verification.deletion;

        assert!(delete_result.removed_operation);
        assert!(verification.matches_recomputed_state());
        assert!(verification.recomputed_removed_operation);
        assert_eq!(delete_result.touched_and_nodes, vec!["And1".to_string()]);
        assert_eq!(
            delete_result.summary_blocks_updated,
            vec!["B3".to_string(), "B1".to_string()]
        );
        assert!(
            !graph.nodes["Act2"]
                .operations
                .contains(&Operation::new("v", OperationType::Write))
        );
        assert!(
            !graph.nodes["B1"].d_opn_set[&("v".to_string(), OperationType::Write)].contains("Act2")
        );
        assert!(
            !graph.nodes["B3"]
                .d_opn_set
                .contains_key(&("v".to_string(), OperationType::Write))
        );
        assert!(
            !graph.nodes["And1"].cca_sets[&CcaType::WriteWrite]
                .contains(&CcaEntry::new("v", "Act2", "Act3"))
        );
        assert_eq!(graph.nodes["And1"].cca_sets, initial_cca_sets);
    }

    #[test]
    fn deletion_matches_recomputed_baseline_for_nested_control_flow() {
        let initial = build_nested_control_flow_graph();
        let mut graph = initial.clone();
        let insert_result = graph.insert_operation("ActThen", "x", OperationType::Write);
        assert!(insert_result.matches_direct_scan());
        assert!(
            graph.nodes["AndRoot"].cca_sets[&CcaType::WriteRead]
                .contains(&CcaEntry::new("x", "ActThen", "ActRead"))
        );

        let verification =
            graph.delete_operation_with_recompute_check("ActThen", "x", OperationType::Write);

        assert!(verification.deletion.removed_operation);
        assert!(verification.matches_recomputed_state());
        assert_eq!(graph, initial);
    }

    #[test]
    fn deletion_verification_missing_operation_matches_recomputed_noop() {
        let mut graph = build_paper_example_graph();
        let before = graph.clone();

        let verification =
            graph.delete_operation_with_recompute_check("Act2", "v", OperationType::Write);

        assert!(!verification.deletion.removed_operation);
        assert!(!verification.recomputed_removed_operation);
        assert!(verification.matches_recomputed_state());
        assert_eq!(graph, before);
    }

    #[test]
    fn deleting_missing_operation_is_noop() {
        let mut graph = build_paper_example_graph();

        let delete_result = graph.delete_operation("Act2", "v", OperationType::Write);

        assert!(!delete_result.removed_operation);
        assert!(delete_result.touched_and_nodes.is_empty());
        assert!(delete_result.summary_blocks_updated.is_empty());
    }

    #[test]
    fn try_operations_report_missing_node_without_panicking() {
        let mut graph = build_paper_example_graph();
        let before = graph.nodes.clone();

        let insert_error = graph
            .try_insert_operation("MissingAct", "v", OperationType::Write)
            .unwrap_err();
        assert_eq!(
            insert_error,
            TTGraphError::MissingNode {
                node_id: "MissingAct".to_string()
            }
        );
        assert_eq!(graph.nodes, before);

        let delete_error = graph
            .try_delete_operation("MissingAct", "v", OperationType::Write)
            .unwrap_err();
        assert_eq!(
            delete_error,
            TTGraphError::MissingNode {
                node_id: "MissingAct".to_string()
            }
        );
        assert_eq!(graph.nodes, before);
    }

    #[test]
    fn try_insert_matches_panic_api_for_valid_graph() {
        let mut try_graph = build_paper_example_graph();
        let mut panic_graph = build_paper_example_graph();

        let try_result = try_graph
            .try_insert_operation("Act2", "v", OperationType::Write)
            .unwrap();
        let panic_result = panic_graph.insert_operation("Act2", "v", OperationType::Write);

        assert_eq!(try_result, panic_result);
        assert_eq!(try_graph.nodes, panic_graph.nodes);
    }

    #[test]
    fn try_delete_updates_graph_for_valid_graph() {
        let mut graph = build_paper_example_graph();
        graph
            .try_insert_operation("Act2", "v", OperationType::Write)
            .unwrap();

        let delete_result = graph
            .try_delete_operation_with_recompute_check("Act2", "v", OperationType::Write)
            .unwrap();

        assert!(delete_result.deletion.removed_operation);
        assert!(delete_result.matches_recomputed_state());
        assert_eq!(
            delete_result.deletion.summary_blocks_updated,
            vec!["B3".to_string(), "B1".to_string()]
        );
        assert!(
            !graph.nodes["Act2"]
                .operations
                .contains(&Operation::new("v", OperationType::Write))
        );
    }

    #[test]
    fn try_delete_missing_operation_is_successful_noop() {
        let mut graph = build_paper_example_graph();
        let before = graph.nodes.clone();

        let delete_result = graph
            .try_delete_operation("Act2", "v", OperationType::Write)
            .unwrap();

        assert!(!delete_result.removed_operation);
        assert!(delete_result.touched_and_nodes.is_empty());
        assert!(delete_result.summary_blocks_updated.is_empty());
        assert_eq!(graph.nodes, before);
    }

    #[test]
    fn try_operations_report_invalid_graph_references_without_mutating() {
        let mut graph = build_paper_example_graph();
        graph.nodes.get_mut("B1").unwrap().sequence_arc = Some("MissingAct".to_string());
        let before = graph.nodes.clone();

        let error = graph
            .try_insert_operation("Act2", "v", OperationType::Write)
            .unwrap_err();

        assert!(matches!(error, TTGraphError::InvalidGraph { .. }));
        assert!(
            error
                .to_string()
                .contains("sequence_arc pointing to non-existent node `MissingAct`")
        );
        assert_eq!(graph.nodes, before);

        let error = graph
            .try_delete_operation("Act2", "v", OperationType::Write)
            .unwrap_err();

        assert!(matches!(error, TTGraphError::InvalidGraph { .. }));
        assert_eq!(graph.nodes, before);
    }

    #[test]
    fn try_insert_reports_invalid_scope_and_branch_references() {
        let mut invalid_scope = build_paper_example_graph();
        invalid_scope.nodes.get_mut("Act2").unwrap().scope_arc = Some("MissingScope".to_string());

        let scope_error = invalid_scope
            .try_insert_operation("Act2", "v", OperationType::Write)
            .unwrap_err();
        assert!(
            scope_error
                .to_string()
                .contains("scope_arc pointing to non-existent node `MissingScope`")
        );

        let mut invalid_branch = build_paper_example_graph();
        invalid_branch
            .nodes
            .get_mut("And1")
            .unwrap()
            .branch_arc
            .push("MissingBranch".to_string());

        let branch_error = invalid_branch
            .try_insert_operation("Act2", "v", OperationType::Write)
            .unwrap_err();
        assert!(
            branch_error
                .to_string()
                .contains("branch_arc pointing to non-existent node `MissingBranch`")
        );
    }

    #[test]
    fn graph_error_display_is_readable() {
        let missing = TTGraphError::MissingNode {
            node_id: "ActX".to_string(),
        };
        assert_eq!(missing.to_string(), "missing TT Graph node `ActX`");

        let invalid = TTGraphError::InvalidGraph {
            errors: vec!["first error".to_string(), "second error".to_string()],
        };
        assert_eq!(
            invalid.to_string(),
            "invalid TT Graph: first error; second error"
        );
    }

    fn build_nested_control_flow_graph() -> TTGraph {
        let mut nodes = HashMap::new();

        nodes.insert(
            "AndRoot".to_string(),
            TTNode::control("AndRoot", ControlType::And, None)
                .with_branch_arc(vec!["B_left".to_string(), "B_right".to_string()]),
        );
        nodes.insert(
            "B_left".to_string(),
            TTNode::block("B_left", "AndRoot").with_sequence_arc("LoopOuter"),
        );
        nodes.insert(
            "LoopOuter".to_string(),
            TTNode::control("LoopOuter", ControlType::Loop, Some("B_left".to_string()))
                .with_operations(vec![Operation::new("guard", OperationType::Read)])
                .with_branch_arc(vec!["B_loop".to_string()]),
        );
        nodes.insert(
            "B_loop".to_string(),
            TTNode::block("B_loop", "LoopOuter").with_sequence_arc("XorInner"),
        );
        nodes.insert(
            "XorInner".to_string(),
            TTNode::control("XorInner", ControlType::Xor, Some("B_loop".to_string()))
                .with_operations(vec![Operation::new("x", OperationType::Read)])
                .with_branch_arc(vec!["B_then".to_string(), "B_else".to_string()]),
        );
        nodes.insert(
            "B_then".to_string(),
            TTNode::block("B_then", "XorInner").with_sequence_arc("ActThen"),
        );
        nodes.insert(
            "ActThen".to_string(),
            TTNode::activity("ActThen", "B_then")
                .with_operations(vec![Operation::new("local", OperationType::Write)]),
        );
        nodes.insert(
            "B_else".to_string(),
            TTNode::block("B_else", "XorInner").with_sequence_arc("ActElse"),
        );
        nodes.insert(
            "ActElse".to_string(),
            TTNode::activity("ActElse", "B_else")
                .with_operations(vec![Operation::new("x", OperationType::Kill)]),
        );

        nodes.insert(
            "B_right".to_string(),
            TTNode::block("B_right", "AndRoot").with_sequence_arc("AndNested"),
        );
        nodes.insert(
            "AndNested".to_string(),
            TTNode::control("AndNested", ControlType::And, Some("B_right".to_string()))
                .with_branch_arc(vec!["B_read".to_string(), "B_kill".to_string()]),
        );
        nodes.insert(
            "B_read".to_string(),
            TTNode::block("B_read", "AndNested").with_sequence_arc("ActRead"),
        );
        nodes.insert(
            "ActRead".to_string(),
            TTNode::activity("ActRead", "B_read")
                .with_operations(vec![Operation::new("x", OperationType::Read)]),
        );
        nodes.insert(
            "B_kill".to_string(),
            TTNode::block("B_kill", "AndNested").with_sequence_arc("ActKill"),
        );
        nodes.insert(
            "ActKill".to_string(),
            TTNode::activity("ActKill", "B_kill")
                .with_operations(vec![Operation::new("x", OperationType::Kill)]),
        );

        TTGraph::new(nodes)
    }

    #[test]
    fn rejects_invalid_node_references() {
        let mut nodes = HashMap::new();
        nodes.insert(
            "And1".to_string(),
            TTNode::control("And1", ControlType::And, None).with_branch_arc(vec!["B1".to_string()]),
        );
        nodes.insert(
            "B1".to_string(),
            TTNode::block("B1", "And1").with_sequence_arc("Act1"),
        );
        nodes.insert("Act1".to_string(), TTNode::activity("Act1", "B1"));

        let graph = TTGraph {
            nodes,
            children_index: HashMap::new(),
        };
        assert!(graph.validate().is_ok());

        let mut malformed_nodes = graph.nodes.clone();
        malformed_nodes.get_mut("B1").unwrap().scope_arc = Some("NonExistentAnd".to_string());
        let graph2 = TTGraph {
            nodes: malformed_nodes,
            children_index: HashMap::new(),
        };
        let errs = graph2.validate().unwrap_err();
        assert_eq!(errs.len(), 1);
        assert!(errs[0].contains("scope_arc pointing to non-existent node `NonExistentAnd`"));

        let mut malformed_nodes2 = graph.nodes.clone();
        malformed_nodes2.get_mut("B1").unwrap().sequence_arc = Some("NonExistentAct".to_string());
        let graph3 = TTGraph {
            nodes: malformed_nodes2,
            children_index: HashMap::new(),
        };
        let errs2 = graph3.validate().unwrap_err();
        assert_eq!(errs2.len(), 1);
        assert!(errs2[0].contains("sequence_arc pointing to non-existent node `NonExistentAct`"));

        let mut malformed_nodes3 = graph.nodes.clone();
        malformed_nodes3.get_mut("B1").unwrap().branch_arc = vec!["NonExistentBranch".to_string()];
        let graph4 = TTGraph {
            nodes: malformed_nodes3,
            children_index: HashMap::new(),
        };
        let errs3 = graph4.validate().unwrap_err();
        assert_eq!(errs3.len(), 1);
        assert!(errs3[0].contains("branch_arc pointing to non-existent node `NonExistentBranch`"));
    }

    fn assert_d_opn(
        graph: &TTGraph,
        block_id: &str,
        variable: &str,
        op: OperationType,
        expected: &[&str],
    ) {
        let expected_set: HashSet<String> =
            expected.iter().map(|node_id| node_id.to_string()).collect();
        assert_eq!(
            graph.nodes[block_id].d_opn_set[&(variable.to_string(), op)],
            expected_set
        );
    }
}
