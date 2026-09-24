//! Versioned structured IR for Argorix Core 0.1.
//!
//! This module is a transitional Rust stage0 implementation. It is deliberately
//! separate from the historical agent IR and must be replaced by `.argx`
//! compiler sources during ESP-013.

use argorix_parser::{
    core::{
        CoreAssignOp, CoreBinaryOp, CoreBlock, CoreConst, CoreEnum, CoreExpr, CoreExprKind,
        CoreField, CoreFunction, CoreImport, CoreItem, CoreItemKind, CoreMatchArm, CoreParameter,
        CorePattern, CorePatternKind, CoreProgram, CoreStatement, CoreStatementKind, CoreStruct,
        CoreType, CoreTypeKind, CoreUnaryOp, CoreVariant,
    },
    span::{Span, Spanned},
};
use argorix_semantics::{
    check_core_package, core_package, verify_core_program, CoreCheckOptions, VerifiedCoreProgram,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;

pub const CORE_IR_VERSION: &str = "0.1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CoreIrProgram {
    pub ir_version: String,
    pub core_version: String,
    pub module: String,
    #[serde(default)]
    pub imports: Vec<CoreIrImport>,
    #[serde(default)]
    pub locked_modules: Vec<String>,
    #[serde(default)]
    pub effect_policy: Vec<CoreIrEffect>,
    pub items: Vec<CoreIrItem>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CoreIrImport {
    pub path: String,
    pub alias: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(tag = "effect", content = "detail", rename_all = "snake_case")]
pub enum CoreIrEffect {
    MemoryRead,
    MemoryWrite,
    Allocate,
    Trap,
    Host(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CoreIrItem {
    pub public: bool,
    #[serde(flatten)]
    pub kind: CoreIrItemKind,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "item", rename_all = "snake_case")]
pub enum CoreIrItemKind {
    Function(CoreIrFunction),
    Struct(CoreIrStruct),
    Enum(CoreIrEnum),
    Const(CoreIrConst),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CoreIrFunction {
    pub name: String,
    pub parameters: Vec<CoreIrParameter>,
    pub return_type: CoreIrType,
    pub body: CoreIrBlock,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CoreIrParameter {
    pub name: String,
    pub ty: CoreIrType,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CoreIrStruct {
    pub name: String,
    pub fields: Vec<CoreIrField>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CoreIrField {
    pub name: String,
    pub ty: CoreIrType,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CoreIrEnum {
    pub name: String,
    pub variants: Vec<CoreIrVariant>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CoreIrVariant {
    pub name: String,
    pub fields: Vec<CoreIrField>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CoreIrConst {
    pub name: String,
    pub ty: CoreIrType,
    pub value: CoreIrExpr,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum CoreIrType {
    Named {
        name: String,
    },
    Container {
        name: String,
        element: Box<CoreIrType>,
        #[serde(skip_serializing_if = "Option::is_none")]
        array_length: Option<u64>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CoreIrBlock {
    pub statements: Vec<CoreIrStatement>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tail: Option<Box<CoreIrExpr>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "statement", rename_all = "snake_case")]
pub enum CoreIrStatement {
    Let {
        mutable: bool,
        name: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        annotation: Option<CoreIrType>,
        value: CoreIrExpr,
    },
    Assign {
        target: CoreIrExpr,
        operator: CoreIrAssignOp,
        value: CoreIrExpr,
    },
    While {
        condition: CoreIrExpr,
        body: CoreIrBlock,
    },
    Break {
        #[serde(skip_serializing_if = "Option::is_none")]
        value: Option<CoreIrExpr>,
    },
    Continue,
    Return {
        #[serde(skip_serializing_if = "Option::is_none")]
        value: Option<CoreIrExpr>,
    },
    Expr {
        value: CoreIrExpr,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CoreIrAssignOp {
    Assign,
    Add,
    Subtract,
    Multiply,
    Divide,
    Remainder,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "expression", rename_all = "snake_case")]
pub enum CoreIrExpr {
    Integer {
        value: u64,
        #[serde(skip_serializing_if = "Option::is_none")]
        suffix: Option<String>,
    },
    String {
        value: String,
    },
    Bool {
        value: bool,
    },
    Unit,
    Path {
        segments: Vec<String>,
    },
    Aggregate {
        path: Vec<String>,
        fields: Vec<CoreIrNamedExpr>,
    },
    Array {
        values: Vec<CoreIrExpr>,
    },
    Block {
        body: CoreIrBlock,
    },
    If {
        condition: Box<CoreIrExpr>,
        then_block: CoreIrBlock,
        #[serde(skip_serializing_if = "Option::is_none")]
        else_expr: Option<Box<CoreIrExpr>>,
    },
    Match {
        value: Box<CoreIrExpr>,
        arms: Vec<CoreIrMatchArm>,
    },
    Loop {
        body: CoreIrBlock,
    },
    Call {
        callee: Box<CoreIrExpr>,
        arguments: Vec<CoreIrExpr>,
    },
    Index {
        value: Box<CoreIrExpr>,
        index: Box<CoreIrExpr>,
    },
    Field {
        value: Box<CoreIrExpr>,
        name: String,
    },
    Unary {
        operator: CoreIrUnaryOp,
        value: Box<CoreIrExpr>,
    },
    Binary {
        left: Box<CoreIrExpr>,
        operator: CoreIrBinaryOp,
        right: Box<CoreIrExpr>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CoreIrNamedExpr {
    pub name: String,
    pub value: CoreIrExpr,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CoreIrUnaryOp {
    Not,
    Negate,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CoreIrBinaryOp {
    Or,
    And,
    Equal,
    NotEqual,
    Less,
    LessEqual,
    Greater,
    GreaterEqual,
    BitOr,
    BitXor,
    BitAnd,
    ShiftLeft,
    ShiftRight,
    Add,
    Subtract,
    Multiply,
    Divide,
    Remainder,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CoreIrMatchArm {
    pub pattern: CoreIrPattern,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub guard: Option<CoreIrExpr>,
    pub value: CoreIrExpr,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "pattern", rename_all = "snake_case")]
pub enum CoreIrPattern {
    Wildcard,
    Bool {
        value: bool,
    },
    Integer {
        value: u64,
    },
    Binding {
        name: String,
    },
    Variant {
        path: Vec<String>,
        fields: Vec<CoreIrPatternField>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CoreIrPatternField {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nested: Option<Box<CoreIrPattern>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CoreIrDiagnostic {
    pub code: String,
    pub message: String,
    pub path: String,
}

impl CoreIrDiagnostic {
    fn new(code: impl Into<String>, message: impl Into<String>, path: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            path: path.into(),
        }
    }
}

/// A raw IR program plus the proof that version, effects, references, types,
/// and control flow passed verification. Future Core backends must accept this
/// wrapper rather than `CoreIrProgram`.
#[derive(Debug, Clone, Copy)]
pub struct VerifiedCoreIr<'a> {
    program: &'a CoreIrProgram,
}

impl<'a> VerifiedCoreIr<'a> {
    pub fn program(self) -> &'a CoreIrProgram {
        self.program
    }

    pub fn semantic_fingerprint(self) -> String {
        let canonical =
            serde_json::to_vec(self.program).expect("Core IR serialization is infallible");
        format!("sha256:{:x}", Sha256::digest(canonical))
    }
}

/// Backend contract: raw or merely deserialized IR cannot cross this boundary.
pub trait CoreIrBackend {
    type Output;
    type Error;

    fn emit(&self, program: VerifiedCoreIr<'_>) -> Result<Self::Output, Self::Error>;
}

pub fn lower_core_program(verified: VerifiedCoreProgram<'_>) -> CoreIrProgram {
    let source = verified.program();
    let mut program = CoreIrProgram {
        ir_version: CORE_IR_VERSION.into(),
        core_version: source.version.value.clone(),
        module: source.module.value.clone(),
        imports: source.imports.iter().map(lower_import).collect(),
        locked_modules: std::iter::once(source.module.value.clone())
            .chain(
                source
                    .imports
                    .iter()
                    .map(|import| import.path.value.clone()),
            )
            .collect(),
        effect_policy: Vec::new(),
        items: source.items.iter().map(lower_item).collect(),
    };
    program.effect_policy = collect_effects(&program).into_iter().collect();
    program
}

/// The canonical IR of a package (see `argorix_semantics::core_package`):
/// the compact JSON of what `argorixc core-emit-ir` emits for its root, and a
/// newline, or `check failed` when the package does not link, check and
/// verify. `compiler/ir.argx` must reproduce it byte for byte (ESP-013.A).
pub fn core_package_ir_dump(files: &[Vec<u8>]) -> String {
    let failed = || "check failed\n".to_string();
    let Ok(package) = core_package(files) else {
        return failed();
    };
    let Ok((_, linked)) = check_core_package(
        &package.root,
        &package.modules,
        &package.duplicates,
        &package.options,
    ) else {
        return failed();
    };
    let Ok(verified) = verify_core_program(&linked, &package.options) else {
        return failed();
    };
    let ir = lower_core_program(verified);
    if verify_core_ir(&ir).is_err() {
        return failed();
    }
    let mut json = serde_json::to_string(&ir).expect("Core IR serialization is infallible");
    json.push('\n');
    json
}

/// What the verifier says of a serialized IR document (ESP-013.B): `decode
/// failed` when it is not an IR document, `ok` when it verifies, or one
/// diagnostic code per line, in the order `verify_core_ir` reports them.
/// `compiler/ir_verify.argx` must reproduce it.
pub fn core_ir_verify_dump(document: &[u8]) -> String {
    let Ok(program) = serde_json::from_slice::<CoreIrProgram>(document) else {
        return "decode failed\n".into();
    };
    match verify_core_ir(&program) {
        Ok(_) => "ok\n".into(),
        Err(diagnostics) => diagnostics
            .iter()
            .map(|diagnostic| format!("{}\n", diagnostic.code))
            .collect(),
    }
}

pub fn verify_core_ir(
    program: &CoreIrProgram,
) -> Result<VerifiedCoreIr<'_>, Vec<CoreIrDiagnostic>> {
    let mut diagnostics = Vec::new();
    if program.ir_version != CORE_IR_VERSION {
        diagnostics.push(CoreIrDiagnostic::new(
            "IrVersionUnsupported",
            format!("unsupported Core IR version `{}`", program.ir_version),
            "ir_version",
        ));
    }
    if program.core_version != "0.1" {
        diagnostics.push(CoreIrDiagnostic::new(
            "CoreVersionUnsupported",
            format!(
                "unsupported Core language version `{}`",
                program.core_version
            ),
            "core_version",
        ));
    }
    let allowed = program
        .effect_policy
        .iter()
        .cloned()
        .collect::<BTreeSet<_>>();
    if allowed.len() != program.effect_policy.len() {
        diagnostics.push(CoreIrDiagnostic::new(
            "DuplicateEffect",
            "effect policy contains duplicate entries",
            "effect_policy",
        ));
    }
    for effect in &allowed {
        if matches!(effect, CoreIrEffect::Host(name) if !CORE_HOST_EFFECTS.contains(&name.as_str()))
        {
            diagnostics.push(CoreIrDiagnostic::new(
                "UnauthorizedEffect",
                "Core IR 0.1 forbids host effects other than package.read and build.write",
                "effect_policy",
            ));
        }
    }
    for effect in collect_effects(program) {
        if !allowed.contains(&effect) {
            diagnostics.push(CoreIrDiagnostic::new(
                "UndeclaredEffect",
                format!("IR uses undeclared effect `{effect:?}`"),
                "effect_policy",
            ));
        }
    }
    let locked = program
        .locked_modules
        .iter()
        .cloned()
        .collect::<BTreeSet<_>>();
    if locked.len() != program.locked_modules.len() {
        diagnostics.push(CoreIrDiagnostic::new(
            "DuplicateLockedModule",
            "locked module set contains duplicates",
            "locked_modules",
        ));
    }
    for import in &program.imports {
        if !locked.contains(&import.path) {
            diagnostics.push(CoreIrDiagnostic::new(
                "ImportNotLocked",
                format!("module `{}` is not locked", import.path),
                "imports",
            ));
        }
    }

    match raise_program(program) {
        Ok(ast) => {
            let options = CoreCheckOptions {
                available_modules: locked,
            };
            if let Err(errors) = verify_core_program(&ast, &options) {
                diagnostics.extend(errors.into_iter().map(|error| CoreIrDiagnostic {
                    code: error.code,
                    message: error.message,
                    path: format!("{}:{}", error.span.line, error.span.column),
                }));
            }
        }
        Err(error) => diagnostics.push(error),
    }
    if diagnostics.is_empty() {
        Ok(VerifiedCoreIr { program })
    } else {
        Err(diagnostics)
    }
}

fn lower_import(value: &CoreImport) -> CoreIrImport {
    CoreIrImport {
        path: value.path.value.clone(),
        alias: value.alias.as_ref().map(|alias| alias.value.clone()),
    }
}

fn lower_item(value: &CoreItem) -> CoreIrItem {
    let kind = match &value.kind {
        CoreItemKind::Function(value) => CoreIrItemKind::Function(CoreIrFunction {
            name: value.name.value.clone(),
            parameters: value
                .parameters
                .iter()
                .map(|parameter| CoreIrParameter {
                    name: parameter.name.value.clone(),
                    ty: lower_type(&parameter.ty),
                })
                .collect(),
            return_type: lower_type(&value.return_type),
            body: lower_block(&value.body),
        }),
        CoreItemKind::Struct(value) => CoreIrItemKind::Struct(CoreIrStruct {
            name: value.name.value.clone(),
            fields: value.fields.iter().map(lower_field).collect(),
        }),
        CoreItemKind::Enum(value) => CoreIrItemKind::Enum(CoreIrEnum {
            name: value.name.value.clone(),
            variants: value
                .variants
                .iter()
                .map(|variant| CoreIrVariant {
                    name: variant.name.value.clone(),
                    fields: variant.fields.iter().map(lower_field).collect(),
                })
                .collect(),
        }),
        CoreItemKind::Const(value) => CoreIrItemKind::Const(CoreIrConst {
            name: value.name.value.clone(),
            ty: lower_type(&value.ty),
            value: lower_expr(&value.value),
        }),
    };
    CoreIrItem {
        public: value.public,
        kind,
    }
}

fn lower_field(value: &CoreField) -> CoreIrField {
    CoreIrField {
        name: value.name.value.clone(),
        ty: lower_type(&value.ty),
    }
}

fn lower_type(value: &CoreType) -> CoreIrType {
    match &value.kind {
        CoreTypeKind::Named(name) => CoreIrType::Named { name: name.clone() },
        CoreTypeKind::Container {
            name,
            element,
            array_length,
        } => CoreIrType::Container {
            name: name.clone(),
            element: Box::new(lower_type(element)),
            array_length: *array_length,
        },
    }
}

fn lower_block(value: &CoreBlock) -> CoreIrBlock {
    CoreIrBlock {
        statements: value.statements.iter().map(lower_statement).collect(),
        tail: value.tail.as_ref().map(|value| Box::new(lower_expr(value))),
    }
}

fn lower_statement(value: &CoreStatement) -> CoreIrStatement {
    match &value.kind {
        CoreStatementKind::Let {
            mutable,
            name,
            annotation,
            value,
        } => CoreIrStatement::Let {
            mutable: *mutable,
            name: name.value.clone(),
            annotation: annotation.as_ref().map(lower_type),
            value: lower_expr(value),
        },
        CoreStatementKind::Assign {
            target,
            operator,
            value,
        } => CoreIrStatement::Assign {
            target: lower_expr(target),
            operator: match operator {
                CoreAssignOp::Assign => CoreIrAssignOp::Assign,
                CoreAssignOp::Add => CoreIrAssignOp::Add,
                CoreAssignOp::Subtract => CoreIrAssignOp::Subtract,
                CoreAssignOp::Multiply => CoreIrAssignOp::Multiply,
                CoreAssignOp::Divide => CoreIrAssignOp::Divide,
                CoreAssignOp::Remainder => CoreIrAssignOp::Remainder,
            },
            value: lower_expr(value),
        },
        CoreStatementKind::While { condition, body } => CoreIrStatement::While {
            condition: lower_expr(condition),
            body: lower_block(body),
        },
        CoreStatementKind::Break(value) => CoreIrStatement::Break {
            value: value.as_ref().map(lower_expr),
        },
        CoreStatementKind::Continue => CoreIrStatement::Continue,
        CoreStatementKind::Return(value) => CoreIrStatement::Return {
            value: value.as_ref().map(lower_expr),
        },
        CoreStatementKind::Expr(value) => CoreIrStatement::Expr {
            value: lower_expr(value),
        },
    }
}

fn lower_expr(value: &CoreExpr) -> CoreIrExpr {
    match &value.kind {
        CoreExprKind::Integer { value, suffix } => CoreIrExpr::Integer {
            value: *value,
            suffix: suffix.clone(),
        },
        CoreExprKind::String(value) => CoreIrExpr::String {
            value: value.clone(),
        },
        CoreExprKind::Bool(value) => CoreIrExpr::Bool { value: *value },
        CoreExprKind::Unit => CoreIrExpr::Unit,
        CoreExprKind::Path(segments) => CoreIrExpr::Path {
            segments: segments.clone(),
        },
        CoreExprKind::Aggregate { path, fields } => CoreIrExpr::Aggregate {
            path: path.clone(),
            fields: fields
                .iter()
                .map(|(name, value)| CoreIrNamedExpr {
                    name: name.value.clone(),
                    value: lower_expr(value),
                })
                .collect(),
        },
        CoreExprKind::Array(values) => CoreIrExpr::Array {
            values: values.iter().map(lower_expr).collect(),
        },
        CoreExprKind::Block(body) => CoreIrExpr::Block {
            body: lower_block(body),
        },
        CoreExprKind::If {
            condition,
            then_block,
            else_expr,
        } => CoreIrExpr::If {
            condition: Box::new(lower_expr(condition)),
            then_block: lower_block(then_block),
            else_expr: else_expr.as_ref().map(|value| Box::new(lower_expr(value))),
        },
        CoreExprKind::Match { value, arms } => CoreIrExpr::Match {
            value: Box::new(lower_expr(value)),
            arms: arms.iter().map(lower_arm).collect(),
        },
        CoreExprKind::Loop(body) => CoreIrExpr::Loop {
            body: lower_block(body),
        },
        CoreExprKind::Call { callee, arguments } => CoreIrExpr::Call {
            callee: Box::new(lower_expr(callee)),
            arguments: arguments.iter().map(lower_expr).collect(),
        },
        CoreExprKind::Index { value, index } => CoreIrExpr::Index {
            value: Box::new(lower_expr(value)),
            index: Box::new(lower_expr(index)),
        },
        CoreExprKind::Field { value, name } => CoreIrExpr::Field {
            value: Box::new(lower_expr(value)),
            name: name.value.clone(),
        },
        CoreExprKind::Unary { operator, value } => CoreIrExpr::Unary {
            operator: match operator {
                CoreUnaryOp::Not => CoreIrUnaryOp::Not,
                CoreUnaryOp::Negate => CoreIrUnaryOp::Negate,
            },
            value: Box::new(lower_expr(value)),
        },
        CoreExprKind::Binary {
            left,
            operator,
            right,
        } => CoreIrExpr::Binary {
            left: Box::new(lower_expr(left)),
            operator: lower_binary(*operator),
            right: Box::new(lower_expr(right)),
        },
    }
}

fn lower_binary(value: CoreBinaryOp) -> CoreIrBinaryOp {
    match value {
        CoreBinaryOp::Or => CoreIrBinaryOp::Or,
        CoreBinaryOp::And => CoreIrBinaryOp::And,
        CoreBinaryOp::Equal => CoreIrBinaryOp::Equal,
        CoreBinaryOp::NotEqual => CoreIrBinaryOp::NotEqual,
        CoreBinaryOp::Less => CoreIrBinaryOp::Less,
        CoreBinaryOp::LessEqual => CoreIrBinaryOp::LessEqual,
        CoreBinaryOp::Greater => CoreIrBinaryOp::Greater,
        CoreBinaryOp::GreaterEqual => CoreIrBinaryOp::GreaterEqual,
        CoreBinaryOp::BitOr => CoreIrBinaryOp::BitOr,
        CoreBinaryOp::BitXor => CoreIrBinaryOp::BitXor,
        CoreBinaryOp::BitAnd => CoreIrBinaryOp::BitAnd,
        CoreBinaryOp::ShiftLeft => CoreIrBinaryOp::ShiftLeft,
        CoreBinaryOp::ShiftRight => CoreIrBinaryOp::ShiftRight,
        CoreBinaryOp::Add => CoreIrBinaryOp::Add,
        CoreBinaryOp::Subtract => CoreIrBinaryOp::Subtract,
        CoreBinaryOp::Multiply => CoreIrBinaryOp::Multiply,
        CoreBinaryOp::Divide => CoreIrBinaryOp::Divide,
        CoreBinaryOp::Remainder => CoreIrBinaryOp::Remainder,
    }
}

fn lower_arm(value: &CoreMatchArm) -> CoreIrMatchArm {
    CoreIrMatchArm {
        pattern: lower_pattern(&value.pattern),
        guard: value.guard.as_ref().map(lower_expr),
        value: lower_expr(&value.value),
    }
}

fn lower_pattern(value: &CorePattern) -> CoreIrPattern {
    match &value.kind {
        CorePatternKind::Wildcard => CoreIrPattern::Wildcard,
        CorePatternKind::Bool(value) => CoreIrPattern::Bool { value: *value },
        CorePatternKind::Integer(value) => CoreIrPattern::Integer { value: *value },
        CorePatternKind::Binding(name) => CoreIrPattern::Binding { name: name.clone() },
        CorePatternKind::Variant { path, fields } => CoreIrPattern::Variant {
            path: path.clone(),
            fields: fields
                .iter()
                .map(|(name, nested)| CoreIrPatternField {
                    name: name.value.clone(),
                    nested: nested.as_ref().map(|value| Box::new(lower_pattern(value))),
                })
                .collect(),
        },
    }
}

/// The named host effects Core IR 0.1 accepts: the two operations of the
/// compiler-host boundary (`spec/core/stdlib.md`). Any other host effect is
/// refused.
pub const CORE_HOST_EFFECTS: [&str; 2] = ["package.read", "build.write"];

/// A host effect comes with the capability that authorizes it, and a
/// capability can only reach a function as a parameter, so the parameters
/// name every host effect the program can use.
fn capability_effect(ty: &CoreIrType) -> Option<CoreIrEffect> {
    match ty {
        CoreIrType::Named { name } if name == "PackageRead" => {
            Some(CoreIrEffect::Host("package.read".into()))
        }
        CoreIrType::Named { name } if name == "BuildWrite" => {
            Some(CoreIrEffect::Host("build.write".into()))
        }
        _ => None,
    }
}

fn collect_effects(program: &CoreIrProgram) -> BTreeSet<CoreIrEffect> {
    let mut effects = BTreeSet::new();
    for item in &program.items {
        match &item.kind {
            CoreIrItemKind::Function(function) => {
                effects.extend(
                    function
                        .parameters
                        .iter()
                        .filter_map(|parameter| capability_effect(&parameter.ty)),
                );
                collect_block_effects(&function.body, &mut effects)
            }
            CoreIrItemKind::Const(value) => collect_expr_effects(&value.value, &mut effects),
            CoreIrItemKind::Struct(_) | CoreIrItemKind::Enum(_) => {}
        }
    }
    effects
}

fn collect_block_effects(block: &CoreIrBlock, effects: &mut BTreeSet<CoreIrEffect>) {
    for statement in &block.statements {
        match statement {
            CoreIrStatement::Let { value, .. }
            | CoreIrStatement::Expr { value }
            | CoreIrStatement::Return { value: Some(value) }
            | CoreIrStatement::Break { value: Some(value) } => collect_expr_effects(value, effects),
            CoreIrStatement::Assign { target, value, .. } => {
                collect_expr_effects(target, effects);
                collect_expr_effects(value, effects);
                if matches!(target, CoreIrExpr::Field { .. } | CoreIrExpr::Index { .. }) {
                    effects.insert(CoreIrEffect::MemoryWrite);
                }
            }
            CoreIrStatement::While { condition, body } => {
                collect_expr_effects(condition, effects);
                collect_block_effects(body, effects);
            }
            CoreIrStatement::Return { value: None }
            | CoreIrStatement::Break { value: None }
            | CoreIrStatement::Continue => {}
        }
    }
    if let Some(tail) = &block.tail {
        collect_expr_effects(tail, effects);
    }
}

fn collect_expr_effects(value: &CoreIrExpr, effects: &mut BTreeSet<CoreIrEffect>) {
    match value {
        CoreIrExpr::Aggregate { fields, .. } => {
            for field in fields {
                collect_expr_effects(&field.value, effects);
            }
        }
        CoreIrExpr::Array { values } => {
            for value in values {
                collect_expr_effects(value, effects);
            }
        }
        CoreIrExpr::Block { body } | CoreIrExpr::Loop { body } => {
            collect_block_effects(body, effects)
        }
        CoreIrExpr::If {
            condition,
            then_block,
            else_expr,
        } => {
            collect_expr_effects(condition, effects);
            collect_block_effects(then_block, effects);
            if let Some(value) = else_expr {
                collect_expr_effects(value, effects);
            }
        }
        CoreIrExpr::Match { value, arms } => {
            collect_expr_effects(value, effects);
            for arm in arms {
                if let Some(guard) = &arm.guard {
                    collect_expr_effects(guard, effects);
                }
                collect_expr_effects(&arm.value, effects);
            }
        }
        CoreIrExpr::Call { callee, arguments } => {
            if let CoreIrExpr::Path { segments } = callee.as_ref() {
                if segments == &["Buffer".to_string(), "new".to_string()]
                    || segments == &["Arena".to_string(), "new".to_string()]
                {
                    effects.insert(CoreIrEffect::Allocate);
                }
            }
            if let CoreIrExpr::Field { name, .. } = callee.as_ref() {
                match name.as_str() {
                    "push" => {
                        effects.insert(CoreIrEffect::MemoryWrite);
                    }
                    "alloc" => {
                        effects.insert(CoreIrEffect::Allocate);
                        effects.insert(CoreIrEffect::MemoryWrite);
                    }
                    "release" => {
                        effects.insert(CoreIrEffect::MemoryWrite);
                    }
                    "decode_utf8_or_trap" => {
                        effects.insert(CoreIrEffect::Trap);
                    }
                    "length" | "as_bytes" => {
                        effects.insert(CoreIrEffect::MemoryRead);
                    }
                    _ => {}
                }
            }
            collect_expr_effects(callee, effects);
            for argument in arguments {
                collect_expr_effects(argument, effects);
            }
        }
        CoreIrExpr::Index { value, index } => {
            effects.insert(CoreIrEffect::MemoryRead);
            effects.insert(CoreIrEffect::Trap);
            collect_expr_effects(value, effects);
            collect_expr_effects(index, effects);
        }
        CoreIrExpr::Field { value, .. } => {
            effects.insert(CoreIrEffect::MemoryRead);
            collect_expr_effects(value, effects);
        }
        CoreIrExpr::Unary { value, .. } => collect_expr_effects(value, effects),
        CoreIrExpr::Binary {
            left,
            operator,
            right,
        } => {
            if matches!(operator, CoreIrBinaryOp::Divide | CoreIrBinaryOp::Remainder) {
                effects.insert(CoreIrEffect::Trap);
            }
            collect_expr_effects(left, effects);
            collect_expr_effects(right, effects);
        }
        CoreIrExpr::Integer { .. }
        | CoreIrExpr::String { .. }
        | CoreIrExpr::Bool { .. }
        | CoreIrExpr::Unit
        | CoreIrExpr::Path { .. } => {}
    }
}

fn synthetic_span() -> Span {
    Span::new(0, 0, 1, 1)
}

fn spanned<T>(value: T) -> Spanned<T> {
    Spanned::new(value, synthetic_span())
}

fn raise_program(value: &CoreIrProgram) -> Result<CoreProgram, CoreIrDiagnostic> {
    if value.module.trim().is_empty() {
        return Err(CoreIrDiagnostic::new(
            "InvalidModule",
            "module cannot be empty",
            "module",
        ));
    }
    Ok(CoreProgram {
        version: spanned(value.core_version.clone()),
        module: spanned(value.module.clone()),
        imports: value.imports.iter().map(raise_import).collect(),
        items: value.items.iter().map(raise_item).collect(),
        span: synthetic_span(),
    })
}

fn raise_import(value: &CoreIrImport) -> CoreImport {
    CoreImport {
        path: spanned(value.path.clone()),
        alias: value.alias.clone().map(spanned),
        span: synthetic_span(),
    }
}

fn raise_item(value: &CoreIrItem) -> CoreItem {
    let kind = match &value.kind {
        CoreIrItemKind::Function(value) => CoreItemKind::Function(CoreFunction {
            name: spanned(value.name.clone()),
            parameters: value
                .parameters
                .iter()
                .map(|parameter| CoreParameter {
                    name: spanned(parameter.name.clone()),
                    ty: raise_type(&parameter.ty),
                    span: synthetic_span(),
                })
                .collect(),
            return_type: raise_type(&value.return_type),
            body: raise_block(&value.body),
        }),
        CoreIrItemKind::Struct(value) => CoreItemKind::Struct(CoreStruct {
            name: spanned(value.name.clone()),
            fields: value.fields.iter().map(raise_field).collect(),
        }),
        CoreIrItemKind::Enum(value) => CoreItemKind::Enum(CoreEnum {
            name: spanned(value.name.clone()),
            variants: value
                .variants
                .iter()
                .map(|variant| CoreVariant {
                    name: spanned(variant.name.clone()),
                    fields: variant.fields.iter().map(raise_field).collect(),
                    span: synthetic_span(),
                })
                .collect(),
        }),
        CoreIrItemKind::Const(value) => CoreItemKind::Const(CoreConst {
            name: spanned(value.name.clone()),
            ty: raise_type(&value.ty),
            value: raise_expr(&value.value),
        }),
    };
    CoreItem {
        public: value.public,
        kind,
        span: synthetic_span(),
    }
}

fn raise_field(value: &CoreIrField) -> CoreField {
    CoreField {
        name: spanned(value.name.clone()),
        ty: raise_type(&value.ty),
        span: synthetic_span(),
    }
}

fn raise_type(value: &CoreIrType) -> CoreType {
    let kind = match value {
        CoreIrType::Named { name } => CoreTypeKind::Named(name.clone()),
        CoreIrType::Container {
            name,
            element,
            array_length,
        } => CoreTypeKind::Container {
            name: name.clone(),
            element: Box::new(raise_type(element)),
            array_length: *array_length,
        },
    };
    CoreType {
        kind,
        span: synthetic_span(),
    }
}

fn raise_block(value: &CoreIrBlock) -> CoreBlock {
    CoreBlock {
        statements: value.statements.iter().map(raise_statement).collect(),
        tail: value.tail.as_ref().map(|value| Box::new(raise_expr(value))),
        span: synthetic_span(),
    }
}

fn raise_statement(value: &CoreIrStatement) -> CoreStatement {
    let kind = match value {
        CoreIrStatement::Let {
            mutable,
            name,
            annotation,
            value,
        } => CoreStatementKind::Let {
            mutable: *mutable,
            name: spanned(name.clone()),
            annotation: annotation.as_ref().map(raise_type),
            value: raise_expr(value),
        },
        CoreIrStatement::Assign {
            target,
            operator,
            value,
        } => CoreStatementKind::Assign {
            target: raise_expr(target),
            operator: match operator {
                CoreIrAssignOp::Assign => CoreAssignOp::Assign,
                CoreIrAssignOp::Add => CoreAssignOp::Add,
                CoreIrAssignOp::Subtract => CoreAssignOp::Subtract,
                CoreIrAssignOp::Multiply => CoreAssignOp::Multiply,
                CoreIrAssignOp::Divide => CoreAssignOp::Divide,
                CoreIrAssignOp::Remainder => CoreAssignOp::Remainder,
            },
            value: raise_expr(value),
        },
        CoreIrStatement::While { condition, body } => CoreStatementKind::While {
            condition: raise_expr(condition),
            body: raise_block(body),
        },
        CoreIrStatement::Break { value } => {
            CoreStatementKind::Break(value.as_ref().map(raise_expr))
        }
        CoreIrStatement::Continue => CoreStatementKind::Continue,
        CoreIrStatement::Return { value } => {
            CoreStatementKind::Return(value.as_ref().map(raise_expr))
        }
        CoreIrStatement::Expr { value } => CoreStatementKind::Expr(raise_expr(value)),
    };
    CoreStatement {
        kind,
        span: synthetic_span(),
    }
}

fn raise_expr(value: &CoreIrExpr) -> CoreExpr {
    let kind = match value {
        CoreIrExpr::Integer { value, suffix } => CoreExprKind::Integer {
            value: *value,
            suffix: suffix.clone(),
        },
        CoreIrExpr::String { value } => CoreExprKind::String(value.clone()),
        CoreIrExpr::Bool { value } => CoreExprKind::Bool(*value),
        CoreIrExpr::Unit => CoreExprKind::Unit,
        CoreIrExpr::Path { segments } => CoreExprKind::Path(segments.clone()),
        CoreIrExpr::Aggregate { path, fields } => CoreExprKind::Aggregate {
            path: path.clone(),
            fields: fields
                .iter()
                .map(|field| (spanned(field.name.clone()), raise_expr(&field.value)))
                .collect(),
        },
        CoreIrExpr::Array { values } => {
            CoreExprKind::Array(values.iter().map(raise_expr).collect())
        }
        CoreIrExpr::Block { body } => CoreExprKind::Block(raise_block(body)),
        CoreIrExpr::If {
            condition,
            then_block,
            else_expr,
        } => CoreExprKind::If {
            condition: Box::new(raise_expr(condition)),
            then_block: raise_block(then_block),
            else_expr: else_expr.as_ref().map(|value| Box::new(raise_expr(value))),
        },
        CoreIrExpr::Match { value, arms } => CoreExprKind::Match {
            value: Box::new(raise_expr(value)),
            arms: arms.iter().map(raise_arm).collect(),
        },
        CoreIrExpr::Loop { body } => CoreExprKind::Loop(raise_block(body)),
        CoreIrExpr::Call { callee, arguments } => CoreExprKind::Call {
            callee: Box::new(raise_expr(callee)),
            arguments: arguments.iter().map(raise_expr).collect(),
        },
        CoreIrExpr::Index { value, index } => CoreExprKind::Index {
            value: Box::new(raise_expr(value)),
            index: Box::new(raise_expr(index)),
        },
        CoreIrExpr::Field { value, name } => CoreExprKind::Field {
            value: Box::new(raise_expr(value)),
            name: spanned(name.clone()),
        },
        CoreIrExpr::Unary { operator, value } => CoreExprKind::Unary {
            operator: match operator {
                CoreIrUnaryOp::Not => CoreUnaryOp::Not,
                CoreIrUnaryOp::Negate => CoreUnaryOp::Negate,
            },
            value: Box::new(raise_expr(value)),
        },
        CoreIrExpr::Binary {
            left,
            operator,
            right,
        } => CoreExprKind::Binary {
            left: Box::new(raise_expr(left)),
            operator: raise_binary(*operator),
            right: Box::new(raise_expr(right)),
        },
    };
    CoreExpr {
        kind,
        span: synthetic_span(),
    }
}

fn raise_binary(value: CoreIrBinaryOp) -> CoreBinaryOp {
    match value {
        CoreIrBinaryOp::Or => CoreBinaryOp::Or,
        CoreIrBinaryOp::And => CoreBinaryOp::And,
        CoreIrBinaryOp::Equal => CoreBinaryOp::Equal,
        CoreIrBinaryOp::NotEqual => CoreBinaryOp::NotEqual,
        CoreIrBinaryOp::Less => CoreBinaryOp::Less,
        CoreIrBinaryOp::LessEqual => CoreBinaryOp::LessEqual,
        CoreIrBinaryOp::Greater => CoreBinaryOp::Greater,
        CoreIrBinaryOp::GreaterEqual => CoreBinaryOp::GreaterEqual,
        CoreIrBinaryOp::BitOr => CoreBinaryOp::BitOr,
        CoreIrBinaryOp::BitXor => CoreBinaryOp::BitXor,
        CoreIrBinaryOp::BitAnd => CoreBinaryOp::BitAnd,
        CoreIrBinaryOp::ShiftLeft => CoreBinaryOp::ShiftLeft,
        CoreIrBinaryOp::ShiftRight => CoreBinaryOp::ShiftRight,
        CoreIrBinaryOp::Add => CoreBinaryOp::Add,
        CoreIrBinaryOp::Subtract => CoreBinaryOp::Subtract,
        CoreIrBinaryOp::Multiply => CoreBinaryOp::Multiply,
        CoreIrBinaryOp::Divide => CoreBinaryOp::Divide,
        CoreIrBinaryOp::Remainder => CoreBinaryOp::Remainder,
    }
}

fn raise_arm(value: &CoreIrMatchArm) -> CoreMatchArm {
    CoreMatchArm {
        pattern: raise_pattern(&value.pattern),
        guard: value.guard.as_ref().map(raise_expr),
        value: raise_expr(&value.value),
        span: synthetic_span(),
    }
}

fn raise_pattern(value: &CoreIrPattern) -> CorePattern {
    let kind = match value {
        CoreIrPattern::Wildcard => CorePatternKind::Wildcard,
        CoreIrPattern::Bool { value } => CorePatternKind::Bool(*value),
        CoreIrPattern::Integer { value } => CorePatternKind::Integer(*value),
        CoreIrPattern::Binding { name } => CorePatternKind::Binding(name.clone()),
        CoreIrPattern::Variant { path, fields } => CorePatternKind::Variant {
            path: path.clone(),
            fields: fields
                .iter()
                .map(|field| {
                    (
                        spanned(field.name.clone()),
                        field
                            .nested
                            .as_ref()
                            .map(|value| Box::new(raise_pattern(value))),
                    )
                })
                .collect(),
        },
    };
    CorePattern {
        kind,
        span: synthetic_span(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use argorix_parser::core::parse_core_source;

    const SIMPLE: &str = r#"
core 0.1;
module test.simple;
fn add(left: i32, right: i32) -> i32 { left + right }
"#;

    fn lowered() -> CoreIrProgram {
        let program = parse_core_source(SIMPLE).unwrap();
        let options = CoreCheckOptions {
            available_modules: BTreeSet::from(["test.simple".into()]),
        };
        lower_core_program(verify_core_program(&program, &options).unwrap())
    }

    #[test]
    fn verified_roundtrip_preserves_semantic_fingerprint() {
        let ir = lowered();
        let before = verify_core_ir(&ir).unwrap().semantic_fingerprint();
        let json = serde_json::to_string_pretty(&ir).unwrap();
        let decoded: CoreIrProgram = serde_json::from_str(&json).unwrap();
        let after = verify_core_ir(&decoded).unwrap().semantic_fingerprint();
        assert_eq!(before, after);
    }

    #[test]
    fn rejects_unknown_function_reference() {
        let mut ir = lowered();
        let CoreIrItemKind::Function(function) = &mut ir.items[0].kind else {
            panic!()
        };
        function.body.tail = Some(Box::new(CoreIrExpr::Call {
            callee: Box::new(CoreIrExpr::Path {
                segments: vec!["missing".into()],
            }),
            arguments: vec![],
        }));
        let errors = verify_core_ir(&ir).unwrap_err();
        assert!(errors.iter().any(|error| error.code == "TypeMismatch"
            || error.code == "ImmutableAssignmentOrUnknownName"));
    }

    #[test]
    fn rejects_unauthorized_host_effect() {
        let mut ir = lowered();
        ir.effect_policy
            .push(CoreIrEffect::Host("process.spawn".into()));
        let errors = verify_core_ir(&ir).unwrap_err();
        assert!(errors
            .iter()
            .any(|error| error.code == "UnauthorizedEffect"));
    }

    #[test]
    fn rejects_invalid_control_flow_and_type() {
        let mut ir = lowered();
        let CoreIrItemKind::Function(function) = &mut ir.items[0].kind else {
            panic!()
        };
        function
            .body
            .statements
            .push(CoreIrStatement::Break { value: None });
        function.body.tail = Some(Box::new(CoreIrExpr::Bool { value: true }));
        let errors = verify_core_ir(&ir).unwrap_err();
        assert!(errors
            .iter()
            .any(|error| error.code == "ControlOutsideLoop"));
        assert!(errors.iter().any(|error| error.code == "TypeMismatch"));
    }
}
