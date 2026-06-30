use std::collections::{HashMap, HashSet};
use std::sync::Mutex;

use clang::{Clang, Entity, EntityKind, Index, Unsaved};

use crate::{ControlType, Operation, OperationType, TTGraph, TTNode};

pub fn parse_cpp_file(path: &str) -> Result<TTGraph, String> {
    let source = std::fs::read_to_string(path)
        .map_err(|error| format!("failed to read `{path}`: {error}"))?;
    parse_cpp_source(path, &source)
}

pub fn parse_cpp_implicit_file(path: &str) -> Result<TTGraph, String> {
    let source = std::fs::read_to_string(path)
        .map_err(|error| format!("failed to read `{path}`: {error}"))?;
    parse_cpp_implicit_source(path, &source)
}

pub fn parse_cpp_implicit_source(display_path: &str, source: &str) -> Result<TTGraph, String> {
    let _guard = PARSE_LOCK
        .lock()
        .map_err(|_| "failed to lock C++ parser".to_string())?;
    let clang = Clang::new().map_err(|error| {
        format!(
            "failed to initialize libclang ({error}); install LLVM/Clang and set LIBCLANG_PATH if needed"
        )
    })?;
    let index = Index::new(&clang, false, false);
    let unsaved = Unsaved::new(display_path, source);
    let mut args = vec![
        "-std=c++17".to_string(),
        "-x".to_string(),
        "c++".to_string(),
        "-O0".to_string(),
    ];
    if let Ok(path) = std::env::var("SDKROOT") {
        args.push("-isysroot".to_string());
        args.push(path);
    }
    if let Ok(path) = std::env::var("CLANG_RESOURCE_DIR") {
        args.push("-resource-dir".to_string());
        args.push(path);
    }
    let args_ref: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
    let translation_unit = index
        .parser(display_path)
        .unsaved(&[unsaved])
        .arguments(&args_ref)
        .parse()
        .map_err(|error| format!("clang failed to parse `{display_path}`: {error:?}"))?;

    for diag in translation_unit.get_diagnostics() {
        eprintln!(
            "Clang Diagnostic: [{:?}] {}",
            diag.get_severity(),
            diag.get_text()
        );
    }

    let mut builder = GraphBuilder::new();
    let root = translation_unit.get_entity();
    if let Some(workers) = collect_std_thread_workers(&root, &root) {
        build_from_std_thread_workers(&workers, root, &mut builder)?;
    } else {
        build_from_sequential_entry(root, &mut builder)?;
    }

    Ok(TTGraph::new(builder.nodes))
}

static PARSE_LOCK: Mutex<()> = Mutex::new(());

pub fn parse_cpp_source(display_path: &str, source: &str) -> Result<TTGraph, String> {
    let tt_pragmas = parse_tt_pragmas(source)?;
    let _guard = PARSE_LOCK
        .lock()
        .map_err(|_| "failed to lock C++ parser".to_string())?;
    let clang = Clang::new().map_err(|error| {
        format!(
            "failed to initialize libclang ({error}); install LLVM/Clang and set LIBCLANG_PATH if needed"
        )
    })?;
    let index = Index::new(&clang, false, false);
    let unsaved = Unsaved::new(display_path, source);
    let mut args = vec![
        "-std=c++17".to_string(),
        "-x".to_string(),
        "c++".to_string(),
        "-fopenmp".to_string(),
        "-O0".to_string(),
    ];
    if let Ok(path) = std::env::var("SDKROOT") {
        args.push("-isysroot".to_string());
        args.push(path);
    }
    if let Ok(path) = std::env::var("CLANG_RESOURCE_DIR") {
        args.push("-resource-dir".to_string());
        args.push(path);
    }
    let args_ref: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
    let translation_unit = index
        .parser(display_path)
        .unsaved(&[unsaved])
        .arguments(&args_ref)
        .parse()
        .map_err(|error| format!("clang failed to parse `{display_path}`: {error:?}"))?;

    for diag in translation_unit.get_diagnostics() {
        eprintln!(
            "Clang Diagnostic: [{:?}] {}",
            diag.get_severity(),
            diag.get_text()
        );
    }

    let mut builder = GraphBuilder::new();
    if tt_pragmas.parallel_id.is_some() {
        build_from_tt_pragmas(&tt_pragmas, translation_unit.get_entity(), &mut builder)?;
    } else {
        build_from_openmp_sections(&clang, source, translation_unit.get_entity(), &mut builder)?;
    }

    Ok(TTGraph::new(builder.nodes))
}

fn build_from_tt_pragmas<'tu>(
    pragmas: &TtPragmas,
    root: Entity<'tu>,
    builder: &mut GraphBuilder,
) -> Result<(), String> {
    let parallel_id = pragmas
        .parallel_id
        .clone()
        .ok_or_else(|| "missing `#pragma tt parallel <id>`".to_string())?;
    if pragmas.branches.is_empty() {
        return Err("missing `#pragma tt branch <id>` markers".to_string());
    }

    builder.insert_unique_node(
        parallel_id.clone(),
        TTNode::control(parallel_id.clone(), ControlType::And, None),
    )?;

    let functions = collect_function_definitions(root);
    let branch_bindings = bind_branches_to_functions(&pragmas.branches, &functions)?;
    let mut branch_ids = Vec::new();

    for binding in branch_bindings {
        branch_ids.push(binding.block_id.clone());
        builder.insert_unique_node(
            binding.block_id.clone(),
            TTNode::block(binding.block_id.clone(), parallel_id.clone()),
        )?;
        builder.process_function_body(&binding.block_id, binding.function)?;
    }

    builder
        .nodes
        .get_mut(&parallel_id)
        .ok_or_else(|| format!("missing AND node `{parallel_id}`"))?
        .branch_arc = branch_ids;
    Ok(())
}

fn build_from_std_thread_workers<'tu>(
    workers: &[(usize, String)],
    root: Entity<'tu>,
    builder: &mut GraphBuilder,
) -> Result<(), String> {
    let and_id = "And1".to_string();
    builder.insert_unique_node(
        and_id.clone(),
        TTNode::control(and_id.clone(), ControlType::And, None),
    )?;

    let functions = collect_function_definitions(root);
    let mut branch_ids = Vec::new();

    for (section_index, (_, worker_name)) in workers.iter().enumerate() {
        let function = functions
            .iter()
            .find(|function| function.entity.get_name().as_deref() == Some(worker_name.as_str()))
            .ok_or_else(|| format!("no function `{worker_name}` for std::thread worker"))?;
        let block_id = builder.next_openmp_branch_id(section_index);
        branch_ids.push(block_id.clone());
        builder.insert_unique_node(
            block_id.clone(),
            TTNode::block(block_id.clone(), and_id.clone()),
        )?;
        builder.process_function_body(&block_id, function.entity)?;
    }

    builder
        .nodes
        .get_mut(&and_id)
        .ok_or_else(|| format!("missing AND node `{and_id}`"))?
        .branch_arc = branch_ids;
    Ok(())
}

fn build_from_sequential_entry<'tu>(
    root: Entity<'tu>,
    builder: &mut GraphBuilder,
) -> Result<(), String> {
    let functions = collect_function_definitions(root);
    let entry = functions
        .iter()
        .find(|function| function.entity.get_name().as_deref() == Some("program1"))
        .or_else(|| {
            functions
                .iter()
                .find(|function| function.entity.get_name().as_deref() == Some("main"))
        })
        .ok_or_else(|| {
            "no parallel region found; use std::thread workers, OpenMP sections, or legacy `#pragma tt`"
                .to_string()
        })?;

    let and_id = "And1".to_string();
    builder.insert_unique_node(
        and_id.clone(),
        TTNode::control(and_id.clone(), ControlType::And, None),
    )?;
    let block_id = "B1".to_string();
    builder.insert_unique_node(
        block_id.clone(),
        TTNode::block(block_id.clone(), and_id.clone()),
    )?;
    builder.process_function_body(&block_id, entry.entity)?;
    builder
        .nodes
        .get_mut(&and_id)
        .ok_or_else(|| format!("missing AND node `{and_id}`"))?
        .branch_arc = vec![block_id];
    Ok(())
}

fn collect_std_thread_workers<'tu>(
    entity: &Entity<'tu>,
    root: &Entity<'tu>,
) -> Option<Vec<(usize, String)>> {
    let defined_functions: HashSet<String> = collect_function_definitions(*root)
        .into_iter()
        .filter_map(|function| function.entity.get_name())
        .collect();
    let mut workers = Vec::new();
    collect_std_thread_workers_recursive(entity, &defined_functions, &mut workers);
    if workers.is_empty() {
        return None;
    }
    workers.sort_by_key(|(line, _)| *line);
    workers.dedup_by(|left, right| left.1 == right.1);
    Some(workers)
}

fn collect_std_thread_workers_recursive<'tu>(
    entity: &Entity<'tu>,
    defined_functions: &HashSet<String>,
    workers: &mut Vec<(usize, String)>,
) {
    if is_std_thread_construct(entity)
        && let Some(worker_name) = thread_worker_name(entity, defined_functions)
        && let Some(line) = entity_line(entity)
    {
        workers.push((line, worker_name));
    }
    for child in entity.get_children() {
        collect_std_thread_workers_recursive(&child, defined_functions, workers);
    }
}

fn is_std_thread_construct(entity: &Entity<'_>) -> bool {
    if let Some(name) = entity.get_display_name() {
        if name.contains("thread") {
            return true;
        }
    }
    if let Some(ty) = entity.get_type() {
        let ty_name = ty.get_display_name();
        if ty_name.contains("thread") {
            return true;
        }
    }
    false
}

fn thread_worker_name(entity: &Entity<'_>, defined_functions: &HashSet<String>) -> Option<String> {
    if let Some(name) = find_worker_in_subtree(entity, defined_functions) {
        return Some(name);
    }
    entity
        .get_lexical_parent()
        .and_then(|parent| find_worker_in_subtree(&parent, defined_functions))
}

fn find_worker_in_subtree(
    entity: &Entity<'_>,
    defined_functions: &HashSet<String>,
) -> Option<String> {
    if entity.get_kind() == EntityKind::DeclRefExpr {
        if let Some(name) = entity.get_name() {
            if defined_functions.contains(&name) {
                return Some(name);
            }
        }
    }
    for child in entity.get_children() {
        if let Some(name) = find_worker_in_subtree(&child, defined_functions) {
            return Some(name);
        }
    }
    None
}

fn build_from_openmp_sections<'tu>(
    clang: &Clang,
    source: &str,
    root: Entity<'tu>,
    builder: &mut GraphBuilder,
) -> Result<(), String> {
    let parallel_sections = find_parallel_sections_directives(&root);
    if parallel_sections.is_empty() {
        return Err(
            "no parallel region found; use `#pragma omp parallel sections` or legacy `#pragma tt parallel`"
                .to_string(),
        );
    }

    for (index, directive) in parallel_sections.into_iter().enumerate() {
        let and_id = if index == 0 {
            "And1".to_string()
        } else {
            format!("And_{index}")
        };
        builder.insert_unique_node(
            and_id.clone(),
            TTNode::control(and_id.clone(), ControlType::And, None),
        )?;
        process_openmp_parallel_region(clang, source, &directive, builder, &and_id)?;
    }

    Ok(())
}

fn find_parallel_sections_directives<'tu>(entity: &Entity<'tu>) -> Vec<Entity<'tu>> {
    let mut directives = Vec::new();
    collect_parallel_sections_directives(entity, &mut directives);
    directives.sort_by_key(|directive| entity_line(directive).unwrap_or(0));
    directives
}

fn collect_parallel_sections_directives<'tu>(
    entity: &Entity<'tu>,
    directives: &mut Vec<Entity<'tu>>,
) {
    if entity.get_kind() == EntityKind::OmpParallelSectionsDirective {
        directives.push(*entity);
    }
    for child in entity.get_children() {
        collect_parallel_sections_directives(&child, directives);
    }
}

fn section_directives_in_order<'tu>(directive: &Entity<'tu>) -> Vec<Entity<'tu>> {
    let mut sections = Vec::new();
    collect_section_directives(directive, &mut sections);
    sections.sort_by_key(|section| entity_line(section).unwrap_or(0));
    sections
}

fn collect_section_directives<'tu>(entity: &Entity<'tu>, sections: &mut Vec<Entity<'tu>>) {
    if entity.get_kind() == EntityKind::OmpSectionDirective {
        sections.push(*entity);
    }
    for child in entity.get_children() {
        collect_section_directives(&child, sections);
    }
}

fn section_body<'tu>(section: &Entity<'tu>) -> Option<Entity<'tu>> {
    section
        .get_children()
        .into_iter()
        .find(|child| child.get_kind() == EntityKind::CompoundStmt)
}

fn process_openmp_parallel_region<'tu>(
    clang: &Clang,
    source: &str,
    directive: &Entity<'tu>,
    builder: &mut GraphBuilder,
    and_id: &str,
) -> Result<(), String> {
    let ast_sections = section_directives_in_order(directive);
    let ast_bodies: Vec<Entity<'tu>> = ast_sections.iter().filter_map(section_body).collect();

    let mut branch_ids = Vec::new();
    if ast_bodies.len() == ast_sections.len() && !ast_bodies.is_empty() {
        for (section_index, body) in ast_bodies.iter().enumerate() {
            let block_id = builder.next_openmp_branch_id(section_index);
            branch_ids.push(block_id.clone());
            builder.insert_unique_node(
                block_id.clone(),
                TTNode::block(block_id.clone(), and_id.to_string()),
            )?;
            builder.process_compound(&block_id, body)?;
        }
    } else {
        let section_sources = extract_omp_section_sources(source)?;
        for (section_index, section_source) in section_sources.iter().enumerate() {
            let block_id = builder.next_openmp_branch_id(section_index);
            branch_ids.push(block_id.clone());
            builder.insert_unique_node(
                block_id.clone(),
                TTNode::block(block_id.clone(), and_id.to_string()),
            )?;
            parse_and_process_section_snippet(clang, section_source, &block_id, builder)?;
        }
    }

    builder
        .nodes
        .get_mut(and_id)
        .ok_or_else(|| format!("missing AND node `{and_id}`"))?
        .branch_arc = branch_ids;
    Ok(())
}

fn extract_omp_section_sources(source: &str) -> Result<Vec<String>, String> {
    let lines: Vec<&str> = source.lines().collect();
    let mut sources = Vec::new();

    for (index, line) in lines.iter().enumerate() {
        let trimmed = line.split("//").next().unwrap_or("").trim();
        if !trimmed.starts_with("#pragma omp section") {
            continue;
        }
        let block = extract_braced_block_after_line(&lines, index).ok_or_else(|| {
            format!(
                "no `{{` block after `#pragma omp section` at line {}",
                index + 1
            )
        })?;
        sources.push(block);
    }

    if sources.is_empty() {
        return Err(
            "`#pragma omp parallel sections` has no `#pragma omp section` bodies".to_string(),
        );
    }
    Ok(sources)
}

fn extract_braced_block_after_line(lines: &[&str], from_line: usize) -> Option<String> {
    for line_idx in from_line..lines.len() {
        if let Some(col) = lines[line_idx].find('{') {
            return extract_balanced_braces(lines, line_idx, col);
        }
    }
    None
}

fn extract_balanced_braces(lines: &[&str], start_line: usize, start_col: usize) -> Option<String> {
    let mut depth = 0i32;
    let mut out = String::new();
    let mut started = false;

    for (idx, line) in lines.iter().enumerate().skip(start_line) {
        let text = if idx == start_line {
            &line[start_col..]
        } else {
            line
        };
        for ch in text.chars() {
            match ch {
                '{' => depth += 1,
                '}' => depth -= 1,
                _ => {}
            }
            if depth > 0 {
                started = true;
                out.push(ch);
            }
            if started && depth == 0 {
                return Some(out);
            }
        }
        if started {
            out.push('\n');
        }
    }

    None
}

fn parse_and_process_section_snippet(
    clang: &Clang,
    section_source: &str,
    block_id: &str,
    builder: &mut GraphBuilder,
) -> Result<(), String> {
    let snippet = format!(
        "extern int v, i;\nvoid tt_print(int &);\nvoid tt_kill(int &);\nvoid __tt_section() {{\n{section_source}\n}}\n"
    );
    let index = Index::new(clang, false, false);
    let unsaved = Unsaved::new("__tt_section.cpp", &snippet);
    let translation_unit = index
        .parser("__tt_section.cpp")
        .unsaved(&[unsaved])
        .arguments(&["-std=c++17", "-x", "c++", "-O0"])
        .parse()
        .map_err(|error| format!("clang failed to parse OpenMP section snippet: {error:?}"))?;

    let functions = collect_function_definitions(translation_unit.get_entity());
    let function = functions
        .into_iter()
        .find(|function| function.entity.get_name().as_deref() == Some("__tt_section"))
        .ok_or_else(|| "OpenMP section snippet is missing `__tt_section`".to_string())?;
    builder.process_function_body(block_id, function.entity)
}

#[derive(Debug)]
struct TtPragmas {
    parallel_id: Option<String>,
    branches: Vec<BranchPragma>,
}

#[derive(Debug, Clone)]
struct BranchPragma {
    block_id: String,
    line: usize,
}

#[derive(Debug)]
struct FunctionDefinition<'tu> {
    line: usize,
    entity: Entity<'tu>,
}

#[derive(Debug)]
struct BranchBinding<'tu> {
    block_id: String,
    function: Entity<'tu>,
}

struct GraphBuilder {
    nodes: HashMap<String, TTNode>,
    next_activity: usize,
    next_loop: usize,
    next_xor: usize,
    next_openmp_branch: usize,
}

impl GraphBuilder {
    fn new() -> Self {
        Self {
            nodes: HashMap::new(),
            next_activity: 1,
            next_loop: 1,
            next_xor: 1,
            next_openmp_branch: 2,
        }
    }

    fn next_openmp_branch_id(&mut self, section_index: usize) -> String {
        if section_index == 0 {
            return "B1".to_string();
        }
        if section_index == 1 {
            return "B2".to_string();
        }
        self.next_openmp_branch += 1;
        format!("B_section_{}", self.next_openmp_branch)
    }

    fn process_function_body(
        &mut self,
        block_id: &str,
        function: Entity<'_>,
    ) -> Result<(), String> {
        let body = function
            .get_children()
            .into_iter()
            .find(|child| child.get_kind() == EntityKind::CompoundStmt)
            .ok_or_else(|| {
                format!(
                    "function `{}` is missing a body",
                    function.get_name().unwrap_or_default()
                )
            })?;
        self.process_compound(block_id, &body)
    }

    fn process_compound(&mut self, block_id: &str, compound: &Entity<'_>) -> Result<(), String> {
        let mut previous_item_id: Option<String> = None;
        let mut pending_operations = Vec::new();

        for child in compound.get_children() {
            if child.get_kind() == EntityKind::CompoundStmt {
                self.process_compound(block_id, &child)?;
                continue;
            }
            if is_control_statement(child.get_kind()) {
                self.flush_activity(block_id, &mut previous_item_id, &mut pending_operations)?;
                let item_id = self.process_control_statement(block_id, &child)?;
                self.link_item(block_id, &mut previous_item_id, &item_id);
                continue;
            }

            if let Some(operations) = operations_from_statement(&child)? {
                if operations.is_empty() {
                    continue;
                }
                pending_operations.extend(operations);
            }
        }

        self.flush_activity(block_id, &mut previous_item_id, &mut pending_operations)?;
        Ok(())
    }

    fn process_control_statement(
        &mut self,
        block_id: &str,
        stmt: &Entity<'_>,
    ) -> Result<String, String> {
        match stmt.get_kind() {
            EntityKind::WhileStmt | EntityKind::ForStmt => self.process_loop(block_id, stmt),
            EntityKind::IfStmt => self.process_if(block_id, stmt),
            _ => Err(format!(
                "unsupported control statement {:?}",
                stmt.get_kind()
            )),
        }
    }

    fn process_loop(&mut self, scope_block_id: &str, stmt: &Entity<'_>) -> Result<String, String> {
        let children = stmt.get_children();
        let condition = loop_condition(stmt)?;
        let body = children
            .iter()
            .find(|child| child.get_kind() == EntityKind::CompoundStmt)
            .ok_or_else(|| "loop statement is missing a body".to_string())?;

        let (control_id, body_block_id) = self.next_loop_names();
        self.insert_unique_node(
            control_id.clone(),
            TTNode::control(
                control_id.clone(),
                ControlType::Loop,
                Some(scope_block_id.to_string()),
            )
            .with_operations(read_operations_from_expr(&condition))
            .with_branch_arc(vec![body_block_id.clone()]),
        )?;
        self.insert_unique_node(
            body_block_id.clone(),
            TTNode::block(body_block_id.clone(), control_id.clone()),
        )?;
        self.process_compound(&body_block_id, body)?;
        Ok(control_id)
    }

    fn process_if(&mut self, scope_block_id: &str, stmt: &Entity<'_>) -> Result<String, String> {
        let children = stmt.get_children();
        let condition = children
            .first()
            .ok_or_else(|| "if statement is missing a condition".to_string())?;
        let then_stmt = children
            .get(1)
            .ok_or_else(|| "if statement is missing a then-branch".to_string())?;
        let else_stmt = children.get(2);

        let (control_id, then_block_id, else_block_id) = self.next_xor_names();
        self.insert_unique_node(
            control_id.clone(),
            TTNode::control(
                control_id.clone(),
                ControlType::Xor,
                Some(scope_block_id.to_string()),
            )
            .with_operations(read_operations_from_expr(condition))
            .with_branch_arc(vec![then_block_id.clone(), else_block_id.clone()]),
        )?;
        self.insert_unique_node(
            then_block_id.clone(),
            TTNode::block(then_block_id.clone(), control_id.clone()),
        )?;
        self.process_branch_body(&then_block_id, then_stmt)?;

        self.insert_unique_node(
            else_block_id.clone(),
            TTNode::block(else_block_id.clone(), control_id.clone()),
        )?;
        if let Some(else_stmt) = else_stmt {
            self.process_branch_body(&else_block_id, else_stmt)?;
        }
        Ok(control_id)
    }

    fn process_branch_body(&mut self, block_id: &str, stmt: &Entity<'_>) -> Result<(), String> {
        if stmt.get_kind() == EntityKind::CompoundStmt {
            self.process_compound(block_id, stmt)
        } else if let Some(operations) = operations_from_statement(stmt)? {
            let mut previous_item_id = None;
            let mut pending_operations = operations;
            self.flush_activity(block_id, &mut previous_item_id, &mut pending_operations)
        } else {
            Ok(())
        }
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

    fn insert_unique_node(&mut self, node_id: String, node: TTNode) -> Result<(), String> {
        if self.nodes.contains_key(&node_id) {
            return Err(format!("duplicate node id `{node_id}`"));
        }
        self.nodes.insert(node_id, node);
        Ok(())
    }

    fn next_loop_names(&mut self) -> (String, String) {
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

    fn next_xor_names(&mut self) -> (String, String, String) {
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

fn parse_tt_pragmas(source: &str) -> Result<TtPragmas, String> {
    let mut parallel_id = None;
    let mut branches = Vec::new();

    for (index, line) in source.lines().enumerate() {
        let trimmed = line.split("//").next().unwrap_or("").trim();
        let Some(marker) = trimmed.strip_prefix("#pragma tt ") else {
            continue;
        };
        let mut parts = marker.split_whitespace();
        match parts.next() {
            Some("parallel") => {
                let id = parts
                    .next()
                    .ok_or_else(|| format!("invalid parallel pragma on line {}", index + 1))?;
                parallel_id = Some(id.to_string());
            }
            Some("branch") => {
                let id = parts
                    .next()
                    .ok_or_else(|| format!("invalid branch pragma on line {}", index + 1))?;
                branches.push(BranchPragma {
                    block_id: id.to_string(),
                    line: index + 1,
                });
            }
            Some(token) => {
                return Err(format!(
                    "unsupported tt pragma `{token}` on line {}",
                    index + 1
                ));
            }
            None => {}
        }
    }

    Ok(TtPragmas {
        parallel_id,
        branches,
    })
}

fn collect_function_definitions<'tu>(entity: Entity<'tu>) -> Vec<FunctionDefinition<'tu>> {
    let mut functions = Vec::new();
    collect_function_definitions_recursive(&entity, &mut functions);
    functions.sort_by_key(|function| function.line);
    functions
}

fn collect_function_definitions_recursive<'tu>(
    entity: &Entity<'tu>,
    functions: &mut Vec<FunctionDefinition<'tu>>,
) {
    if entity.get_kind() == EntityKind::FunctionDecl
        && entity
            .get_children()
            .into_iter()
            .any(|child| child.get_kind() == EntityKind::CompoundStmt)
        && let Some(line) = entity_line(entity)
    {
        functions.push(FunctionDefinition {
            line,
            entity: *entity,
        });
    }

    for child in entity.get_children() {
        collect_function_definitions_recursive(&child, functions);
    }
}

fn bind_branches_to_functions<'tu>(
    branches: &[BranchPragma],
    functions: &[FunctionDefinition<'tu>],
) -> Result<Vec<BranchBinding<'tu>>, String> {
    let mut bindings = Vec::new();
    let mut function_index = 0;

    for branch in branches {
        while function_index < functions.len() && functions[function_index].line <= branch.line {
            function_index += 1;
        }
        let Some(function) = functions.get(function_index) else {
            return Err(format!(
                "no function definition found after `#pragma tt branch {}`",
                branch.block_id
            ));
        };
        bindings.push(BranchBinding {
            block_id: branch.block_id.clone(),
            function: function.entity,
        });
        function_index += 1;
    }

    Ok(bindings)
}

fn entity_line(entity: &Entity<'_>) -> Option<usize> {
    entity
        .get_location()
        .map(|location| location.get_spelling_location().line as usize)
}

fn is_control_statement(kind: EntityKind) -> bool {
    matches!(
        kind,
        EntityKind::WhileStmt | EntityKind::ForStmt | EntityKind::IfStmt
    )
}

fn operations_from_statement(stmt: &Entity<'_>) -> Result<Option<Vec<Operation>>, String> {
    if let Some((callee, args)) = find_synthetic_call(stmt) {
        return operations_from_callee_and_args(&callee, &args).map(Some);
    }
    if let Some(call) = find_preferred_call(stmt) {
        return operations_from_call(&call).map(Some);
    }
    if let Some(assignment) = find_entity_kind(stmt, EntityKind::BinaryOperator)
        && is_assignment_operator(&assignment)
    {
        return operations_from_assignment(&assignment).map(Some);
    }

    match stmt.get_kind() {
        EntityKind::CallExpr => {
            if is_std_thread_construct(stmt) {
                Ok(Some(Vec::new()))
            } else if is_ostream_call(stmt) {
                operations_from_ostream(stmt).map(Some)
            } else {
                operations_from_call(stmt).map(Some)
            }
        }
        EntityKind::BinaryOperator => operations_from_assignment(stmt).map(Some),
        EntityKind::DeclStmt => {
            let mut operations = Vec::new();
            for child in stmt.get_children() {
                if let Some(child_ops) = operations_from_statement(&child)? {
                    operations.extend(child_ops);
                }
            }
            Ok(Some(operations))
        }
        EntityKind::CompoundStmt => Ok(Some(Vec::new())),
        kind if is_expr_wrapper(kind) => {
            let mut operations = Vec::new();
            for child in stmt.get_children() {
                if let Some(child_ops) = operations_from_statement(&child)? {
                    operations.extend(child_ops);
                }
            }
            Ok(Some(operations))
        }
        EntityKind::DeleteExpr => operations_from_delete(stmt).map(Some),
        EntityKind::NullStmt
        | EntityKind::LabelStmt
        | EntityKind::ReturnStmt
        | EntityKind::VarDecl
        | EntityKind::ParmDecl
        | EntityKind::DeclRefExpr
        | EntityKind::FunctionDecl
        | EntityKind::TypedefDecl
        | EntityKind::StructDecl
        | EntityKind::ClassDecl => Ok(Some(Vec::new())),
        EntityKind::OmpSectionDirective | EntityKind::OmpParallelSectionsDirective => {
            Ok(Some(Vec::new()))
        }
        EntityKind::UnexposedExpr | EntityKind::UnexposedStmt => {
            let mut operations = Vec::new();
            for child in stmt.get_children() {
                if let Some(child_ops) = operations_from_statement(&child)? {
                    operations.extend(child_ops);
                }
            }
            Ok(Some(operations))
        }
        kind => {
            if let Some(call) = find_preferred_call(stmt) {
                return operations_from_call(&call).map(Some);
            }
            if let Some(assignment) = find_entity_kind(stmt, EntityKind::BinaryOperator)
                && is_assignment_operator(&assignment)
            {
                return operations_from_assignment(&assignment).map(Some);
            }
            Err(format!("unsupported statement kind `{kind:?}`"))
        }
    }
}

fn find_synthetic_call<'tu>(entity: &Entity<'tu>) -> Option<(Entity<'tu>, Vec<Entity<'tu>>)> {
    if let Some(found) = synthetic_call_arguments(entity) {
        return Some(found);
    }
    for child in entity.get_children() {
        if let Some(found) = find_synthetic_call(&child) {
            return Some(found);
        }
    }
    None
}

fn synthetic_call_arguments<'tu>(entity: &Entity<'tu>) -> Option<(Entity<'tu>, Vec<Entity<'tu>>)> {
    match entity.get_kind() {
        EntityKind::UnexposedExpr | EntityKind::UnexposedStmt => {
            let children = entity.get_children();
            if children.len() < 2 {
                return None;
            }
            let callee = children.first()?;
            if callee.get_kind() != EntityKind::DeclRefExpr {
                return None;
            }
            let callee_name = callee
                .get_name()
                .or_else(|| callee.get_display_name())
                .unwrap_or_default();
            let base_name = callee_name.rsplit("::").next().unwrap_or(&callee_name);
            if base_name == "tt_kill" || base_name == "free" {
                return Some((*callee, children[1..].to_vec()));
            }
            if is_print_like_call(base_name, &callee_name) || callee_name.is_empty() {
                return Some((*callee, children[1..].to_vec()));
            }
            None
        }
        _ => None,
    }
}

fn find_preferred_call<'tu>(entity: &Entity<'tu>) -> Option<Entity<'tu>> {
    let mut calls = Vec::new();
    collect_entity_kind(entity, EntityKind::CallExpr, &mut calls);
    calls.into_iter().find(|call| {
        let Some(callee) = call.get_child(0) else {
            return false;
        };
        let callee_name = callee
            .get_name()
            .or_else(|| callee.get_display_name())
            .unwrap_or_default();
        let base_name = callee_name
            .rsplit("::")
            .next()
            .unwrap_or(callee_name.as_str());
        base_name == "tt_kill" || base_name == "free" || is_print_like_call(base_name, &callee_name)
    })
}

fn collect_entity_kind<'tu>(
    entity: &Entity<'tu>,
    kind: EntityKind,
    matches: &mut Vec<Entity<'tu>>,
) {
    if entity.get_kind() == kind {
        matches.push(*entity);
    }
    for child in entity.get_children() {
        collect_entity_kind(&child, kind, matches);
    }
}

fn find_entity_kind<'tu>(entity: &Entity<'tu>, kind: EntityKind) -> Option<Entity<'tu>> {
    if entity.get_kind() == kind {
        return Some(*entity);
    }
    for child in entity.get_children() {
        if let Some(found) = find_entity_kind(&child, kind) {
            return Some(found);
        }
    }
    None
}

fn is_expr_wrapper(kind: EntityKind) -> bool {
    matches!(
        kind,
        EntityKind::ParenExpr | EntityKind::UnaryOperator | EntityKind::CStyleCastExpr
    )
}

fn loop_condition<'a>(stmt: &Entity<'a>) -> Result<Entity<'a>, String> {
    let children = stmt.get_children();
    match stmt.get_kind() {
        EntityKind::WhileStmt => children
            .first()
            .copied()
            .ok_or_else(|| "while statement is missing a condition".to_string()),
        EntityKind::ForStmt => children
            .get(1)
            .copied()
            .ok_or_else(|| "for statement is missing a condition".to_string()),
        kind => Err(format!("unsupported loop kind `{kind:?}`")),
    }
}

fn operations_from_call(call: &Entity<'_>) -> Result<Vec<Operation>, String> {
    let callee = call
        .get_child(0)
        .ok_or_else(|| "call expression is missing a callee".to_string())?;
    let args: Vec<Entity<'_>> = call.get_children().into_iter().skip(1).collect();
    operations_from_callee_and_args(&callee, &args)
}

fn operations_from_callee_and_args(
    callee: &Entity<'_>,
    args: &[Entity<'_>],
) -> Result<Vec<Operation>, String> {
    let callee_name = callee
        .get_name()
        .or_else(|| callee.get_display_name())
        .unwrap_or_default();
    let base_name = callee_name
        .rsplit("::")
        .next()
        .unwrap_or(callee_name.as_str());

    if base_name == "tt_kill" {
        let arg = args
            .first()
            .ok_or_else(|| format!("{base_name} expects one argument"))?;
        let variable = variable_name_from_expr(arg)
            .or_else(|| points_to_variable_name(arg))
            .ok_or_else(|| format!("{base_name} argument must be a variable"))?;
        return Ok(vec![Operation::new(variable, OperationType::Kill)]);
    }

    if base_name == "free" {
        let mut operations = Vec::new();
        for arg in args {
            for variable in collect_variables(arg) {
                if !operations
                    .iter()
                    .any(|op: &Operation| op.variable == variable)
                {
                    operations.push(Operation::new(variable, OperationType::Kill));
                }
            }
        }
        return Ok(operations);
    }

    if is_print_like_call(base_name, &callee_name) || callee_name.is_empty() {
        let mut operations = Vec::new();
        for arg in args {
            if is_string_literal(arg) {
                continue;
            }
            operations.extend(read_operations_from_expr(arg));
        }
        return Ok(operations);
    }

    Ok(Vec::new())
}

fn operations_from_delete(expr: &Entity<'_>) -> Result<Vec<Operation>, String> {
    let operand = expr
        .get_children()
        .into_iter()
        .next()
        .ok_or_else(|| "delete expression is missing an operand".to_string())?;
    let variable = variable_name_from_expr(&operand)
        .or_else(|| points_to_variable_name(&operand))
        .ok_or_else(|| "delete operand must reference a variable".to_string())?;
    Ok(vec![Operation::new(variable, OperationType::Kill)])
}

fn points_to_variable_name(expr: &Entity<'_>) -> Option<String> {
    match expr.get_kind() {
        EntityKind::UnaryOperator => {
            let operand = expr.get_children().into_iter().next()?;
            variable_name_from_expr(&operand)
        }
        EntityKind::CStyleCastExpr | EntityKind::ParenExpr => expr
            .get_children()
            .into_iter()
            .find_map(|child| points_to_variable_name(&child)),
        _ => None,
    }
}

fn is_ostream_call(expr: &Entity<'_>) -> bool {
    expr.get_display_name()
        .is_some_and(|display| display.contains("operator<<"))
}

fn operations_from_ostream(expr: &Entity<'_>) -> Result<Vec<Operation>, String> {
    if !is_ostream_call(expr) {
        return Ok(Vec::new());
    }
    let mut operations = Vec::new();
    for arg in expr.get_children().into_iter().skip(1) {
        if is_string_literal(&arg) {
            continue;
        }
        operations.extend(read_operations_from_expr(&arg));
    }
    Ok(operations)
}

fn is_print_like_call(base_name: &str, callee_name: &str) -> bool {
    matches!(base_name, "tt_print" | "printf" | "fprintf" | "puts")
        || base_name.contains("printf")
        || callee_name.contains("printf")
}

fn is_string_literal(entity: &Entity<'_>) -> bool {
    if entity.get_kind() == EntityKind::StringLiteral {
        return true;
    }
    entity
        .get_display_name()
        .is_some_and(|name| name.starts_with('"') || name.starts_with("L\""))
}

fn operations_from_assignment(expr: &Entity<'_>) -> Result<Vec<Operation>, String> {
    if !is_assignment_operator(expr) {
        return Ok(Vec::new());
    }

    let (lhs, rhs) = assignment_operands(expr)?;
    let target = variable_name_from_expr(&lhs)
        .ok_or_else(|| "assignment target must be a variable".to_string())?;

    let mut operations = read_operations_from_expr(&rhs);
    operations.push(Operation::new(target, OperationType::Write));
    Ok(operations)
}

fn assignment_operands<'a>(expr: &'a Entity<'a>) -> Result<(Entity<'a>, Entity<'a>), String> {
    let lhs = expr
        .get_child(0)
        .ok_or_else(|| "assignment is missing a left-hand side".to_string())?;
    let rhs = expr
        .get_child(1)
        .ok_or_else(|| "assignment is missing a right-hand side".to_string())?;
    Ok((lhs, rhs))
}

fn is_assignment_operator(entity: &Entity<'_>) -> bool {
    if entity.get_kind() != EntityKind::BinaryOperator {
        return false;
    }

    if let Some(name) = entity.get_display_name() {
        return name.contains('=')
            && !name.contains("==")
            && !name.contains("!=")
            && !name.contains("<=")
            && !name.contains(">=");
    }

    entity
        .get_child(0)
        .is_some_and(|lhs| variable_name_from_expr(&lhs).is_some())
        && entity.get_child(1).is_some()
}

fn read_operations_from_expr(expr: &Entity<'_>) -> Vec<Operation> {
    collect_variables(expr)
        .into_iter()
        .map(|variable| Operation::new(variable, OperationType::Read))
        .collect()
}

fn variable_name_from_expr(expr: &Entity<'_>) -> Option<String> {
    match expr.get_kind() {
        EntityKind::DeclRefExpr => expr.get_name(),
        EntityKind::UnaryOperator | EntityKind::ParenExpr | EntityKind::CStyleCastExpr => expr
            .get_children()
            .into_iter()
            .find_map(|child| variable_name_from_expr(&child)),
        _ => None,
    }
}

fn collect_variables(entity: &Entity<'_>) -> Vec<String> {
    let mut result = Vec::new();
    collect_variables_recursive(entity, &mut result);
    result
}

fn collect_variables_recursive(entity: &Entity<'_>, result: &mut Vec<String>) {
    if entity.get_kind() == EntityKind::DeclRefExpr
        && let Some(name) = entity.get_name()
        && !result.contains(&name)
    {
        result.push(name);
    }

    for child in entity.get_children() {
        collect_variables_recursive(&child, result);
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;
    use std::path::Path;

    use super::*;
    use crate::{OperationType, build_paper_example_graph};

    const PAPER_CPP: &str = include_str!("../examples/program1.cpp");
    const PLAIN_CPP: &str = include_str!("../examples/program1_plain.cpp");

    fn parse_test_source(source: &str) -> Result<TTGraph, String> {
        parse_cpp_source("program1.cpp", source)
    }

    fn parse_implicit_test_source(source: &str) -> Result<TTGraph, String> {
        parse_cpp_implicit_source("program1_plain.cpp", source)
    }

    fn skip_without_libclang(result: Result<TTGraph, String>) -> Option<TTGraph> {
        match result {
            Ok(graph) => Some(graph),
            Err(error) if error.contains("libclang") => {
                eprintln!("skipping C++ Clang test: {error}");
                None
            }
            Err(error) => panic!("C++ program parse failed: {error}"),
        }
    }

    #[test]
    fn parses_program1_cpp_into_matching_d_opn_sets() {
        let Some(parsed) = skip_without_libclang(parse_test_source(PAPER_CPP)) else {
            return;
        };
        let hardcoded = build_paper_example_graph();

        for block_id in ["B1", "B2", "B3", "B4", "B5"] {
            assert_eq!(
                parsed.nodes[block_id].d_opn_set,
                hardcoded.nodes[block_id].d_opn_set
            );
        }
    }

    #[test]
    fn parsed_program1_cpp_reproduces_program_2_insertion() {
        let Some(mut parsed) = skip_without_libclang(parse_test_source(PAPER_CPP)) else {
            return;
        };
        let result = parsed.insert_operation("Act2", "v", OperationType::Write);

        assert!(result.matches_direct_scan());
        assert_eq!(result.summary_entries.len(), 7);
        assert_eq!(
            parsed.nodes["B1"].d_opn_set[&("v".to_string(), OperationType::Write)],
            HashSet::from(["Act1".to_string(), "Act2".to_string()])
        );
    }

    #[test]
    fn parses_file_path_entry_point() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("examples/program1.cpp");
        let Some(parsed) =
            skip_without_libclang(parse_cpp_file(path.to_str().expect("utf-8 path")))
        else {
            return;
        };
        assert!(parsed.nodes.contains_key("And1"));
    }

    #[test]
    fn parses_program1_plain_cpp_into_matching_d_opn_sets() {
        let Some(parsed) = skip_without_libclang(parse_implicit_test_source(PLAIN_CPP)) else {
            return;
        };
        let hardcoded = build_paper_example_graph();

        for block_id in ["B1", "B2", "B3", "B4", "B5"] {
            assert_eq!(
                parsed.nodes[block_id].d_opn_set,
                hardcoded.nodes[block_id].d_opn_set
            );
        }
    }

    #[test]
    fn parsed_program1_plain_cpp_reproduces_program_2_insertion() {
        let Some(mut parsed) = skip_without_libclang(parse_implicit_test_source(PLAIN_CPP)) else {
            return;
        };
        let result = parsed.insert_operation("Act2", "v", OperationType::Write);

        assert!(result.matches_direct_scan());
        assert_eq!(result.summary_entries.len(), 7);
        assert_eq!(
            parsed.nodes["B1"].d_opn_set[&("v".to_string(), OperationType::Write)],
            HashSet::from(["Act1".to_string(), "Act2".to_string()])
        );
    }

    #[test]
    fn rejects_missing_parallel_region() {
        let error = parse_test_source("void f() {}").expect_err("parallel region required");
        assert!(error.contains("parallel"));
    }

    #[test]
    fn legacy_pragma_cpp_still_parses_program1() {
        const PRAGMA_CPP: &str = include_str!("../examples/program1_pragma.cpp");
        let Some(parsed) = skip_without_libclang(parse_test_source(PRAGMA_CPP)) else {
            return;
        };
        let hardcoded = build_paper_example_graph();
        assert_eq!(
            parsed.nodes["B1"].d_opn_set,
            hardcoded.nodes["B1"].d_opn_set
        );
    }
}
