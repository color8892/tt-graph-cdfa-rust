use crate::{ControlType, Operation, OperationType, TTGraph, TTNode};

pub fn parse_toy_program(source: &str) -> Result<TTGraph, String> {
    let tokens = tokenize(source)?;
    let mut parser = Parser::new(tokens);
    let root_id = parser.parse_control(None)?;
    parser.expect_end()?;
    if parser.builder.nodes.contains_key(&root_id) {
        Ok(TTGraph::new(parser.builder.nodes))
    } else {
        Err("parser did not produce a root control node".to_string())
    }
}

#[derive(Debug)]
struct Parser {
    tokens: Vec<String>,
    position: usize,
    builder: crate::builder::GraphBuilder,
}

impl Parser {
    fn new(tokens: Vec<String>) -> Self {
        Self {
            tokens,
            position: 0,
            builder: crate::builder::GraphBuilder::new(),
        }
    }

    fn parse_control(&mut self, scope_block_id: Option<String>) -> Result<String, String> {
        let keyword = self.next()?;
        match keyword.as_str() {
            "and" => self.parse_branching_control(ControlType::And, scope_block_id),
            "xor" => self.parse_branching_control(ControlType::Xor, scope_block_id),
            "loop" => self.parse_loop(scope_block_id),
            _ => Err(format!("expected control keyword, found `{keyword}`")),
        }
    }

    fn parse_branching_control(
        &mut self,
        control_type: ControlType,
        scope_block_id: Option<String>,
    ) -> Result<String, String> {
        let control_id = self.next_identifier("control id")?;
        let operations = self.parse_optional_operations()?;
        self.builder.insert_unique_node(
            control_id.clone(),
            TTNode::control(control_id.clone(), control_type, scope_block_id)
                .with_operations(operations),
        )?;

        self.expect("{")?;
        let mut branch_ids = Vec::new();
        while !self.consume("}") {
            self.expect("branch")?;
            let block_id = self.next_identifier("branch block id")?;
            branch_ids.push(block_id.clone());
            self.builder.insert_unique_node(
                block_id.clone(),
                TTNode::block(block_id.clone(), control_id.clone()),
            )?;
            self.expect("{")?;
            self.parse_block_items(&block_id)?;
            self.expect("}")?;
        }

        if branch_ids.is_empty() {
            return Err(format!("{control_id} must contain at least one branch"));
        }

        self.builder
            .nodes
            .get_mut(&control_id)
            .expect("control node exists")
            .branch_arc = branch_ids;
        Ok(control_id)
    }

    fn parse_loop(&mut self, scope_block_id: Option<String>) -> Result<String, String> {
        let control_id = self.next_identifier("loop control id")?;
        let operations = self.parse_optional_operations()?;
        self.expect("body")?;
        let block_id = self.next_identifier("loop body block id")?;

        self.builder.insert_unique_node(
            control_id.clone(),
            TTNode::control(control_id.clone(), ControlType::Loop, scope_block_id)
                .with_operations(operations)
                .with_branch_arc(vec![block_id.clone()]),
        )?;
        self.builder.insert_unique_node(
            block_id.clone(),
            TTNode::block(block_id.clone(), control_id.clone()),
        )?;

        self.expect("{")?;
        self.parse_block_items(&block_id)?;
        self.expect("}")?;
        Ok(control_id)
    }

    fn parse_block_items(&mut self, block_id: &str) -> Result<(), String> {
        let mut previous_item_id: Option<String> = None;
        while !self.peek_is("}") {
            let item_id = match self.peek().map(String::as_str) {
                Some("activity") => self.parse_activity(block_id)?,
                Some("and" | "xor" | "loop") => self.parse_control(Some(block_id.to_string()))?,
                Some(token) => {
                    return Err(format!(
                        "expected activity/control item in {block_id}, found `{token}`"
                    ));
                }
                None => return Err(format!("unclosed block {block_id}")),
            };

            self.builder
                .link_item(block_id, &mut previous_item_id, &item_id);
        }
        Ok(())
    }

    fn parse_activity(&mut self, block_id: &str) -> Result<String, String> {
        self.expect("activity")?;
        let activity_id = self.next_identifier("activity id")?;
        let operations = self.parse_required_operations()?;

        self.builder.insert_unique_node(
            activity_id.clone(),
            TTNode::activity(activity_id.clone(), block_id.to_string()).with_operations(operations),
        )?;
        Ok(activity_id)
    }

    fn parse_optional_operations(&mut self) -> Result<Vec<Operation>, String> {
        if self.consume("ops") {
            self.parse_required_operations()
        } else {
            Ok(Vec::new())
        }
    }

    fn parse_required_operations(&mut self) -> Result<Vec<Operation>, String> {
        self.expect("{")?;

        let mut operations = Vec::new();
        while !self.consume("}") {
            let op = match self.next()?.as_str() {
                "write" => OperationType::Write,
                "read" => OperationType::Read,
                "kill" => OperationType::Kill,
                token => return Err(format!("expected operation, found `{token}`")),
            };
            let variable = self.next_identifier("variable")?;
            self.expect(";")?;
            operations.push(Operation::new(variable, op));
        }
        Ok(operations)
    }

    fn expect_end(&self) -> Result<(), String> {
        if self.position == self.tokens.len() {
            Ok(())
        } else {
            Err(format!(
                "unexpected trailing token `{}`",
                self.tokens[self.position]
            ))
        }
    }

    fn expect(&mut self, expected: &str) -> Result<(), String> {
        let actual = self.next()?;
        if actual == expected {
            Ok(())
        } else {
            Err(format!("expected `{expected}`, found `{actual}`"))
        }
    }

    fn consume(&mut self, expected: &str) -> bool {
        if self.peek_is(expected) {
            self.position += 1;
            true
        } else {
            false
        }
    }

    fn peek_is(&self, expected: &str) -> bool {
        self.peek().is_some_and(|token| token == expected)
    }

    fn peek(&self) -> Option<&String> {
        self.tokens.get(self.position)
    }

    fn next(&mut self) -> Result<String, String> {
        let token = self
            .tokens
            .get(self.position)
            .ok_or_else(|| "unexpected end of input".to_string())?
            .clone();
        self.position += 1;
        Ok(token)
    }

    fn next_identifier(&mut self, label: &str) -> Result<String, String> {
        let token = self.next()?;
        if token
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
        {
            Ok(token)
        } else {
            Err(format!("expected {label}, found `{token}`"))
        }
    }
}

fn tokenize(source: &str) -> Result<Vec<String>, String> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut chars = source.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '/' && chars.peek() == Some(&'/') {
            chars.next();
            for comment_ch in chars.by_ref() {
                if comment_ch == '\n' {
                    break;
                }
            }
            flush_token(&mut tokens, &mut current);
            continue;
        }

        if ch.is_ascii_whitespace() {
            flush_token(&mut tokens, &mut current);
        } else if matches!(ch, '{' | '}' | ';') {
            flush_token(&mut tokens, &mut current);
            tokens.push(ch.to_string());
        } else if ch.is_ascii_alphanumeric() || ch == '_' {
            current.push(ch);
        } else {
            return Err(format!("unsupported character `{ch}`"));
        }
    }

    flush_token(&mut tokens, &mut current);
    Ok(tokens)
}

fn flush_token(tokens: &mut Vec<String>, current: &mut String) {
    if !current.is_empty() {
        tokens.push(std::mem::take(current));
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::*;
    use crate::{OperationType, build_paper_example_graph};

    const PAPER_PROGRAM: &str = include_str!("../examples/program1.tt");

    #[test]
    fn parses_program_1_into_matching_d_opn_sets() {
        let parsed = parse_toy_program(PAPER_PROGRAM).expect("toy program parses");
        let hardcoded = build_paper_example_graph();

        for block_id in ["B1", "B2", "B3", "B4", "B5"] {
            assert_eq!(
                parsed.nodes[block_id].d_opn_set,
                hardcoded.nodes[block_id].d_opn_set
            );
        }
    }

    #[test]
    fn parsed_program_reproduces_program_2_insertion() {
        let mut parsed = parse_toy_program(PAPER_PROGRAM).expect("toy program parses");
        let result = parsed.insert_operation("Act2", "v", OperationType::Write);

        assert!(result.matches_direct_scan());
        assert_eq!(result.summary_entries.len(), 7);
        assert_eq!(
            parsed.nodes["B1"].d_opn_set[&("v".to_string(), OperationType::Write)],
            HashSet::from(["Act1".to_string(), "Act2".to_string()])
        );
    }
}
