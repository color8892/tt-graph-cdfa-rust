use crate::{Operation, TTNode};
use std::collections::HashMap;

/// A shared builder to construct a TT Graph node-by-node and maintain structural sequence arcs.
#[derive(Debug, Clone)]
pub struct GraphBuilder {
    pub nodes: HashMap<String, TTNode>,
    pub next_activity: usize,
    pub next_loop: usize,
    pub next_xor: usize,
}

impl GraphBuilder {
    /// Creates a new, empty GraphBuilder.
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
            next_activity: 1,
            next_loop: 1,
            next_xor: 1,
        }
    }

    /// Inserts a node into the graph, ensuring that its ID is unique.
    pub fn insert_unique_node(&mut self, node_id: String, node: TTNode) -> Result<(), String> {
        if self.nodes.contains_key(&node_id) {
            return Err(format!("duplicate node id `{node_id}`"));
        }
        self.nodes.insert(node_id, node);
        Ok(())
    }

    /// Links a new item sequentially to a previous item or sets it as the first item of a block.
    pub fn link_item(
        &mut self,
        block_id: &str,
        previous_item_id: &mut Option<String>,
        item_id: &str,
    ) {
        if let Some(prev_id) = previous_item_id {
            if let Some(node) = self.nodes.get_mut(prev_id) {
                node.sequence_arc = Some(item_id.to_string());
            }
        } else {
            if let Some(node) = self.nodes.get_mut(block_id) {
                node.sequence_arc = Some(item_id.to_string());
            }
        }
        *previous_item_id = Some(item_id.to_string());
    }

    /// Flushes any pending operations into a newly created activity node and links it.
    pub fn flush_activity(
        &mut self,
        block_id: &str,
        previous_item_id: &mut Option<String>,
        pending_operations: &mut Vec<Operation>,
    ) -> Result<(), String> {
        if pending_operations.is_empty() {
            return Ok(());
        }

        let activity_id = format!("Act{}", self.next_activity);
        self.next_activity += 1;
        self.insert_unique_node(
            activity_id.clone(),
            TTNode::activity(activity_id.clone(), block_id.to_string())
                .with_operations(std::mem::take(pending_operations)),
        )?;
        self.link_item(block_id, previous_item_id, &activity_id);
        Ok(())
    }

    /// Generates the next sequential loop and loop block names.
    pub fn next_loop_names(&mut self) -> (String, String) {
        let control_id = if self.next_loop == 1 {
            "Loop1".to_string()
        } else {
            format!("Loop{}", self.next_loop)
        };
        let body_block_id = if self.next_loop == 1 {
            "B3".to_string()
        } else {
            format!("B_loop_{}", self.next_loop)
        };
        self.next_loop += 1;
        (control_id, body_block_id)
    }

    /// Generates the next sequential conditional xor and branches names.
    pub fn next_xor_names(&mut self) -> (String, String, String) {
        let control_id = if self.next_xor == 1 {
            "Xor1".to_string()
        } else {
            format!("Xor{}", self.next_xor)
        };
        let then_block_id = if self.next_xor == 1 {
            "B4".to_string()
        } else {
            format!("B_then_{}", self.next_xor)
        };
        let else_block_id = if self.next_xor == 1 {
            "B5".to_string()
        } else {
            format!("B_else_{}", self.next_xor)
        };
        self.next_xor += 1;
        (control_id, then_block_id, else_block_id)
    }
}

impl Default for GraphBuilder {
    fn default() -> Self {
        Self::new()
    }
}
