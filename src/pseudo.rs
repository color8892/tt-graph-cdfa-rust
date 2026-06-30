use crate::{ControlType, Operation, OperationType, TTGraph, TTNode};

pub fn parse_pseudo_program(source: &str) -> Result<TTGraph, String> {
    let lines = logical_lines(source);
    let mut parser = Parser::new(lines);
    parser.parse_split(None)?;
    parser.expect_end()?;
    Ok(TTGraph::new(parser.builder.nodes))
}

#[derive(Debug)]
struct Parser {
    lines: Vec<Vec<String>>,
    position: usize,
    builder: crate::builder::GraphBuilder,
}

impl Parser {
    fn new(lines: Vec<Vec<String>>) -> Self {
        Self {
            lines,
            position: 0,
            builder: crate::builder::GraphBuilder::new(),
        }
    }

    fn parse_split(&mut self, scope_block_id: Option<&str>) -> Result<String, String> {
        let line = self.next_line()?;
        if line.len() != 2 || line[0] != "split" {
            return Err(format!("expected `split <id>`, found `{}`", line.join(" ")));
        }

        let control_id = line[1].clone();
        self.builder.insert_unique_node(
            control_id.clone(),
            TTNode::control(
                control_id.clone(),
                ControlType::And,
                scope_block_id.map(str::to_string),
            ),
        )?;

        let mut branch_ids = Vec::new();
        while !self.consume_line(&["join"]) {
            let branch_id = self.parse_branch(&control_id)?;
            branch_ids.push(branch_id);
        }

        self.builder
            .nodes
            .get_mut(&control_id)
            .expect("AND control exists")
            .branch_arc = branch_ids;
        Ok(control_id)
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
        self.builder.insert_unique_node(
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
                Some("split" | "while" | "if") => {
                    self.builder.flush_activity(
                        block_id,
                        &mut previous_item_id,
                        &mut pending_operations,
                    )?;
                    match self.peek_line().map(|line| line[0].as_str()) {
                        Some("split") => self.parse_split(Some(block_id))?,
                        Some("while") => self.parse_while(block_id)?,
                        Some("if") => self.parse_if(block_id)?,
                        _ => unreachable!(),
                    }
                }
                Some(token) => {
                    return Err(format!(
                        "expected operation/split/while/if in {block_id}, found `{token}`"
                    ));
                }
                None => return Err(format!("unclosed block {block_id}")),
            };
            self.builder
                .link_item(block_id, &mut previous_item_id, &item_id);
        }

        self.builder
            .flush_activity(block_id, &mut previous_item_id, &mut pending_operations)?;
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

        let header = parse_while_header(&line)?;
        let (control_id, body_block_id) = match (header.control_id, header.body_block_id) {
            (Some(cid), Some(bid)) => (cid, bid),
            _ => self.builder.next_loop_names(),
        };
        self.builder.insert_unique_node(
            control_id.clone(),
            TTNode::control(
                control_id.clone(),
                ControlType::Loop,
                Some(scope_block_id.to_string()),
            )
            .with_operations(condition_read_operations(header.condition_tokens))
            .with_branch_arc(vec![body_block_id.clone()]),
        )?;
        self.builder.insert_unique_node(
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

        let header = parse_if_header(&line)?;
        let (control_id, then_block_id, else_block_id) = match (
            header.control_id,
            header.then_block_id,
            header.else_block_id,
        ) {
            (Some(cid), Some(tbid), Some(ebid)) => (cid, tbid, ebid),
            _ => self.builder.next_xor_names(),
        };

        self.builder.insert_unique_node(
            control_id.clone(),
            TTNode::control(
                control_id.clone(),
                ControlType::Xor,
                Some(scope_block_id.to_string()),
            )
            .with_operations(condition_read_operations(header.condition_tokens))
            .with_branch_arc(vec![then_block_id.clone(), else_block_id.clone()]),
        )?;
        self.builder.insert_unique_node(
            then_block_id.clone(),
            TTNode::block(then_block_id.clone(), control_id.clone()),
        )?;
        self.parse_block_items(&then_block_id, &["else"])?;
        self.expect_line(&["else"])?;

        self.builder.insert_unique_node(
            else_block_id.clone(),
            TTNode::block(else_block_id.clone(), control_id.clone()),
        )?;
        self.parse_block_items(&else_block_id, &["endif", "end if"])?;
        self.expect_any_line(&[&["endif"], &["end", "if"]])?;
        Ok(control_id)
    }

    fn parse_statement_operations(&mut self) -> Result<Vec<Operation>, String> {
        let line = self.next_line()?;
        operations_for_statement(&line)
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

struct WhileHeader<'a> {
    control_id: Option<String>,
    condition_tokens: &'a [String],
    body_block_id: Option<String>,
}

struct IfHeader<'a> {
    control_id: Option<String>,
    condition_tokens: &'a [String],
    then_block_id: Option<String>,
    else_block_id: Option<String>,
}

fn parse_while_header(line: &[String]) -> Result<WhileHeader<'_>, String> {
    if let Some(body_index) = line.iter().position(|token| token == "body") {
        if body_index + 1 >= line.len() {
            return Err(format!(
                "while line is missing body block id: `{}`",
                line.join(" ")
            ));
        }
        return Ok(WhileHeader {
            control_id: Some(line[1].clone()),
            condition_tokens: &line[2..body_index],
            body_block_id: Some(line[body_index + 1].clone()),
        });
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
    Ok(WhileHeader {
        control_id: None,
        condition_tokens: &line[1..do_index],
        body_block_id: None,
    })
}

fn parse_if_header(line: &[String]) -> Result<IfHeader<'_>, String> {
    let then_index = line
        .iter()
        .position(|token| token == "then")
        .ok_or_else(|| format!("if line is missing `then`: `{}`", line.join(" ")))?;

    if let Some(else_index) = line.iter().position(|token| token == "else") {
        if then_index + 1 >= line.len() || else_index + 1 >= line.len() || then_index >= else_index
        {
            return Err(format!("invalid if line: `{}`", line.join(" ")));
        }
        return Ok(IfHeader {
            control_id: Some(line[1].clone()),
            condition_tokens: &line[2..then_index],
            then_block_id: Some(line[then_index + 1].clone()),
            else_block_id: Some(line[else_index + 1].clone()),
        });
    }

    if then_index <= 1 {
        return Err(format!(
            "if line is missing condition: `{}`",
            line.join(" ")
        ));
    }
    Ok(IfHeader {
        control_id: None,
        condition_tokens: &line[1..then_index],
        then_block_id: None,
        else_block_id: None,
    })
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
    use crate::{CcaEntry, CcaType, OperationType, build_paper_example_graph};

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

    #[test]
    fn parses_nested_split_inside_branch() {
        let mut parsed = parse_pseudo_program(
            r#"
            split AndRoot
            branch B_left
            split AndInner
            branch B_then
            read x
            endbranch
            branch B_else
            kill x
            endbranch
            join
            endbranch
            branch B_right
            read x
            endbranch
            join
            "#,
        )
        .expect("nested split parses");

        assert_eq!(
            parsed.nodes["B_left"].d_opn_set[&("x".to_string(), OperationType::Read)],
            HashSet::from(["Act1".to_string()])
        );
        assert_eq!(
            parsed.nodes["B_left"].d_opn_set[&("x".to_string(), OperationType::Kill)],
            HashSet::from(["Act2".to_string()])
        );

        let result = parsed.insert_operation("Act1", "x", OperationType::Write);

        assert!(result.matches_direct_scan());
        assert_eq!(
            result.touched_and_nodes,
            vec!["AndInner".to_string(), "AndRoot".to_string()]
        );
        assert_eq!(result.summary_entries.len(), 2);
        assert!(
            result
                .summary_entries
                .contains(&(CcaType::WriteKill, CcaEntry::new("x", "Act1", "Act2")))
        );
        assert!(
            result
                .summary_entries
                .contains(&(CcaType::WriteRead, CcaEntry::new("x", "Act1", "Act3")))
        );
    }

    #[test]
    fn rejects_duplicate_nested_split_ids() {
        let error = parse_pseudo_program(
            r#"
            split And1
            branch B1
            split And1
            branch B2
            read x
            endbranch
            join
            endbranch
            join
            "#,
        )
        .expect_err("duplicate control id is rejected");

        assert!(error.contains("duplicate node id `And1`"));
    }
}
