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
}

pub fn check_core_program(
    program: &CoreProgram,
    options: &CoreCheckOptions,
) -> Result<(), Vec<CoreDiagnostic>> {
    let mut checker = Checker::new(program, options);
    checker.collect_declarations();
    checker.check_imports();
    checker.check_recursive_values();
    checker.check_constants();
    checker.check_functions();
    if checker.diagnostics.is_empty() {
        Ok(())
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
            self.expected_return = self.lower_type(&function.return_type);
            for parameter in &function.parameters {
                let ty = self.lower_type(&parameter.ty);
                self.define(
                    &parameter.name.value,
                    Binding { ty, mutable: false },
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
            .map(|tail| self.infer_expr(tail, None))
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
                if let Some(expected) = annotated {
                    self.require_compatible(&expected, &actual, value.span);
                    actual = self.merge_types(&expected, &actual);
                }
                self.define(
                    name.value.as_str(),
                    Binding {
                        ty: actual,
                        mutable: *mutable,
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
                self.require_compatible(&target_ty, &value_ty, value.span);
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
                self.loop_depth += 1;
                self.loop_breaks.push(Vec::new());
                self.check_block(body, true);
                self.loop_breaks.pop();
                self.loop_depth -= 1;
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
                        .map(|value| self.infer_expr(value, None))
                        .unwrap_or(Ty::Unit);
                    if let Some(values) = self.loop_breaks.last_mut() {
                        values.push(ty);
                    }
                }
            }
            CoreStatementKind::Continue => {
                if self.loop_depth == 0 {
                    self.error(
                        "ControlOutsideLoop",
                        "`continue` is only valid inside a loop",
                        statement.span,
                    );
                }
            }
            CoreStatementKind::Return(value) => {
                let expected = self.expected_return.clone();
                let actual = value
                    .as_ref()
                    .map(|value| self.infer_expr(value, Some(&expected)))
                    .unwrap_or(Ty::Unit);
                self.require_compatible(&expected, &actual, statement.span);
            }
            CoreStatementKind::Expr(expression) => {
                self.infer_expr(expression, None);
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
                let then_ty = self.check_block(then_block, true);
                let else_ty = else_expr
                    .as_ref()
                    .map(|value| self.infer_expr(value, expected))
                    .unwrap_or(Ty::Unit);
                self.unify_branches(&then_ty, &else_ty, expression.span)
            }
            CoreExprKind::Match { value, arms } => {
                self.infer_match(value, arms, expected, expression.span)
            }
            CoreExprKind::Loop(block) => {
                self.loop_depth += 1;
                self.loop_breaks.push(Vec::new());
                self.check_block(block, true);
                let breaks = self.loop_breaks.pop().unwrap_or_default();
                self.loop_depth -= 1;
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
            if let Some(binding) = self.lookup(&path[0]) {
                return binding.ty.clone();
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
            if path.len() == 1 {
                if let Some(function) = self.functions.get(&path[0]).cloned() {
                    if function.parameters.len() != arguments.len() {
                        self.error("TypeMismatch", "wrong argument count", span);
                    }
                    for (argument, expected) in arguments.iter().zip(&function.parameters) {
                        let actual = self.infer_expr(argument, Some(expected));
                        self.require_compatible(expected, &actual, argument.span);
                    }
                    return function.result;
                }
            }
        }
        if let CoreExprKind::Field { value, name } = &callee.kind {
            let receiver = self.infer_expr(value, None);
            return self.infer_method(receiver, &name.value, arguments, span);
        }
        self.infer_expr(callee, None);
        self.error("TypeMismatch", "expression is not callable", span);
        Ty::Unknown
    }

    fn infer_method(&mut self, receiver: Ty, name: &str, arguments: &[CoreExpr], span: Span) -> Ty {
        let receiver = self.deref_handle(receiver);
        match (name, receiver) {
            ("length", Ty::Array(_, _) | Ty::Slice(_) | Ty::Buffer(_) | Ty::Bytes | Ty::String)
                if arguments.is_empty() =>
            {
                Ty::Int("u64".into())
            }
            ("push", Ty::Buffer(element)) if arguments.len() == 1 => {
                let actual = self.infer_expr(&arguments[0], Some(&element));
                if *element != Ty::Unknown {
                    self.require_compatible(&element, &actual, arguments[0].span);
                }
                Ty::Unit
            }
            ("alloc", Ty::Arena(element)) if arguments.len() == 1 => {
                let actual = self.infer_expr(&arguments[0], Some(&element));
                self.require_compatible(&element, &actual, arguments[0].span);
                Ty::Handle(element)
            }
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
                let right_ty = self.infer_expr(right, Some(&Ty::Bool));
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
        let mut result = Ty::Never;
        let mut wildcard = false;
        let mut covered = HashSet::new();
        for arm in arms {
            self.scopes.push(HashMap::new());
            self.check_pattern(&arm.pattern, &scrutinee, &mut wildcard, &mut covered);
            if let Some(guard) = &arm.guard {
                let guard_ty = self.infer_expr(guard, Some(&Ty::Bool));
                self.require_compatible(&Ty::Bool, &guard_ty, guard.span);
            }
            let arm_ty = self.infer_expr(&arm.value, expected);
            result = if result == Ty::Never {
                arm_ty
            } else {
                self.unify_branches(&result, &arm_ty, arm.span)
            };
            self.scopes.pop();
        }
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

    fn define(&mut self, name: &str, binding: Binding, span: Span) {
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
