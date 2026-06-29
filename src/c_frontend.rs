use crate::{ControlType, Operation, OperationType, TTGraph, TTNode};

pub fn parse_c_program(source: &str) -> Result<TTGraph, String> {
    let tokens = tokenize(source)?;
    let mut parser = Parser::new(tokens);
    parser.parse_program()
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum Token {
    Keyword(Keyword),
    Ident(String),
    Number(String),
    Symbol(char),
    Eof,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Keyword {
    Parallel,
    Branch,
    While,
    If,
    Else,
    Print,
    Kill,
}

struct Parser {
    tokens: Vec<Token>,
    position: usize,
    builder: crate::builder::GraphBuilder,
}

impl Parser {
    fn new(tokens: Vec<Token>) -> Self {
        Self {
            tokens,
            position: 0,
            builder: crate::builder::GraphBuilder::new(),
        }
    }

    fn parse_program(&mut self) -> Result<TTGraph, String> {
        let root = self.parse_parallel(None)?;
        if !matches!(self.peek(), Token::Eof) {
            return Err(format!("unexpected token after `{root}`"));
        }
        Ok(TTGraph::new(std::mem::take(&mut self.builder.nodes)))
    }

    fn parse_parallel(&mut self, scope_block_id: Option<&str>) -> Result<String, String> {
        self.expect_keyword(Keyword::Parallel)?;
        let control_id = self.expect_ident()?;
        self.builder.insert_unique_node(
            control_id.clone(),
            TTNode::control(
                control_id.clone(),
                ControlType::And,
                scope_block_id.map(str::to_string),
            ),
        )?;

        self.expect_symbol('{')?;
        let mut branch_ids = Vec::new();
        while matches!(self.peek(), Token::Keyword(Keyword::Branch)) {
            branch_ids.push(self.parse_branch(&control_id)?);
        }
        self.expect_symbol('}')?;

        self.builder
            .nodes
            .get_mut(&control_id)
            .ok_or_else(|| format!("missing AND node `{control_id}`"))?
            .branch_arc = branch_ids;
        Ok(control_id)
    }

    fn parse_branch(&mut self, control_id: &str) -> Result<String, String> {
        self.expect_keyword(Keyword::Branch)?;
        let block_id = self.expect_ident()?;
        self.builder.insert_unique_node(
            block_id.clone(),
            TTNode::block(block_id.clone(), control_id.to_string()),
        )?;
        self.parse_block(&block_id)?;
        Ok(block_id)
    }

    fn parse_block(&mut self, block_id: &str) -> Result<(), String> {
        self.expect_symbol('{')?;
        let mut previous_item_id: Option<String> = None;
        let mut pending_operations = Vec::new();

        while !matches!(self.peek(), Token::Symbol('}')) {
            match self.peek() {
                Token::Keyword(Keyword::Print | Keyword::Kill) => {
                    pending_operations.extend(self.parse_simple_statement()?);
                }
                Token::Ident(_) if self.peek_is_assignment() => {
                    pending_operations.extend(self.parse_assignment()?);
                }
                Token::Keyword(Keyword::Parallel) => {
                    self.builder.flush_activity(
                        block_id,
                        &mut previous_item_id,
                        &mut pending_operations,
                    )?;
                    let nested_id = self.parse_parallel(Some(block_id))?;
                    self.builder
                        .link_item(block_id, &mut previous_item_id, &nested_id);
                }
                Token::Keyword(Keyword::While) => {
                    self.builder.flush_activity(
                        block_id,
                        &mut previous_item_id,
                        &mut pending_operations,
                    )?;
                    let loop_id = self.parse_while(block_id)?;
                    self.builder
                        .link_item(block_id, &mut previous_item_id, &loop_id);
                }
                Token::Keyword(Keyword::If) => {
                    self.builder.flush_activity(
                        block_id,
                        &mut previous_item_id,
                        &mut pending_operations,
                    )?;
                    let xor_id = self.parse_if(block_id)?;
                    self.builder
                        .link_item(block_id, &mut previous_item_id, &xor_id);
                }
                token => return Err(format!("unexpected statement in `{block_id}`: {token:?}")),
            }
        }

        self.builder
            .flush_activity(block_id, &mut previous_item_id, &mut pending_operations)?;
        self.expect_symbol('}')?;
        Ok(())
    }

    fn parse_while(&mut self, scope_block_id: &str) -> Result<String, String> {
        self.expect_keyword(Keyword::While)?;
        self.expect_symbol('(')?;
        let condition = self.parse_expression_tokens(')')?;
        self.expect_symbol(')')?;

        let (control_id, body_block_id) = self.builder.next_loop_names();
        self.builder.insert_unique_node(
            control_id.clone(),
            TTNode::control(
                control_id.clone(),
                ControlType::Loop,
                Some(scope_block_id.to_string()),
            )
            .with_operations(condition_read_operations(&condition))
            .with_branch_arc(vec![body_block_id.clone()]),
        )?;
        self.builder.insert_unique_node(
            body_block_id.clone(),
            TTNode::block(body_block_id.clone(), control_id.clone()),
        )?;
        self.parse_block(&body_block_id)?;
        Ok(control_id)
    }

    fn parse_if(&mut self, scope_block_id: &str) -> Result<String, String> {
        self.expect_keyword(Keyword::If)?;
        self.expect_symbol('(')?;
        let condition = self.parse_expression_tokens(')')?;
        self.expect_symbol(')')?;

        let (control_id, then_block_id, else_block_id) = self.builder.next_xor_names();
        self.builder.insert_unique_node(
            control_id.clone(),
            TTNode::control(
                control_id.clone(),
                ControlType::Xor,
                Some(scope_block_id.to_string()),
            )
            .with_operations(condition_read_operations(&condition))
            .with_branch_arc(vec![then_block_id.clone(), else_block_id.clone()]),
        )?;
        self.builder.insert_unique_node(
            then_block_id.clone(),
            TTNode::block(then_block_id.clone(), control_id.clone()),
        )?;
        self.parse_block(&then_block_id)?;

        self.expect_keyword(Keyword::Else)?;
        self.builder.insert_unique_node(
            else_block_id.clone(),
            TTNode::block(else_block_id.clone(), control_id.clone()),
        )?;
        self.parse_block(&else_block_id)?;
        Ok(control_id)
    }

    fn parse_simple_statement(&mut self) -> Result<Vec<Operation>, String> {
        let keyword = match self.advance() {
            Token::Keyword(keyword @ (Keyword::Print | Keyword::Kill)) => keyword,
            token => return Err(format!("expected print/kill statement, found {token:?}")),
        };
        self.expect_symbol('(')?;
        let expr = self.parse_expression_tokens(')')?;
        self.expect_symbol(')')?;
        self.expect_symbol(';')?;

        match keyword {
            Keyword::Print => Ok(condition_read_operations(&expr)),
            Keyword::Kill => {
                let variables = identifiers(&expr);
                if variables.len() != 1 {
                    return Err(format!(
                        "expected `kill(<var>)`, found `{}`",
                        tokens_to_string(&expr)
                    ));
                }
                Ok(vec![Operation::new(
                    variables[0].clone(),
                    OperationType::Kill,
                )])
            }
            _ => unreachable!(),
        }
    }

    fn parse_assignment(&mut self) -> Result<Vec<Operation>, String> {
        let target = self.expect_ident()?;
        self.expect_symbol('=')?;
        let expr = self.parse_expression_tokens(';')?;
        self.expect_symbol(';')?;

        let mut operations = condition_read_operations(&expr);
        operations.push(Operation::new(target, OperationType::Write));
        Ok(operations)
    }

    fn parse_expression_tokens(&mut self, end: char) -> Result<Vec<Token>, String> {
        let mut depth: usize = 0;
        let mut expr = Vec::new();
        loop {
            match self.peek() {
                Token::Eof => return Err("unexpected end of expression".to_string()),
                Token::Symbol('(') => {
                    depth += 1;
                    expr.push(self.advance());
                }
                Token::Symbol(')') if end == ')' && depth == 0 => break,
                Token::Symbol(')') => {
                    depth = depth
                        .checked_sub(1)
                        .ok_or_else(|| "unbalanced `)` in expression".to_string())?;
                    expr.push(self.advance());
                }
                Token::Symbol(ch) if ch == end && depth == 0 => break,
                _ => expr.push(self.advance()),
            }
        }
        Ok(expr)
    }

    fn peek_is_assignment(&self) -> bool {
        matches!(
            (
                &self.tokens.get(self.position),
                self.tokens.get(self.position + 1)
            ),
            (Some(Token::Ident(_)), Some(Token::Symbol('=')))
        )
    }

    fn peek(&self) -> Token {
        self.tokens
            .get(self.position)
            .cloned()
            .unwrap_or(Token::Eof)
    }

    fn advance(&mut self) -> Token {
        let token = self.peek();
        if !matches!(token, Token::Eof) {
            self.position += 1;
        }
        token
    }

    fn expect_keyword(&mut self, keyword: Keyword) -> Result<(), String> {
        match self.advance() {
            Token::Keyword(found) if found == keyword => Ok(()),
            token => Err(format!("expected `{keyword:?}`, found {token:?}")),
        }
    }

    fn expect_ident(&mut self) -> Result<String, String> {
        match self.advance() {
            Token::Ident(ident) => Ok(ident),
            token => Err(format!("expected identifier, found {token:?}")),
        }
    }

    fn expect_symbol(&mut self, expected: char) -> Result<(), String> {
        match self.advance() {
            Token::Symbol(ch) if ch == expected => Ok(()),
            token => Err(format!("expected `{expected}`, found {token:?}")),
        }
    }
}

fn tokenize(source: &str) -> Result<Vec<Token>, String> {
    let mut tokens = Vec::new();
    let mut chars = source.chars().peekable();

    while let Some(ch) = chars.peek().copied() {
        match ch {
            ' ' | '\t' | '\r' | '\n' => {
                chars.next();
            }
            '/' if chars.clone().nth(1) == Some('/') => {
                chars.next();
                chars.next();
                while matches!(chars.peek(), Some(c) if *c != '\n') {
                    chars.next();
                }
            }
            '{' | '}' | '(' | ')' | ';' | '=' | '<' | '>' | '+' | '-' | '*' | '/' | '%' => {
                tokens.push(Token::Symbol(ch));
                chars.next();
            }
            ch if ch.is_ascii_digit() => {
                let mut number = String::new();
                while matches!(chars.peek(), Some(c) if c.is_ascii_digit()) {
                    number.push(chars.next().expect("digit"));
                }
                tokens.push(Token::Number(number));
            }
            ch if ch.is_ascii_alphabetic() || ch == '_' => {
                let mut ident = String::new();
                while matches!(chars.peek(), Some(c) if c.is_ascii_alphanumeric() || *c == '_') {
                    ident.push(chars.next().expect("ident"));
                }
                tokens.push(match ident.as_str() {
                    "parallel" => Token::Keyword(Keyword::Parallel),
                    "branch" => Token::Keyword(Keyword::Branch),
                    "while" => Token::Keyword(Keyword::While),
                    "if" => Token::Keyword(Keyword::If),
                    "else" => Token::Keyword(Keyword::Else),
                    "print" => Token::Keyword(Keyword::Print),
                    "kill" => Token::Keyword(Keyword::Kill),
                    _ => Token::Ident(ident),
                });
            }
            ch => return Err(format!("unexpected character `{ch}`")),
        }
    }

    tokens.push(Token::Eof);
    Ok(tokens)
}

fn condition_read_operations(tokens: &[Token]) -> Vec<Operation> {
    identifiers(tokens)
        .into_iter()
        .map(|variable| Operation::new(variable, OperationType::Read))
        .collect()
}

fn identifiers(tokens: &[Token]) -> Vec<String> {
    let mut result = Vec::new();
    for token in tokens {
        if let Token::Ident(ident) = token
            && is_identifier(ident)
            && !result.contains(ident)
        {
            result.push(ident.clone());
        }
    }
    result
}

fn is_identifier(token: &str) -> bool {
    let mut chars = token.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    (first.is_ascii_alphabetic() || first == '_')
        && chars.all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
}

fn tokens_to_string(tokens: &[Token]) -> String {
    tokens
        .iter()
        .map(|token| match token {
            Token::Ident(ident) => ident.clone(),
            Token::Number(number) => number.clone(),
            Token::Symbol(ch) => ch.to_string(),
            Token::Keyword(keyword) => format!("{keyword:?}"),
            Token::Eof => String::new(),
        })
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::*;
    use crate::{OperationType, build_paper_example_graph};

    const PAPER_C: &str = include_str!("../examples/program1.c");

    #[test]
    fn parses_program1_c_into_matching_d_opn_sets() {
        let parsed = parse_c_program(PAPER_C).expect("C program parses");
        let hardcoded = build_paper_example_graph();

        for block_id in ["B1", "B2", "B3", "B4", "B5"] {
            assert_eq!(
                parsed.nodes[block_id].d_opn_set,
                hardcoded.nodes[block_id].d_opn_set
            );
        }
    }

    #[test]
    fn parsed_program1_c_reproduces_program_2_insertion() {
        let mut parsed = parse_c_program(PAPER_C).expect("C program parses");
        let result = parsed.insert_operation("Act2", "v", OperationType::Write);

        assert!(result.matches_direct_scan());
        assert_eq!(result.summary_entries.len(), 7);
        assert_eq!(
            parsed.nodes["B1"].d_opn_set[&("v".to_string(), OperationType::Write)],
            HashSet::from(["Act1".to_string(), "Act2".to_string()])
        );
    }

    #[test]
    fn parses_nested_parallel_inside_branch() {
        let mut parsed = parse_c_program(
            r#"
            parallel AndRoot {
              branch B_left {
                parallel AndInner {
                  branch B_then {
                    print(x);
                  }
                  branch B_else {
                    kill(x);
                  }
                }
              }
              branch B_right {
                print(x);
              }
            }
            "#,
        )
        .expect("nested parallel parses");

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
    }

    #[test]
    fn rejects_duplicate_parallel_ids() {
        let error = parse_c_program(
            r#"
            parallel And1 {
              branch B1 {
                parallel And1 {
                  branch B2 {
                    print(x);
                  }
                }
              }
            }
            "#,
        )
        .expect_err("duplicate control id is rejected");

        assert!(error.contains("duplicate node id `And1`"));
    }
}
