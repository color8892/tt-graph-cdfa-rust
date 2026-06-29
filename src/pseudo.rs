use std::collections::HashMap;

use crate::{ControlType, Operation, OperationType, TTGraph, TTNode};

pub fn parse_pseudo_program(source: &str) -> Result<TTGraph, String> {
    let lines = logical_lines(source);
    let mut parser = Parser::new(lines);
    parser.parse_split()?;
    parser.expect_end()?;
    Ok(TTGraph::new(parser.nodes))
}

#[derive(Debug)]
struct Parser {
    lines: Vec<Vec<String>>,
    position: usize,
    nodes: HashMap<String, TTNode>,
    next_activity: usize,
}

impl Parser {
    fn new(lines: Vec<Vec<String>>) -> Self {
        Self {
            lines,
            position: 0,
            nodes: HashMap::new(),
            next_activity: 1,
        }
    }

    fn parse_split(&mut self) -> Result<(), String> {
        let line = self.next_line()?;
        if line.as_slice() != ["split", "And1"] {
            return Err("expected `split And1` as the root control".to_string());
        }

        self.insert_unique_node(
            "And1".to_string(),
            TTNode::control("And1", ControlType::And, None),
        )?;

        let mut branch_ids = Vec::new();
        while !self.consume_line(&["join"]) {
            let branch_id = self.parse_branch("And1")?;
            branch_ids.push(branch_id);
        }

        self.nodes.get_mut("And1").expect("And1 exists").branch_arc = branch_ids;
        Ok(())
    }

    fn parse_branch(&mut self, control_id: &str) -> Result<String, String> {
        let line = self.next_line()?;
        if line.len() != 2 || line[0] != "branch" {
            return Err(format!(
                "expected `branch <id>`, found `{}`",
                line.join(" ")
            ));
        }

        let block_id = line[1].clone();
        self.insert_unique_node(
            block_id.clone(),
            TTNode::block(block_id.clone(), control_id.to_string()),
        )?;
        self.parse_block_items(&block_id, &["endbranch"])?;
        self.expect_line(&["endbranch"])?;
        Ok(block_id)
    }

    fn parse_block_items(&mut self, block_id: &str, stop_words: &[&str]) -> Result<(), String> {
        let mut previous_item_id: Option<String> = None;
        let mut pending_operations = Vec::new();

        while !self
            .peek_line()
            .is_none_or(|line| line_is_any_stop(line, stop_words))
        {
            let item_id = match self.peek_line().map(|line| line[0].as_str()) {
                Some("read" | "write" | "kill" | "print") => {
                    pending_operations.extend(self.parse_statement_operations()?);
                    continue;
                }
                Some(_)
                    if self
                        .peek_line()
                        .is_some_and(|line| is_assignment_line(line)) =>
                {
                    pending_operations.extend(self.parse_statement_operations()?);
                    continue;
                }
                Some("while" | "if") => {
                    self.flush_activity(block_id, &mut previous_item_id, &mut pending_operations)?;
                    match self.peek_line().map(|line| line[0].as_str()) {
                        Some("while") => self.parse_while(block_id)?,
                        Some("if") => self.parse_if(block_id)?,
                        _ => unreachable!(),
                    }
                }
                Some(token) => {
                    return Err(format!(
                        "expected operation/while/if in {block_id}, found `{token}`"
                    ));
                }
                None => return Err(format!("unclosed block {block_id}")),
            };
            self.link_item(block_id, &mut previous_item_id, &item_id);
        }

        self.flush_activity(block_id, &mut previous_item_id, &mut pending_operations)?;
        Ok(())
    }

    fn parse_while(&mut self, scope_block_id: &str) -> Result<String, String> {
        let line = self.next_line()?;
        if line.len() < 4 || line[0] != "while" {
            return Err(format!(
                "expected `while <id> <condition> body <block>` or `while <condition> do`, found `{}`",
                line.join(" ")
            ));
        }

        let (control_id, condition_tokens, body_block_id) = parse_while_header(&line)?;
        self.insert_unique_node(
            control_id.clone(),
            TTNode::control(
                control_id.clone(),
                ControlType::Loop,
                Some(scope_block_id.to_string()),
            )
            .with_operations(condition_read_operations(condition_tokens))
            .with_branch_arc(vec![body_block_id.clone()]),
        )?;
        self.insert_unique_node(
            body_block_id.clone(),
            TTNode::block(body_block_id.clone(), control_id.clone()),
        )?;
        self.parse_block_items(&body_block_id, &["endwhile", "end while"])?;
        self.expect_any_line(&[&["endwhile"], &["end", "while"]])?;
        Ok(control_id)
    }

    fn parse_if(&mut self, scope_block_id: &str) -> Result<String, String> {
        let line = self.next_line()?;
        if line.len() < 4 || line[0] != "if" {
            return Err(format!(
                "expected `if <id> <condition> then <block> else <block>` or `if <condition> then`, found `{}`",
                line.join(" ")
            ));
        }

        let (control_id, condition_tokens, then_block_id, else_block_id) = parse_if_header(&line)?;

        self.insert_unique_node(
            control_id.clone(),
            TTNode::control(
                control_id.clone(),
                ControlType::Xor,
                Some(scope_block_id.to_string()),
            )
            .with_operations(condition_read_operations(condition_tokens))
            .with_branch_arc(vec![then_block_id.clone(), else_block_id.clone()]),
        )?;
        self.insert_unique_node(
            then_block_id.clone(),
            TTNode::block(then_block_id.clone(), control_id.clone()),
        )?;
        self.parse_block_items(&then_block_id, &["else"])?;
        self.expect_line(&["else"])?;

        self.insert_unique_node(
            else_block_id.clone(),
            TTNode::block(else_block_id.clone(), control_id.clone()),
        )?;
        self.parse_block_items(&else_block_id, &["endif", "end if"])?;
        self.expect_any_line(&[&["endif"], &["end", "if"]])?;
        Ok(control_id)
    }

    fn flush_activity(
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

    fn link_item(&mut self, block_id: &str, previous_item_id: &mut Option<String>, item_id: &str) {
        if let Some(previous_item_id) = previous_item_id {
            self.nodes
                .get_mut(previous_item_id)
                .expect("previous item exists")
                .sequence_arc = Some(item_id.to_string());
        } else {
            self.nodes
                .get_mut(block_id)
                .expect("block exists")
                .sequence_arc = Some(item_id.to_string());
        }
        *previous_item_id = Some(item_id.to_string());
    }

    fn parse_statement_operations(&mut self) -> Result<Vec<Operation>, String> {
        let line = self.next_line()?;
        operations_for_statement(&line)
    }

    fn insert_unique_node(&mut self, node_id: String, node: TTNode) -> Result<(), String> {
        if self.nodes.contains_key(&node_id) {
            return Err(format!("duplicate node id `{node_id}`"));
        }
        self.nodes.insert(node_id, node);
        Ok(())
    }

    fn expect_end(&self) -> Result<(), String> {
        if self.position == self.lines.len() {
            Ok(())
        } else {
            Err(format!(
                "unexpected trailing line `{}`",
                self.lines[self.position].join(" ")
            ))
        }
    }

    fn expect_line(&mut self, expected: &[&str]) -> Result<(), String> {
        let line = self.next_line()?;
        if line.iter().map(String::as_str).eq(expected.iter().copied()) {
            Ok(())
        } else {
            Err(format!(
                "expected `{}`, found `{}`",
                expected.join(" "),
                line.join(" ")
            ))
        }
    }

    fn expect_any_line(&mut self, expected_options: &[&[&str]]) -> Result<(), String> {
        let line = self.next_line()?;
        if expected_options
            .iter()
            .any(|expected| line.iter().map(String::as_str).eq(expected.iter().copied()))
        {
            Ok(())
        } else {
            let expected = expected_options
                .iter()
                .map(|tokens| tokens.join(" "))
                .collect::<Vec<_>>()
                .join(" or ");
            Err(format!("expected `{expected}`, found `{}`", line.join(" ")))
        }
    }

    fn consume_line(&mut self, expected: &[&str]) -> bool {
        if self
            .peek_line()
            .is_some_and(|line| line.iter().map(String::as_str).eq(expected.iter().copied()))
        {
            self.position += 1;
            true
        } else {
            false
        }
    }

    fn peek_line(&self) -> Option<&Vec<String>> {
        self.lines.get(self.position)
    }

    fn next_line(&mut self) -> Result<Vec<String>, String> {
        let line = self
            .lines
            .get(self.position)
            .ok_or_else(|| "unexpected end of input".to_string())?
            .clone();
        self.position += 1;
        Ok(line)
    }
}

fn logical_lines(source: &str) -> Vec<Vec<String>> {
    source
        .lines()
        .filter_map(|line| {
            let without_comment = line.split("//").next().unwrap_or("").trim();
            if without_comment.is_empty() {
                None
            } else {
                let normalized = without_comment
                    .replace(['(', ')', ',', ';'], " ")
                    .replace(":=", " := ");
                Some(normalized.split_whitespace().map(str::to_string).collect())
            }
        })
        .collect()
}

fn line_is_any_stop(line: &[String], stop_words: &[&str]) -> bool {
    stop_words.iter().any(|stop_word| {
        let expected: Vec<&str> = stop_word.split_whitespace().collect();
        line.iter().map(String::as_str).eq(expected.iter().copied())
    })
}

fn parse_while_header(line: &[String]) -> Result<(String, &[String], String), String> {
    if let Some(body_index) = line.iter().position(|token| token == "body") {
        if body_index + 1 >= line.len() {
            return Err(format!(
                "while line is missing body block id: `{}`",
                line.join(" ")
            ));
        }
        return Ok((
            line[1].clone(),
            &line[2..body_index],
            line[body_index + 1].clone(),
        ));
    }

    let do_index = line
        .iter()
        .position(|token| token == "do")
        .ok_or_else(|| format!("while line is missing `body` or `do`: `{}`", line.join(" ")))?;
    if do_index <= 1 {
        return Err(format!(
            "while line is missing condition: `{}`",
            line.join(" ")
        ));
    }
    Ok(("Loop1".to_string(), &line[1..do_index], "B3".to_string()))
}

fn parse_if_header(line: &[String]) -> Result<(String, &[String], String, String), String> {
    let then_index = line
        .iter()
        .position(|token| token == "then")
        .ok_or_else(|| format!("if line is missing `then`: `{}`", line.join(" ")))?;

    if let Some(else_index) = line.iter().position(|token| token == "else") {
        if then_index + 1 >= line.len() || else_index + 1 >= line.len() || then_index >= else_index
        {
            return Err(format!("invalid if line: `{}`", line.join(" ")));
        }
        return Ok((
            line[1].clone(),
            &line[2..then_index],
            line[then_index + 1].clone(),
            line[else_index + 1].clone(),
        ));
    }

    if then_index <= 1 {
        return Err(format!(
            "if line is missing condition: `{}`",
            line.join(" ")
        ));
    }
    Ok((
        "Xor1".to_string(),
        &line[1..then_index],
        "B4".to_string(),
        "B5".to_string(),
    ))
}

fn operations_for_statement(line: &[String]) -> Result<Vec<Operation>, String> {
    match line.first().map(String::as_str) {
        Some("read") | Some("write") | Some("kill") => explicit_operation(line),
        Some("print") => Ok(condition_read_operations(&line[1..])),
        Some(_) if is_assignment_line(line) => assignment_operations(line),
        Some(token) => Err(format!("unsupported statement `{token}`")),
        None => Err("empty statement".to_string()),
    }
}

fn explicit_operation(line: &[String]) -> Result<Vec<Operation>, String> {
    if line.len() != 2 {
        return Err(format!("expected `<op> <var>`, found `{}`", line.join(" ")));
    }
    let op = match line[0].as_str() {
        "read" => OperationType::Read,
        "write" => OperationType::Write,
        "kill" => OperationType::Kill,
        _ => return Err(format!("expected operation, found `{}`", line[0])),
    };
    Ok(vec![Operation::new(line[1].clone(), op)])
}

fn assignment_operations(line: &[String]) -> Result<Vec<Operation>, String> {
    if line.len() < 3 || line[1] != ":=" {
        return Err(format!(
            "expected `<var> := <expr>`, found `{}`",
            line.join(" ")
        ));
    }

    let mut operations = condition_read_operations(&line[2..]);
    operations.push(Operation::new(line[0].clone(), OperationType::Write));
    Ok(operations)
}

fn condition_read_operations(tokens: &[String]) -> Vec<Operation> {
    identifiers(tokens)
        .into_iter()
        .map(|variable| Operation::new(variable, OperationType::Read))
        .collect()
}

fn identifiers(tokens: &[String]) -> Vec<String> {
    let mut result = Vec::new();
    for token in tokens {
        if is_identifier(token) && !is_keyword(token) && !result.contains(token) {
            result.push(token.clone());
        }
    }
    result
}

fn is_assignment_line(line: &[String]) -> bool {
    line.len() >= 3 && is_identifier(&line[0]) && line[1] == ":="
}

fn is_identifier(token: &str) -> bool {
    let mut chars = token.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    (first.is_ascii_alphabetic() || first == '_')
        && chars.all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
}

fn is_keyword(token: &str) -> bool {
    matches!(
        token,
        "split"
            | "branch"
            | "while"
            | "body"
            | "if"
            | "then"
            | "else"
            | "join"
            | "endbranch"
            | "endwhile"
            | "endif"
            | "mod"
            | "do"
            | "read"
            | "write"
            | "kill"
            | "print"
    )
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::*;
    use crate::{OperationType, build_paper_example_graph};

    const PAPER_PSEUDO: &str = include_str!("../examples/program1.pseudo");

    #[test]
    fn parses_paper_pseudo_into_matching_d_opn_sets() {
        let parsed = parse_pseudo_program(PAPER_PSEUDO).expect("pseudo program parses");
        let hardcoded = build_paper_example_graph();

        for block_id in ["B1", "B2", "B3", "B4", "B5"] {
            assert_eq!(
                parsed.nodes[block_id].d_opn_set,
                hardcoded.nodes[block_id].d_opn_set
            );
        }
    }

    #[test]
    fn parsed_pseudo_reproduces_program_2_insertion() {
        let mut parsed = parse_pseudo_program(PAPER_PSEUDO).expect("pseudo program parses");
        let result = parsed.insert_operation("Act2", "v", OperationType::Write);

        assert!(result.matches_direct_scan());
        assert_eq!(result.summary_entries.len(), 7);
        assert_eq!(
            parsed.nodes["B1"].d_opn_set[&("v".to_string(), OperationType::Write)],
            HashSet::from(["Act1".to_string(), "Act2".to_string()])
        );
    }
}
