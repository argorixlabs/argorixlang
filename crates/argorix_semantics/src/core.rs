//! Semantic checker for the transitional Argorix Core 0.1 frontend.
//!
//! This Rust implementation is stage0 only. It defines behavior that the
//! self-hosted `.argx` frontend must reproduce before Rust can be retired.

use argorix_parser::core::*;
use argorix_parser::span::Span;
use std::collections::{BTreeSet, HashMap, HashSet};

#[derive(Debug, Clone, Default)]
pub struct CoreCheckOptions {
    pub available_modules: BTreeSet<String>,
}

/// Proof object produced only after the complete Core semantic checker passes.
///
/// Stage0 lowering accepts this wrapper instead of a raw AST so future Core
/// backends cannot accidentally consume unchecked source.
#[derive(Debug, Clone, Copy)]
pub struct VerifiedCoreProgram<'a> {
    program: &'a CoreProgram,
}

impl<'a> VerifiedCoreProgram<'a> {
    pub fn program(self) -> &'a CoreProgram {
        self.program
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Ty {
    Unit,
    Bool,
    Int(String),
    Bytes,
    String,
    Array(Box<Ty>, u64),
    Slice(Box<Ty>),
    Buffer(Box<Ty>),
    Arena(Box<Ty>),
    Handle(Box<Ty>),
    Named(String),
    Unknown,
    Never,
}

impl Ty {
    fn display(&self) -> String {
        match self {
            Self::Unit => "unit".into(),
            Self::Bool => "bool".into(),
            Self::Int(name) | Self::Named(name) => name.clone(),
            Self::Bytes => "bytes".into(),
            Self::String => "string".into(),
            Self::Array(element, length) => format!("Array<{}, {length}>", element.display()),
            Self::Slice(element) => format!("Slice<{}>", element.display()),
            Self::Buffer(element) => format!("Buffer<{}>", element.display()),
            Self::Arena(element) => format!("Arena<{}>", element.display()),
            Self::Handle(element) => format!("Handle<{}>", element.display()),
            Self::Unknown => "<unknown>".into(),
            Self::Never => "!".into(),
        }
    }

    fn is_integer(&self) -> bool {
        matches!(self, Self::Int(_))
    }
}

#[derive(Debug, Clone)]
struct StructInfo {
    fields: HashMap<String, Ty>,
    span: Span,
}

#[derive(Debug, Clone)]
struct VariantInfo {
    fields: HashMap<String, Ty>,
}

#[derive(Debug, Clone)]
struct EnumInfo {
    variants: HashMap<String, VariantInfo>,
}

#[derive(Debug, Clone)]
struct FunctionInfo {
    parameters: Vec<Ty>,
    result: Ty,
}

#[derive(Debug, Clone)]
struct Binding {
    ty: Ty,
    mutable: bool,
    /// Unique within a function, assigned by `define`, so a moved binding is
    /// told apart from a later one that shadows it.
    id: usize,
}

/// What the loop being checked has seen: the first binding id declared
/// inside it, and the moved-sets that reach each `break` and `continue`.
#[derive(Debug, Default)]
struct LoopFlow {
    outer_limit: usize,
    breaks: Vec<BTreeSet<usize>>,
    continues: Vec<BTreeSet<usize>>,
}

struct Checker<'a> {
    program: &'a CoreProgram,
    options: &'a CoreCheckOptions,
    structs: HashMap<String, StructInfo>,
    enums: HashMap<String, EnumInfo>,
    functions: HashMap<String, FunctionInfo>,
    constants: HashMap<String, Ty>,
    diagnostics: Vec<CoreDiagnostic>,
    scopes: Vec<HashMap<String, Binding>>,
    expected_return: Ty,
    loop_depth: usize,
    loop_breaks: Vec<Vec<Ty>>,
    /// Ownership: bindings of a resource type whose value has been moved on
    /// some path reaching the current point. Using one is an error.
    moved: BTreeSet<usize>,
    /// Whether the current point can be reached; after `return`, `break` or
    /// `continue` it cannot, and its moved-set does not flow anywhere.
    reachable: bool,
    next_binding: usize,
    /// The name of each binding id, for diagnostics.
    binding_names: Vec<String>,
    loop_flows: Vec<LoopFlow>,
    /// Set while checking a call argument that is `x.as_slice()` itself.
    view_argument: bool,
}

pub fn check_core_program(
    program: &CoreProgram,
    options: &CoreCheckOptions,
) -> Result<(), Vec<CoreDiagnostic>> {
    verify_core_program(program, options).map(|_| ())
}

pub fn verify_core_program<'a>(
    program: &'a CoreProgram,
    options: &CoreCheckOptions,
) -> Result<VerifiedCoreProgram<'a>, Vec<CoreDiagnostic>> {
    let mut checker = Checker::new(program, options);
    checker.collect_declarations();
    checker.check_imports();
    checker.check_recursive_values();
    checker.check_constants();
    checker.check_functions();
    if checker.diagnostics.is_empty() {
        Ok(VerifiedCoreProgram { program })
    } else {
        Err(checker.diagnostics)
    }
}

impl<'a> Checker<'a> {
    fn new(program: &'a CoreProgram, options: &'a CoreCheckOptions) -> Self {
        Self {
            program,
            options,
            structs: HashMap::new(),
            enums: HashMap::new(),
            functions: HashMap::new(),
            constants: HashMap::new(),
            diagnostics: Vec::new(),
            scopes: Vec::new(),
            expected_return: Ty::Unit,
            loop_depth: 0,
            loop_breaks: Vec::new(),
            moved: BTreeSet::new(),
            reachable: true,
            next_binding: 0,
            binding_names: Vec::new(),
            loop_flows: Vec::new(),
            view_argument: false,
        }
    }

    fn collect_declarations(&mut self) {
        let mut names = HashSet::new();
        for item in &self.program.items {
            let (name, span) = match &item.kind {
                CoreItemKind::Function(value) => (&value.name.value, value.name.span),
                CoreItemKind::Struct(value) => (&value.name.value, value.name.span),
                CoreItemKind::Enum(value) => (&value.name.value, value.name.span),
                CoreItemKind::Const(value) => (&value.name.value, value.name.span),
            };
            if !names.insert(name.clone()) {
                self.error(
                    "DuplicateDeclaration",
                    format!("duplicate declaration `{name}`"),
                    span,
                );
                continue;
            }
            match &item.kind {
                CoreItemKind::Struct(value) => {
                    let fields = value
                        .fields
                        .iter()
                        .map(|field| (field.name.value.clone(), self.lower_type(&field.ty)))
                        .collect();
                    self.structs.insert(
                        value.name.value.clone(),
                        StructInfo {
                            fields,
                            span: value.name.span,
                        },
                    );
                }
                CoreItemKind::Enum(value) => {
                    let variants = value
                        .variants
                        .iter()
                        .map(|variant| {
                            let fields = variant
                                .fields
                                .iter()
                                .map(|field| (field.name.value.clone(), self.lower_type(&field.ty)))
                                .collect();
                            (variant.name.value.clone(), VariantInfo { fields })
                        })
                        .collect();
                    self.enums
                        .insert(value.name.value.clone(), EnumInfo { variants });
                }
                CoreItemKind::Function(value) => {
                    let parameters = value
                        .parameters
                        .iter()
                        .map(|parameter| self.lower_type(&parameter.ty))
                        .collect();
                    let result = self.lower_type(&value.return_type);
                    self.functions.insert(
                        value.name.value.clone(),
                        FunctionInfo { parameters, result },
                    );
                }
                CoreItemKind::Const(value) => {
                    let ty = self.lower_type(&value.ty);
                    self.constants.insert(value.name.value.clone(), ty);
                }
            }
        }
    }

    fn check_imports(&mut self) {
        for import in &self.program.imports {
            if !self.options.available_modules.contains(&import.path.value) {
                self.diagnostics.push(CoreDiagnostic::new(
                    CorePhase::Resolution,
                    "ImportNotLocked",
                    format!(
                        "module `{}` is not present in the locked compilation set",
                        import.path.value
                    ),
                    import.path.span,
                ));
            }
        }
    }

    fn check_recursive_values(&mut self) {
        for (name, info) in &self.structs {
            if info
                .fields
                .values()
                .any(|ty| self.contains_unboxed_named(ty, name, &mut HashSet::new()))
            {
                self.diagnostics.push(CoreDiagnostic::new(
                    CorePhase::Semantic,
                    "InfiniteType",
                    format!("`{name}` contains itself without Handle indirection"),
                    info.span,
                ));
            }
        }
    }

    fn contains_unboxed_named(&self, ty: &Ty, target: &str, seen: &mut HashSet<String>) -> bool {
        match ty {
            Ty::Handle(_) => false,
            Ty::Named(name) if name == target => true,
            Ty::Named(name) if seen.insert(name.clone()) => {
                self.structs.get(name).is_some_and(|value| {
                    value
                        .fields
                        .values()
                        .any(|field| self.contains_unboxed_named(field, target, seen))
                })
            }
            Ty::Array(element, _)
            | Ty::Slice(element)
            | Ty::Buffer(element)
            | Ty::Arena(element) => self.contains_unboxed_named(element, target, seen),
            _ => false,
        }
    }

    fn check_constants(&mut self) {
        for item in &self.program.items {
            if let CoreItemKind::Const(value) = &item.kind {
                let expected = self.lower_type(&value.ty);
                let actual = self.infer_expr(&value.value, Some(&expected));
                self.require_compatible(&expected, &actual, value.value.span);
            }
        }
    }

    fn check_functions(&mut self) {
        for item in &self.program.items {
            let CoreItemKind::Function(function) = &item.kind else {
                continue;
            };
            self.scopes.clear();
            self.scopes.push(HashMap::new());
            self.moved.clear();
            self.reachable = true;
            self.loop_flows.clear();
            self.expected_return = self.lower_type(&function.return_type);
            for parameter in &function.parameters {
                let ty = self.lower_type(&parameter.ty);
                self.define(
                    &parameter.name.value,
                    Binding {
                        ty,
                        mutable: false,
                        id: 0,
                    },
                    parameter.name.span,
                );
            }
            let actual = self.check_block(&function.body, false);
            let expected = self.expected_return.clone();
            self.require_compatible(&expected, &actual, function.body.span);
        }
    }

    fn check_block(&mut self, block: &CoreBlock, scoped: bool) -> Ty {
        if scoped {
            self.scopes.push(HashMap::new());
        }
        for statement in &block.statements {
            self.check_statement(statement);
        }
        let result = block
            .tail
            .as_ref()
            .map(|tail| {
                // The tail is the block's value: it leaves the block, so a
                // resource local named there is moved out.
                let ty = self.infer_expr(tail, None);
                self.consume(tail, &ty);
                ty
            })
            .unwrap_or_else(|| {
                if block.statements.last().is_some_and(|statement| {
                    matches!(
                        statement.kind,
                        CoreStatementKind::Return(_)
                            | CoreStatementKind::Break(_)
                            | CoreStatementKind::Continue
                    )
                }) {
                    Ty::Never
                } else {
                    Ty::Unit
                }
            });
        if scoped {
            self.scopes.pop();
        }
        result
    }

    fn check_statement(&mut self, statement: &CoreStatement) {
        match &statement.kind {
            CoreStatementKind::Let {
                mutable,
                name,
                annotation,
                value,
            } => {
                let annotated = annotation.as_ref().map(|ty| self.lower_type(ty));
                let mut actual = self.infer_expr(value, annotated.as_ref());
                self.consume(value, &actual);
                if let Some(expected) = annotated {
                    self.require_compatible(&expected, &actual, value.span);
                    actual = self.merge_types(&expected, &actual);
                }
                self.define(
                    name.value.as_str(),
                    Binding {
                        ty: actual,
                        mutable: *mutable,
                        id: 0,
                    },
                    name.span,
                );
            }
            CoreStatementKind::Assign {
                target,
                operator,
                value,
            } => {
                let target_ty = self.assignment_target(target);
                let value_ty = self.infer_expr(value, Some(&target_ty));
                self.consume(value, &value_ty);
                self.require_compatible(&target_ty, &value_ty, value.span);
                // A moved local that is assigned owns a value again.
                if let CoreExprKind::Path(path) = &target.kind {
                    if let [name] = path.as_slice() {
                        if let Some(binding) = self.lookup(name) {
                            let id = binding.id;
                            self.moved.remove(&id);
                        }
                    }
                }
                if !matches!(operator, CoreAssignOp::Assign) && !target_ty.is_integer() {
                    self.error(
                        "TypeMismatch",
                        "compound assignment requires integer operands",
                        statement.span,
                    );
                }
            }
            CoreStatementKind::While { condition, body } => {
                let actual = self.infer_expr(condition, Some(&Ty::Bool));
                self.require_compatible(&Ty::Bool, &actual, condition.span);
                let entry = (self.moved.clone(), self.reachable);
                self.loop_depth += 1;
                self.loop_breaks.push(Vec::new());
                self.begin_loop();
                self.check_block(body, true);
                let flow = self.end_loop(&entry.0, body.span);
                self.loop_breaks.pop();
                self.loop_depth -= 1;
                // After the loop: the condition was false on entry, or a
                // `break` left it.
                let mut branches = vec![entry];
                branches.extend(flow.breaks.into_iter().map(|moved| (moved, true)));
                self.join(branches);
            }
            CoreStatementKind::Break(value) => {
                if self.loop_depth == 0 {
                    self.error(
                        "ControlOutsideLoop",
                        "`break` is only valid inside a loop",
                        statement.span,
                    );
                } else {
                    let ty = value
                        .as_ref()
                        .map(|value| {
                            let ty = self.infer_expr(value, None);
                            self.consume(value, &ty);
                            ty
                        })
                        .unwrap_or(Ty::Unit);
                    if let Some(values) = self.loop_breaks.last_mut() {
                        values.push(ty);
                    }
                    if self.reachable {
                        let moved = self.moved.clone();
                        if let Some(flow) = self.loop_flows.last_mut() {
                            flow.breaks.push(moved);
                        }
                    }
                    self.reachable = false;
                }
            }
            CoreStatementKind::Continue => {
                if self.loop_depth == 0 {
                    self.error(
                        "ControlOutsideLoop",
                        "`continue` is only valid inside a loop",
                        statement.span,
                    );
                } else {
                    if self.reachable {
                        let moved = self.moved.clone();
                        if let Some(flow) = self.loop_flows.last_mut() {
                            flow.continues.push(moved);
                        }
                    }
                    self.reachable = false;
                }
            }
            CoreStatementKind::Return(value) => {
                let expected = self.expected_return.clone();
                let actual = value
                    .as_ref()
                    .map(|value| {
                        let ty = self.infer_expr(value, Some(&expected));
                        self.consume(value, &ty);
                        ty
                    })
                    .unwrap_or(Ty::Unit);
                self.require_compatible(&expected, &actual, statement.span);
                self.reachable = false;
            }
            CoreStatementKind::Expr(expression) => {
                // A value computed and dropped on the floor: a resource local
                // named on its own is released here, so it counts as moved.
                let ty = self.infer_expr(expression, None);
                self.consume(expression, &ty);
            }
        }
    }

    fn infer_expr(&mut self, expression: &CoreExpr, expected: Option<&Ty>) -> Ty {
        match &expression.kind {
            CoreExprKind::Integer { value, suffix } => {
                let ty = suffix
                    .as_ref()
                    .map(|name| Ty::Int(name.clone()))
                    .or_else(|| expected.filter(|ty| ty.is_integer()).cloned())
                    .unwrap_or_else(|| Ty::Int("i64".into()));
                if let Ty::Int(name) = &ty {
                    if !integer_fits(*value, name) {
                        self.error(
                            "LiteralOutOfRangeOrConstantTrap",
                            format!("literal `{value}{name}` is outside `{name}`"),
                            expression.span,
                        );
                    }
                }
                ty
            }
            CoreExprKind::String(_) => Ty::String,
            CoreExprKind::Bool(_) => Ty::Bool,
            CoreExprKind::Unit => Ty::Unit,
            CoreExprKind::Path(path) => self.infer_path(path, expression.span),
            CoreExprKind::Aggregate { path, fields } => {
                self.infer_aggregate(path, fields, expression.span)
            }
            CoreExprKind::Array(values) => {
                let expected_element = match expected {
                    Some(Ty::Array(element, _)) => Some(element.as_ref()),
                    _ => None,
                };
                let mut element = expected_element.cloned().unwrap_or(Ty::Unknown);
                for value in values {
                    let actual = self.infer_expr(value, Some(&element));
                    self.consume(value, &actual);
                    if element == Ty::Unknown {
                        element = actual;
                    } else {
                        self.require_compatible(&element, &actual, value.span);
                    }
                }
                Ty::Array(Box::new(element), values.len() as u64)
            }
            CoreExprKind::Block(block) => self.check_block(block, true),
            CoreExprKind::If {
                condition,
                then_block,
                else_expr,
            } => {
                let condition_ty = self.infer_expr(condition, Some(&Ty::Bool));
                self.require_compatible(&Ty::Bool, &condition_ty, condition.span);
                let entry = (self.moved.clone(), self.reachable);
                let then_ty = self.check_block(then_block, true);
                let then_flow = (self.moved.clone(), self.reachable);
                (self.moved, self.reachable) = entry;
                let else_ty = else_expr
                    .as_ref()
                    .map(|value| {
                        let ty = self.infer_expr(value, expected);
                        self.consume(value, &ty);
                        ty
                    })
                    .unwrap_or(Ty::Unit);
                let else_flow = (self.moved.clone(), self.reachable);
                self.join(vec![then_flow, else_flow]);
                self.unify_branches(&then_ty, &else_ty, expression.span)
            }
            CoreExprKind::Match { value, arms } => {
                self.infer_match(value, arms, expected, expression.span)
            }
            CoreExprKind::Loop(block) => {
                let entry = self.moved.clone();
                self.loop_depth += 1;
                self.loop_breaks.push(Vec::new());
                self.begin_loop();
                self.check_block(block, true);
                let flow = self.end_loop(&entry, block.span);
                let breaks = self.loop_breaks.pop().unwrap_or_default();
                self.loop_depth -= 1;
                // Only a `break` leaves a `loop`.
                self.join(flow.breaks.into_iter().map(|moved| (moved, true)).collect());
                breaks
                    .into_iter()
                    .reduce(|left, right| self.unify_branches(&left, &right, expression.span))
                    .unwrap_or(Ty::Never)
            }
            CoreExprKind::Call { callee, arguments } => {
                self.infer_call(callee, arguments, expression.span)
            }
            CoreExprKind::Index { value, index } => {
                let base = self.infer_expr(value, None);
                self.require_place_for_resource(value, &base);
                let index_ty = self.infer_expr(index, Some(&Ty::Int("u64".into())));
                if !index_ty.is_integer() {
                    self.error("TypeMismatch", "index must be an integer", index.span);
                }
                match self.deref_handle(base) {
                    Ty::String => {
                        self.error(
                            "StringNotByteIndexable",
                            "string indexing is forbidden; decode or use bytes explicitly",
                            expression.span,
                        );
                        Ty::Unknown
                    }
                    Ty::Array(element, _) | Ty::Slice(element) | Ty::Buffer(element) => *element,
                    Ty::Bytes => Ty::Int("u8".into()),
                    other => {
                        self.error(
                            "TypeMismatch",
                            format!("`{}` is not indexable", other.display()),
                            value.span,
                        );
                        Ty::Unknown
                    }
                }
            }
            CoreExprKind::Field { value, name } => self.infer_field(value, &name.value, name.span),
            CoreExprKind::Unary { operator, value } => {
                let actual = self.infer_expr(value, expected);
                match operator {
                    CoreUnaryOp::Not if actual == Ty::Bool => Ty::Bool,
                    CoreUnaryOp::Negate if actual.is_integer() => actual,
                    _ => {
                        self.error("TypeMismatch", "invalid unary operand", expression.span);
                        Ty::Unknown
                    }
                }
            }
            CoreExprKind::Binary {
                left,
                operator,
                right,
            } => self.infer_binary(left, *operator, right, expression.span),
        }
    }

    fn infer_path(&mut self, path: &[String], span: Span) -> Ty {
        if path.len() == 1 {
            if let Some(binding) = self.lookup(&path[0]).cloned() {
                if self.moved.contains(&binding.id) && self.reachable {
                    self.error(
                        "UseAfterMove",
                        format!(
                            "`{}` was moved on a path that reaches here; a `{}` has one owner at a time",
                            path[0],
                            binding.ty.display()
                        ),
                        span,
                    );
                }
                return binding.ty;
            }
            if let Some(ty) = self.constants.get(&path[0]) {
                return ty.clone();
            }
        } else if path.len() == 2
            && self.enums.get(&path[0]).is_some_and(|value| {
                value
                    .variants
                    .get(&path[1])
                    .is_some_and(|variant| variant.fields.is_empty())
            })
        {
            return Ty::Named(path[0].clone());
        }
        self.error(
            "ImmutableAssignmentOrUnknownName",
            format!("unknown name `{}`", path.join("::")),
            span,
        );
        Ty::Unknown
    }

    fn infer_aggregate(
        &mut self,
        path: &[String],
        fields: &[(argorix_parser::span::Spanned<String>, CoreExpr)],
        span: Span,
    ) -> Ty {
        let (result, required) = if path.len() == 1 {
            (
                Ty::Named(path[0].clone()),
                self.structs.get(&path[0]).map(|value| value.fields.clone()),
            )
        } else if path.len() == 2 {
            (
                Ty::Named(path[0].clone()),
                self.enums
                    .get(&path[0])
                    .and_then(|value| value.variants.get(&path[1]))
                    .map(|value| value.fields.clone()),
            )
        } else {
            (Ty::Unknown, None)
        };
        let Some(required) = required else {
            self.error(
                "TypeMismatch",
                format!("unknown aggregate `{}`", path.join("::")),
                span,
            );
            return Ty::Unknown;
        };
        for (name, value) in fields {
            if let Some(expected) = required.get(&name.value) {
                let actual = self.infer_expr(value, Some(expected));
                self.consume(value, &actual);
                self.require_compatible(expected, &actual, value.span);
            } else {
                self.error(
                    "TypeMismatch",
                    format!("unknown field `{}`", name.value),
                    name.span,
                );
            }
        }
        for name in required.keys() {
            if !fields.iter().any(|(field, _)| &field.value == name) {
                self.error("TypeMismatch", format!("missing field `{name}`"), span);
            }
        }
        result
    }

    fn infer_call(&mut self, callee: &CoreExpr, arguments: &[CoreExpr], span: Span) -> Ty {
        if let CoreExprKind::Path(path) = &callee.kind {
            if path == &["Buffer".to_string(), "new".to_string()] && arguments.is_empty() {
                return Ty::Buffer(Box::new(Ty::Unknown));
            }
            if path == &["Arena".to_string(), "new".to_string()] && arguments.is_empty() {
                return Ty::Arena(Box::new(Ty::Unknown));
            }
            if path.len() == 1 {
                if let Some(function) = self.functions.get(&path[0]).cloned() {
                    if function.parameters.len() != arguments.len() {
                        self.error("TypeMismatch", "wrong argument count", span);
                    }
                    // Parameters are passed by value: a resource argument
                    // moves into the callee. `x.as_slice()` is the one view
                    // of owned storage, and only as a direct argument, where
                    // the call bounds its life.
                    let before = self.moved.clone();
                    let viewed: Vec<String> = arguments
                        .iter()
                        .filter_map(|argument| Self::viewed_root(argument).map(str::to_string))
                        .collect();
                    for (argument, expected) in arguments.iter().zip(&function.parameters) {
                        self.view_argument = Self::viewed_root(argument).is_some();
                        let actual = self.infer_expr(argument, Some(expected));
                        self.view_argument = false;
                        self.consume(argument, &actual);
                        self.require_compatible(expected, &actual, argument.span);
                    }
                    // The storage a view points into cannot also move into the
                    // same call, where the callee could grow or release it.
                    for root in viewed {
                        if let Some(binding) = self.lookup(&root) {
                            let id = binding.id;
                            if self.moved.contains(&id) && !before.contains(&id) && self.reachable {
                                self.error(
                                    "SliceAliasesMove",
                                    format!("`{root}` is both viewed and moved by this call"),
                                    span,
                                );
                            }
                        }
                    }
                    return function.result;
                }
            }
        }
        if let CoreExprKind::Field { value, name } = &callee.kind {
            let view_allowed = std::mem::replace(&mut self.view_argument, false);
            let receiver = self.infer_expr(value, None);
            self.require_place_for_resource(value, &receiver);
            if name.value == "push" {
                // Growing a buffer changes it, so the place must be mutable,
                // unless it is reached through a handle: then the arena slot
                // changes, not the binding.
                if let Some(root) = Self::place_root(value) {
                    if let Some(binding) = self.lookup(root).cloned() {
                        if !binding.mutable && !matches!(binding.ty, Ty::Handle(_)) {
                            self.error(
                                "ImmutableAssignmentOrUnknownName",
                                format!("`{root}` is immutable; declare it with `let mut` to push into it"),
                                value.span,
                            );
                        }
                    }
                }
            }
            return self.infer_method(receiver, &name.value, arguments, span, view_allowed);
        }
        self.infer_expr(callee, None);
        self.error("TypeMismatch", "expression is not callable", span);
        Ty::Unknown
    }

    fn infer_method(
        &mut self,
        receiver: Ty,
        name: &str,
        arguments: &[CoreExpr],
        span: Span,
        view_allowed: bool,
    ) -> Ty {
        let receiver = self.deref_handle(receiver);
        match (name, receiver) {
            ("as_slice", Ty::Buffer(element) | Ty::Array(element, _)) if arguments.is_empty() => {
                if !view_allowed && self.reachable {
                    self.error(
                        "SliceEscapes",
                        "a view of owned storage can only be passed directly as an argument: `f(x.as_slice())`",
                        span,
                    );
                }
                Ty::Slice(element)
            }
            ("slice", Ty::Slice(element)) if arguments.len() == 2 => {
                // A narrower view of a view; bounds are checked at run time.
                for argument in arguments {
                    let actual = self.infer_expr(argument, Some(&Ty::Int("u64".into())));
                    self.require_compatible(&Ty::Int("u64".into()), &actual, argument.span);
                }
                Ty::Slice(element)
            }
            ("length", Ty::Array(_, _) | Ty::Slice(_) | Ty::Buffer(_) | Ty::Bytes | Ty::String)
                if arguments.is_empty() =>
            {
                Ty::Int("u64".into())
            }
            ("push", Ty::Buffer(element)) if arguments.len() == 1 => {
                let actual = self.infer_expr(&arguments[0], Some(&element));
                self.consume(&arguments[0], &actual);
                if *element != Ty::Unknown {
                    self.require_compatible(&element, &actual, arguments[0].span);
                }
                Ty::Unit
            }
            ("alloc", Ty::Arena(element)) if arguments.len() == 1 => {
                let actual = self.infer_expr(&arguments[0], Some(&element));
                if self.is_resource(&actual) {
                    // An arena slot is freed without looking inside it, so a
                    // value that owns storage would leak there.
                    self.error(
                        "ResourceInArena",
                        format!(
                            "a `{}` owns storage and cannot live in an arena slot",
                            actual.display()
                        ),
                        arguments[0].span,
                    );
                }
                self.require_compatible(&element, &actual, arguments[0].span);
                Ty::Handle(element)
            }
            ("release", Ty::Arena(_)) if arguments.is_empty() => Ty::Unit,
            ("as_bytes", Ty::Array(element, _))
                if *element == Ty::Int("u8".into()) && arguments.is_empty() =>
            {
                Ty::Bytes
            }
            ("decode_utf8_or_trap", Ty::Bytes) if arguments.is_empty() => Ty::String,
            ("wrapping_add", integer) if integer.is_integer() && arguments.len() == 1 => {
                let actual = self.infer_expr(&arguments[0], Some(&integer));
                self.require_compatible(&integer, &actual, arguments[0].span);
                integer
            }
            _ => {
                self.error("TypeMismatch", format!("unknown method `{name}`"), span);
                Ty::Unknown
            }
        }
    }

    fn infer_field(&mut self, value: &CoreExpr, name: &str, span: Span) -> Ty {
        let inferred = self.infer_expr(value, None);
        self.require_place_for_resource(value, &inferred);
        let base = self.deref_handle(inferred);
        if let Ty::Named(type_name) = base {
            if let Some(field) = self
                .structs
                .get(&type_name)
                .and_then(|value| value.fields.get(name))
            {
                return field.clone();
            }
        }
        self.error("TypeMismatch", format!("unknown field `{name}`"), span);
        Ty::Unknown
    }

    fn infer_binary(
        &mut self,
        left: &CoreExpr,
        operator: CoreBinaryOp,
        right: &CoreExpr,
        span: Span,
    ) -> Ty {
        use CoreBinaryOp::*;
        match operator {
            Or | And => {
                let left_ty = self.infer_expr(left, Some(&Ty::Bool));
                // The right side may not run, so what it moves is only
                // maybe-moved afterwards.
                let skipped = (self.moved.clone(), self.reachable);
                let right_ty = self.infer_expr(right, Some(&Ty::Bool));
                let evaluated = (self.moved.clone(), self.reachable);
                self.join(vec![skipped, evaluated]);
                self.require_compatible(&Ty::Bool, &left_ty, left.span);
                self.require_compatible(&Ty::Bool, &right_ty, right.span);
                Ty::Bool
            }
            Equal | NotEqual => {
                let left_ty = self.infer_expr(left, None);
                let right_ty = self.infer_expr(right, Some(&left_ty));
                self.require_compatible(&left_ty, &right_ty, span);
                Ty::Bool
            }
            Less | LessEqual | Greater | GreaterEqual => {
                let left_ty = self.infer_expr(left, None);
                let right_ty = self.infer_expr(right, Some(&left_ty));
                if !left_ty.is_integer() || !right_ty.is_integer() {
                    self.error("TypeMismatch", "comparison requires integers", span);
                }
                self.require_compatible(&left_ty, &right_ty, span);
                Ty::Bool
            }
            BitOr | BitXor | BitAnd | ShiftLeft | ShiftRight | Add | Subtract | Multiply
            | Divide | Remainder => {
                let left_ty = self.infer_expr(left, None);
                let right_ty = self.infer_expr(right, Some(&left_ty));
                if !left_ty.is_integer() || !right_ty.is_integer() {
                    self.error("TypeMismatch", "arithmetic requires integers", span);
                }
                self.require_compatible(&left_ty, &right_ty, span);
                if matches!(operator, Divide | Remainder)
                    && matches!(right.kind, CoreExprKind::Integer { value: 0, .. })
                {
                    self.error(
                        "LiteralOutOfRangeOrConstantTrap",
                        "constant division by zero",
                        right.span,
                    );
                }
                left_ty
            }
        }
    }

    fn infer_match(
        &mut self,
        value: &CoreExpr,
        arms: &[CoreMatchArm],
        expected: Option<&Ty>,
        span: Span,
    ) -> Ty {
        let scrutinee = self.infer_expr(value, None);
        // Matching takes the scrutinee: the arm that runs owns what its
        // pattern binds, and the backend releases the rest.
        self.consume(value, &scrutinee);
        let entry = (self.moved.clone(), self.reachable);
        let mut flows = Vec::new();
        let mut result = Ty::Never;
        let mut wildcard = false;
        let mut covered = HashSet::new();
        for arm in arms {
            (self.moved, self.reachable) = entry.clone();
            self.scopes.push(HashMap::new());
            self.check_pattern(&arm.pattern, &scrutinee, &mut wildcard, &mut covered);
            if let Some(guard) = &arm.guard {
                let guard_ty = self.infer_expr(guard, Some(&Ty::Bool));
                self.require_compatible(&Ty::Bool, &guard_ty, guard.span);
            }
            let arm_ty = self.infer_expr(&arm.value, expected);
            self.consume(&arm.value, &arm_ty);
            result = if result == Ty::Never {
                arm_ty
            } else {
                self.unify_branches(&result, &arm_ty, arm.span)
            };
            self.scopes.pop();
            flows.push((self.moved.clone(), self.reachable));
        }
        if flows.is_empty() {
            flows.push(entry);
        }
        self.join(flows);
        if let Ty::Named(name) = &scrutinee {
            if let Some(info) = self.enums.get(name) {
                let missing: Vec<_> = info
                    .variants
                    .keys()
                    .filter(|variant| !covered.contains(*variant))
                    .cloned()
                    .collect();
                if !wildcard && !missing.is_empty() {
                    self.error(
                        "NonExhaustiveMatch",
                        format!("missing variants: {}", missing.join(", ")),
                        span,
                    );
                }
            }
        }
        result
    }

    fn check_pattern(
        &mut self,
        pattern: &CorePattern,
        expected: &Ty,
        wildcard: &mut bool,
        covered: &mut HashSet<String>,
    ) {
        match &pattern.kind {
            CorePatternKind::Wildcard => *wildcard = true,
            CorePatternKind::Binding(name) => {
                self.define(
                    name,
                    Binding {
                        ty: expected.clone(),
                        mutable: false,
                        id: 0,
                    },
                    pattern.span,
                );
            }
            CorePatternKind::Bool(_) => self.require_compatible(&Ty::Bool, expected, pattern.span),
            CorePatternKind::Integer(_) => {
                if !expected.is_integer() {
                    self.error(
                        "TypeMismatch",
                        "integer pattern requires integer value",
                        pattern.span,
                    );
                }
            }
            CorePatternKind::Variant { path, fields } => {
                let Ty::Named(enum_name) = expected else {
                    self.error(
                        "TypeMismatch",
                        "variant pattern requires enum value",
                        pattern.span,
                    );
                    return;
                };
                if path.len() != 2 || &path[0] != enum_name {
                    self.error(
                        "TypeMismatch",
                        "variant does not belong to matched enum",
                        pattern.span,
                    );
                    return;
                }
                let Some(variant) = self
                    .enums
                    .get(enum_name)
                    .and_then(|value| value.variants.get(&path[1]))
                    .cloned()
                else {
                    self.error("TypeMismatch", "unknown enum variant", pattern.span);
                    return;
                };
                covered.insert(path[1].clone());
                for (name, nested) in fields {
                    if let Some(field_ty) = variant.fields.get(&name.value) {
                        if let Some(nested) = nested {
                            self.check_pattern(nested, field_ty, wildcard, covered);
                        } else {
                            self.define(
                                &name.value,
                                Binding {
                                    ty: field_ty.clone(),
                                    mutable: false,
                                    id: 0,
                                },
                                name.span,
                            );
                        }
                    } else {
                        self.error(
                            "TypeMismatch",
                            format!("unknown variant field `{}`", name.value),
                            name.span,
                        );
                    }
                }
            }
        }
    }

    fn assignment_target(&mut self, target: &CoreExpr) -> Ty {
        match &target.kind {
            CoreExprKind::Path(path) if path.len() == 1 => match self.lookup(&path[0]).cloned() {
                Some(binding) if binding.mutable => binding.ty,
                Some(binding) => {
                    self.error(
                        "ImmutableAssignmentOrUnknownName",
                        format!("`{}` is immutable", path[0]),
                        target.span,
                    );
                    binding.ty
                }
                None => {
                    self.error(
                        "ImmutableAssignmentOrUnknownName",
                        format!("unknown name `{}`", path[0]),
                        target.span,
                    );
                    Ty::Unknown
                }
            },
            CoreExprKind::Field { .. } | CoreExprKind::Index { .. }
                if self.root_is_mutable_or_handle(target) =>
            {
                self.infer_expr(target, None)
            }
            _ => {
                self.error(
                    "ImmutableAssignmentOrUnknownName",
                    "assignment target is not mutable",
                    target.span,
                );
                self.infer_expr(target, None)
            }
        }
    }

    fn root_is_mutable_or_handle(&self, expression: &CoreExpr) -> bool {
        match &expression.kind {
            CoreExprKind::Path(path) if path.len() == 1 => self
                .lookup(&path[0])
                .is_some_and(|binding| binding.mutable || matches!(binding.ty, Ty::Handle(_))),
            CoreExprKind::Field { value, .. } | CoreExprKind::Index { value, .. } => {
                self.root_is_mutable_or_handle(value)
            }
            _ => false,
        }
    }

    fn lower_type(&mut self, source: &CoreType) -> Ty {
        match &source.kind {
            CoreTypeKind::Named(name) => match name.as_str() {
                "unit" => Ty::Unit,
                "bool" => Ty::Bool,
                "bytes" => Ty::Bytes,
                "string" => Ty::String,
                "u8" | "u16" | "u32" | "u64" | "i8" | "i16" | "i32" | "i64" => {
                    Ty::Int(name.clone())
                }
                _ => Ty::Named(name.clone()),
            },
            CoreTypeKind::Container {
                name,
                element,
                array_length,
            } => {
                let element = Box::new(self.lower_type(element));
                if Self::contains_slice(&element) {
                    self.error(
                        "SliceEscapes",
                        format!("a slice is a view into storage it does not own and cannot be held in a `{name}`"),
                        source.span,
                    );
                }
                match name.as_str() {
                    "Array" => Ty::Array(element, array_length.unwrap_or(0)),
                    "Slice" => Ty::Slice(element),
                    "Buffer" => Ty::Buffer(element),
                    "Arena" => Ty::Arena(element),
                    "Handle" => Ty::Handle(element),
                    _ => {
                        self.error(
                            "FeatureDeferred",
                            format!("unsupported container `{name}`"),
                            source.span,
                        );
                        Ty::Unknown
                    }
                }
            }
        }
    }

    fn deref_handle(&self, mut ty: Ty) -> Ty {
        while let Ty::Handle(value) = ty {
            ty = *value;
        }
        ty
    }

    fn compatible(&self, expected: &Ty, actual: &Ty) -> bool {
        if expected == actual
            || matches!(expected, Ty::Unknown | Ty::Never)
            || matches!(actual, Ty::Unknown | Ty::Never)
        {
            return true;
        }
        match (expected, actual) {
            (Ty::Buffer(left), Ty::Buffer(right)) if **right == Ty::Unknown => true,
            (Ty::Array(left, left_len), Ty::Array(right, right_len)) => {
                left_len == right_len && self.compatible(left, right)
            }
            (Ty::Slice(left), Ty::Slice(right))
            | (Ty::Buffer(left), Ty::Buffer(right))
            | (Ty::Arena(left), Ty::Arena(right))
            | (Ty::Handle(left), Ty::Handle(right)) => self.compatible(left, right),
            _ => false,
        }
    }

    fn merge_types(&self, expected: &Ty, actual: &Ty) -> Ty {
        if matches!(actual, Ty::Unknown) {
            expected.clone()
        } else if let (Ty::Buffer(left), Ty::Buffer(right)) = (expected, actual) {
            if **right == Ty::Unknown {
                Ty::Buffer(left.clone())
            } else {
                actual.clone()
            }
        } else if let (Ty::Arena(left), Ty::Arena(right)) = (expected, actual) {
            if **right == Ty::Unknown {
                Ty::Arena(left.clone())
            } else {
                actual.clone()
            }
        } else {
            actual.clone()
        }
    }

    fn require_compatible(&mut self, expected: &Ty, actual: &Ty, span: Span) {
        if !self.compatible(expected, actual) {
            self.error(
                "TypeMismatch",
                format!(
                    "expected `{}`, found `{}`",
                    expected.display(),
                    actual.display()
                ),
                span,
            );
        }
    }

    fn unify_branches(&mut self, left: &Ty, right: &Ty, span: Span) -> Ty {
        if left == &Ty::Never {
            return right.clone();
        }
        if right == &Ty::Never {
            return left.clone();
        }
        if self.compatible(left, right) {
            return self.merge_types(left, right);
        }
        self.error(
            "TypeMismatch",
            format!(
                "branch types differ: `{}` and `{}`",
                left.display(),
                right.display()
            ),
            span,
        );
        Ty::Unknown
    }

    /// Whether a value of this type owns storage: a `Buffer`, or a struct,
    /// enum or array that holds one. Such a value moves instead of copying.
    fn is_resource(&self, ty: &Ty) -> bool {
        self.is_resource_seen(ty, &mut HashSet::new())
    }

    fn is_resource_seen(&self, ty: &Ty, seen: &mut HashSet<String>) -> bool {
        match ty {
            Ty::Buffer(_) => true,
            Ty::Array(element, _) => self.is_resource_seen(element, seen),
            Ty::Named(name) => {
                if !seen.insert(name.clone()) {
                    return false;
                }
                if let Some(info) = self.structs.get(name) {
                    info.fields
                        .values()
                        .any(|field| self.is_resource_seen(field, seen))
                } else if let Some(info) = self.enums.get(name) {
                    info.variants.values().any(|variant| {
                        variant
                            .fields
                            .values()
                            .any(|field| self.is_resource_seen(field, seen))
                    })
                } else {
                    false
                }
            }
            _ => false,
        }
    }

    fn contains_slice(ty: &Ty) -> bool {
        match ty {
            Ty::Slice(_) => true,
            Ty::Array(element, _)
            | Ty::Buffer(element)
            | Ty::Arena(element)
            | Ty::Handle(element) => Self::contains_slice(element),
            _ => false,
        }
    }

    /// The local at the root of a place, if the expression is one.
    fn place_root(expression: &CoreExpr) -> Option<&str> {
        match &expression.kind {
            CoreExprKind::Path(path) if path.len() == 1 => Some(&path[0]),
            CoreExprKind::Field { value, .. } | CoreExprKind::Index { value, .. } => {
                Self::place_root(value)
            }
            _ => None,
        }
    }

    /// `x.as_slice()`, the only way to make a view of owned storage: its
    /// receiver's root local, if the expression is that call.
    fn viewed_root(expression: &CoreExpr) -> Option<&str> {
        let CoreExprKind::Call { callee, .. } = &expression.kind else {
            return None;
        };
        let CoreExprKind::Field { value, name } = &callee.kind else {
            return None;
        };
        (name.value == "as_slice")
            .then(|| Self::place_root(value))
            .flatten()
    }

    /// A local, or a field or element of one: something that has a place in
    /// memory and can be inspected without taking it.
    fn is_place(expression: &CoreExpr) -> bool {
        match &expression.kind {
            CoreExprKind::Path(path) => path.len() == 1,
            CoreExprKind::Field { value, .. } | CoreExprKind::Index { value, .. } => {
                Self::is_place(value)
            }
            _ => false,
        }
    }

    /// `expression`, of type `ty`, is handed to a new owner: bound, passed,
    /// returned, stored or discarded. A resource local is moved by that; a
    /// resource field or element cannot be, since it would leave a hole in
    /// the value that still owns it.
    fn consume(&mut self, expression: &CoreExpr, ty: &Ty) {
        if !self.is_resource(ty) {
            return;
        }
        match &expression.kind {
            CoreExprKind::Path(path) if path.len() == 1 => {
                if let Some(binding) = self.lookup(&path[0]) {
                    let id = binding.id;
                    self.moved.insert(id);
                }
            }
            CoreExprKind::Field { .. } | CoreExprKind::Index { .. } if self.reachable => {
                self.error(
                    "MoveOutOfPlace",
                    format!(
                        "a `{}` cannot be moved out of a field or an element; the value that holds it still owns it",
                        ty.display()
                    ),
                    expression.span,
                );
            }
            _ => {}
        }
    }

    /// A resource used only to be inspected must be a place: a temporary of
    /// that kind would have no owner to release it afterwards.
    fn require_place_for_resource(&mut self, expression: &CoreExpr, ty: &Ty) {
        if self.is_resource(ty) && !Self::is_place(expression) && self.reachable {
            self.error(
                "ResourceTemporary",
                format!(
                    "bind this `{}` to a local before inspecting it, so something owns it",
                    ty.display()
                ),
                expression.span,
            );
        }
    }

    fn begin_loop(&mut self) {
        self.loop_flows.push(LoopFlow {
            outer_limit: self.next_binding,
            ..LoopFlow::default()
        });
    }

    /// Check the paths back to the loop head: a binding declared before the
    /// loop and moved on one of them would be moved again by the next
    /// iteration, so it has to be moved after the loop, or given a new value
    /// before the iteration ends.
    fn end_loop(&mut self, entry: &BTreeSet<usize>, span: Span) -> LoopFlow {
        let flow = self.loop_flows.pop().unwrap_or_default();
        let mut back = flow.continues.clone();
        if self.reachable {
            back.push(self.moved.clone());
        }
        let mut reported = BTreeSet::new();
        for state in &back {
            for id in state {
                if *id < flow.outer_limit && !entry.contains(id) && reported.insert(*id) {
                    let name = self.binding_names.get(*id).cloned().unwrap_or_default();
                    self.error(
                        "MoveInLoop",
                        format!(
                            "`{name}` is declared before this loop and moved inside it, so the next iteration would move it again; move it after the loop, or assign it before the iteration ends"
                        ),
                        span,
                    );
                }
            }
        }
        flow
    }

    /// Join the moved-sets of the paths that meet here: a binding moved on
    /// any of them may be moved, and cannot be used.
    fn join(&mut self, branches: Vec<(BTreeSet<usize>, bool)>) {
        let mut moved = BTreeSet::new();
        let mut reachable = false;
        for (branch, branch_reachable) in branches {
            if branch_reachable {
                moved.extend(branch);
                reachable = true;
            }
        }
        self.moved = moved;
        self.reachable = reachable;
    }

    fn define(&mut self, name: &str, mut binding: Binding, span: Span) {
        binding.id = self.next_binding;
        self.next_binding += 1;
        self.binding_names.push(name.to_string());
        let scope = self.scopes.last_mut().expect("scope exists");
        if scope.insert(name.to_string(), binding).is_some() {
            self.error(
                "DuplicateDeclaration",
                format!("duplicate local `{name}`"),
                span,
            );
        }
    }

    fn lookup(&self, name: &str) -> Option<&Binding> {
        self.scopes.iter().rev().find_map(|scope| scope.get(name))
    }

    fn error(&mut self, code: &str, message: impl Into<String>, span: Span) {
        self.diagnostics.push(CoreDiagnostic::new(
            CorePhase::Semantic,
            code,
            message,
            span,
        ));
    }
}

fn integer_fits(value: u64, name: &str) -> bool {
    match name {
        "u8" => value <= u8::MAX as u64,
        "u16" => value <= u16::MAX as u64,
        "u32" => value <= u32::MAX as u64,
        "u64" | "i64" => true,
        "i8" => value <= i8::MAX as u64,
        "i16" => value <= i16::MAX as u64,
        "i32" => value <= i32::MAX as u64,
        _ => false,
    }
}
