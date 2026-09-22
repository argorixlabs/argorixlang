//! Random Core 0.1 programs with their spec-derived expected results.
//!
//! The oracle below evaluates the generated AST by the rules of
//! `spec/core/evaluation.md` and `spec/core/types.md`: exact-width arithmetic
//! that traps on overflow, trapping division by zero and `MIN / -1`,
//! truncating `/` and `%`, strict left-to-right evaluation, and
//! short-circuiting `&&` and `||`. It never consults `argorixc`, the C backend
//! or the C runtime, so the expectation is independent of the implementation
//! under test, as the master plan requires.
//!
//! The generator stays inside the subset the backend claims to support and
//! avoids every shape recorded in `conformance/core_c/gaps/`, so a failure is
//! a real divergence rather than a known gap.

use std::collections::BTreeSet;
use std::path::Path;

use anyhow::Result;
use serde::Serialize;

use crate::harness::Case;

pub const MAX_LOOP_ITERATIONS: u32 = 10_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IntType {
    pub name: &'static str,
    pub low: i128,
    pub high: i128,
}

pub const TYPES: [IntType; 8] = [
    IntType {
        name: "u8",
        low: 0,
        high: 255,
    },
    IntType {
        name: "u16",
        low: 0,
        high: 65_535,
    },
    IntType {
        name: "u32",
        low: 0,
        high: 4_294_967_295,
    },
    IntType {
        name: "u64",
        low: 0,
        high: 18_446_744_073_709_551_615,
    },
    IntType {
        name: "i8",
        low: -128,
        high: 127,
    },
    IntType {
        name: "i16",
        low: -32_768,
        high: 32_767,
    },
    IntType {
        name: "i32",
        low: -2_147_483_648,
        high: 2_147_483_647,
    },
    IntType {
        name: "i64",
        low: -9_223_372_036_854_775_808,
        high: 9_223_372_036_854_775_807,
    },
];

pub fn unsigned(kind: IntType) -> bool {
    kind.name.starts_with('u')
}

/// The exact width of the type, taken from its own name: `u8` is 8 bits.
pub fn width(kind: IntType) -> u32 {
    kind.name[1..]
        .parse()
        .expect("every declared type name ends in its width")
}

/// Fit a value into the type's two's-complement representation.
///
/// Only the shifts use this: `spec/core/evaluation.md` gives them a single
/// rule, the amount below the width, and keeps the modular forms for the
/// explicit `wrapping_*` intrinsics. Bits that leave the width are therefore
/// dropped, as they are in the `checked_shl` of the language Core's literals
/// follow, and no second overflow rule is invented here.
fn truncate(value: i128, kind: IntType) -> i128 {
    let bits = width(kind);
    let modulus = 1i128 << bits;
    let wrapped = value.rem_euclid(modulus);
    if unsigned(kind) || wrapped <= kind.high {
        wrapped
    } else {
        wrapped - modulus
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Trap {
    IntegerOverflow,
    DivisionByZero,
    IndexOutOfBounds,
    ArenaReleased,
    ShiftOutOfRange,
}

impl Trap {
    pub fn code(&self) -> &'static str {
        match self {
            Trap::IntegerOverflow => "INTEGER_OVERFLOW",
            Trap::DivisionByZero => "DIVISION_BY_ZERO",
            Trap::IndexOutOfBounds => "INDEX_OUT_OF_BOUNDS",
            Trap::ArenaReleased => "ARENA_RELEASED",
            Trap::ShiftOutOfRange => "SHIFT_OUT_OF_RANGE",
        }
    }
}

/// Why a generated program cannot become a case.
#[derive(Debug)]
pub enum Unusable {
    DoesNotFinish,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Expr {
    Literal(i128),
    Var(String),
    Arith(&'static str, Box<Expr>, Box<Expr>),
    /// `left << right` or `left >> right`. Both operands carry the program's
    /// integer type, as the frontend requires, and an amount that reaches the
    /// width traps.
    Shift(&'static str, Box<Expr>, Box<Expr>),
    /// `-value`, generated only for a signed program: the backend has no
    /// representable result for an unsigned one.
    Negate(Box<Expr>),
    Compare(&'static str, Box<Expr>, Box<Expr>),
    And(Box<Expr>, Box<Expr>),
    Or(Box<Expr>, Box<Expr>),
    Not(Box<Expr>),
    If(Box<Expr>, Box<Expr>, Box<Expr>),
    Call(String, Vec<Expr>),
    /// `array[index]`, where both are locals. The index is a `u64` local, and
    /// it may be out of range, which must trap.
    Index {
        array: String,
        index: String,
    },
    /// `value.fN` on a flat struct local.
    Field {
        value: String,
        field: usize,
    },
    /// `handle.fN`. Reading a handle whose arena was released must trap.
    HandleField {
        handle: String,
        field: usize,
    },
    /// `{ let ..; tail }` used as a value. Its locals belong to the block
    /// alone and may shadow an outer name, which was gap g04.
    Block {
        body: Vec<Stmt>,
        tail: Box<Expr>,
    },
    /// `readN(local)`: the generated reader function whose body is an
    /// exhaustive `match` over the enum this local holds.
    ReadEnum {
        declaration: usize,
        reader: String,
        value: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Stmt {
    Let {
        name: String,
        mutable: bool,
        value: Expr,
    },
    Assign {
        name: String,
        value: Expr,
    },
    Compound {
        name: String,
        op: &'static str,
        value: Expr,
    },
    While {
        condition: Expr,
        body: Vec<Stmt>,
    },
    /// `if condition { .. } else { .. }` used as a statement, with no value
    /// and no `else` required. It was gap g12.
    IfStatement {
        condition: Expr,
        then_body: Vec<Stmt>,
        else_body: Option<Vec<Stmt>>,
    },
    /// `let name: Array<T, N> = [..];`, declared at function level only:
    /// an array declared inside an `if` or a loop body hits gap g06.
    LetArray {
        name: String,
        elements: Vec<Expr>,
    },
    /// `let name: u64 = K;`, the only `u64` in a program of another width,
    /// used to index an array. It may point past the end.
    LetIndex {
        name: String,
        value: u64,
    },
    /// `let name: SN = SN { f0: .., f1: .. };` for a flat struct.
    LetStruct {
        name: String,
        type_name: String,
        fields: Vec<Expr>,
    },
    /// `let mut name: Buffer<T> = Buffer::new();`
    LetBuffer {
        name: String,
    },
    /// `name.push(value);`
    Push {
        name: String,
        value: Expr,
    },
    /// `let mut name: Arena<SN> = Arena::new();`
    LetArena {
        name: String,
        type_name: String,
    },
    /// `let handle: Handle<SN> = arena.alloc(SN { .. });`
    Alloc {
        handle: String,
        arena: String,
        type_name: String,
        fields: Vec<Expr>,
    },
    /// `arena.release();`, after which its handles must trap.
    Release {
        arena: String,
    },
    /// `let name: EN = EN::Variant { p0: .. };`
    LetEnum {
        name: String,
        type_name: String,
        variant: usize,
        payload: Option<Expr>,
    },
}

/// An enum, its variants, and the function that reads one back.
///
/// Each variant carries at most one field, and the reader's `match` lists
/// every variant, since Core requires exhaustiveness.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnumDecl {
    pub name: String,
    pub reader: String,
    /// One entry per variant: `Some(fallback)` for a fieldless variant, which
    /// the arm answers with that literal, or `None` when it carries a field.
    pub variants: Vec<Option<i128>>,
}

#[derive(Debug, Clone)]
pub struct Function {
    pub name: String,
    pub params: Vec<String>,
    pub body: Vec<Stmt>,
    pub tail: Expr,
}

#[derive(Debug, Clone)]
pub struct Program {
    pub id: String,
    pub kind: IntType,
    /// Flat struct declarations, as (name, field count). Structs holding
    /// structs, and arrays of structs, are gaps g02 and g03.
    pub structs: Vec<(String, usize)>,
    pub enums: Vec<EnumDecl>,
    pub functions: Vec<Function>,
    pub main: Function,
}

// --------------------------------------------------------------------------
// Oracle

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Value {
    Int(i128),
    Bool(bool),
}

impl Value {
    fn int(self) -> i128 {
        match self {
            Value::Int(value) => value,
            Value::Bool(value) => value as i128,
        }
    }
    fn truthy(self) -> bool {
        match self {
            Value::Bool(value) => value,
            Value::Int(value) => value != 0,
        }
    }
}

pub enum Failure {
    Trapped(Trap),
    Unusable(Unusable),
}

fn checked(value: i128, kind: IntType) -> Result<i128, Failure> {
    if value < kind.low || value > kind.high {
        return Err(Failure::Trapped(Trap::IntegerOverflow));
    }
    Ok(value)
}

pub fn arithmetic(op: &str, left: i128, right: i128, kind: IntType) -> Result<i128, Failure> {
    match op {
        "+" => checked(left + right, kind),
        "-" => checked(left - right, kind),
        "*" => checked(left * right, kind),
        "/" | "%" => {
            if right == 0 {
                return Err(Failure::Trapped(Trap::DivisionByZero));
            }
            if left == kind.low && right == -1 {
                return Err(Failure::Trapped(Trap::IntegerOverflow));
            }
            // C-style truncation toward zero, as the spec requires.
            let quotient =
                (left.abs() / right.abs()) * if (left < 0) != (right < 0) { -1 } else { 1 };
            checked(
                if op == "/" {
                    quotient
                } else {
                    left - quotient * right
                },
                kind,
            )
        }
        "&" => Ok(left & right),
        "|" => Ok(left | right),
        "^" => Ok(left ^ right),
        other => panic!("unsupported operator {other}"),
    }
}

/// `left << right` and `left >> right` by the rules of
/// `spec/core/evaluation.md`: the amount must be below the width, and the
/// signed right shift keeps the sign.
pub fn shift(op: &str, left: i128, right: i128, kind: IntType) -> Result<i128, Failure> {
    if right < 0 || right >= i128::from(width(kind)) {
        return Err(Failure::Trapped(Trap::ShiftOutOfRange));
    }
    let amount = right as u32;
    match op {
        "<<" => Ok(truncate(left << amount, kind)),
        ">>" => Ok(left >> amount),
        other => panic!("unsupported shift {other}"),
    }
}

/// `-value`. The minimum of a signed type has no positive counterpart, so it
/// is the one input that overflows.
pub fn negate(value: i128, kind: IntType) -> Result<i128, Failure> {
    if value == kind.low {
        return Err(Failure::Trapped(Trap::IntegerOverflow));
    }
    Ok(-value)
}

#[derive(Default)]
struct Environment {
    values: Vec<(String, i128)>,
    /// Array and struct locals, each held as its element or field values.
    aggregates: Vec<(String, Vec<i128>)>,
    /// `u64` index locals, kept apart because they are not of the program's
    /// integer type.
    indexes: Vec<(String, u64)>,
    /// Enum locals, as (name, variant index, payload when the variant has one).
    enums: Vec<(String, usize, Option<i128>)>,
    /// Arenas by name, with whether they have been released.
    arenas: Vec<(String, bool)>,
    /// Handles, as (name, owning arena, field values).
    handles: Vec<(String, String, Vec<i128>)>,
}

/// The lengths of every binding list, so a block can drop exactly what it
/// declared and leave assignments to outer locals in place.
#[derive(Clone, Copy)]
struct Scope {
    values: usize,
    aggregates: usize,
    indexes: usize,
    enums: usize,
    arenas: usize,
    handles: usize,
}

impl Environment {
    fn mark(&self) -> Scope {
        Scope {
            values: self.values.len(),
            aggregates: self.aggregates.len(),
            indexes: self.indexes.len(),
            enums: self.enums.len(),
            arenas: self.arenas.len(),
            handles: self.handles.len(),
        }
    }
    fn restore(&mut self, scope: Scope) {
        self.values.truncate(scope.values);
        self.aggregates.truncate(scope.aggregates);
        self.indexes.truncate(scope.indexes);
        self.enums.truncate(scope.enums);
        self.arenas.truncate(scope.arenas);
        self.handles.truncate(scope.handles);
    }
    /// `let name = ..`: always a new binding, so an inner one shadows an
    /// outer of the same name instead of overwriting it.
    fn declare(&mut self, name: &str, value: i128) {
        self.values.push((name.to_string(), value));
    }
    fn get(&self, name: &str) -> i128 {
        self.values
            .iter()
            .rev()
            .find(|(key, _)| key == name)
            .map(|(_, value)| *value)
            .unwrap_or(0)
    }
    fn set(&mut self, name: &str, value: i128) {
        if let Some(entry) = self.values.iter_mut().rev().find(|(key, _)| key == name) {
            entry.1 = value;
        } else {
            self.values.push((name.to_string(), value));
        }
    }
    fn aggregate(&self, name: &str) -> &[i128] {
        self.aggregates
            .iter()
            .rev()
            .find(|(key, _)| key == name)
            .map(|(_, items)| items.as_slice())
            .unwrap_or(&[])
    }
    fn index(&self, name: &str) -> u64 {
        self.indexes
            .iter()
            .rev()
            .find(|(key, _)| key == name)
            .map(|(_, value)| *value)
            .unwrap_or(0)
    }
    fn push(&mut self, name: &str, value: i128) {
        if let Some(entry) = self
            .aggregates
            .iter_mut()
            .rev()
            .find(|(key, _)| key == name)
        {
            entry.1.push(value);
        }
    }
    /// The field values behind a handle, or `None` once its arena is released.
    fn handle(&self, name: &str) -> Option<&[i128]> {
        let (_, arena, fields) = self.handles.iter().rev().find(|(key, _, _)| key == name)?;
        let released = self
            .arenas
            .iter()
            .rev()
            .find(|(key, _)| key == arena)
            .map(|(_, released)| *released)
            .unwrap_or(false);
        (!released).then_some(fields.as_slice())
    }
}

pub fn evaluate_program(program: &Program) -> Result<i128, Failure> {
    let mut fuel = MAX_LOOP_ITERATIONS;
    let mut env = Environment::default();
    let value = run_body(
        &program.main.body,
        &program.main.tail,
        &mut env,
        program,
        &mut fuel,
    )?;
    Ok(value.int())
}

fn run_body(
    body: &[Stmt],
    tail: &Expr,
    env: &mut Environment,
    program: &Program,
    fuel: &mut u32,
) -> Result<Value, Failure> {
    for statement in body {
        execute(statement, env, program, fuel)?;
    }
    evaluate(tail, env, program, fuel)
}

fn execute(
    statement: &Stmt,
    env: &mut Environment,
    program: &Program,
    fuel: &mut u32,
) -> Result<(), Failure> {
    match statement {
        Stmt::Let { name, value, .. } => {
            let computed = evaluate(value, env, program, fuel)?.int();
            env.declare(name, computed);
        }
        Stmt::Assign { name, value } => {
            let computed = evaluate(value, env, program, fuel)?.int();
            env.set(name, computed);
        }
        Stmt::Compound { name, op, value } => {
            let right = evaluate(value, env, program, fuel)?.int();
            let computed = arithmetic(op, env.get(name), right, program.kind)?;
            env.set(name, computed);
        }
        Stmt::LetArray { name, elements } => {
            let mut values = Vec::with_capacity(elements.len());
            for element in elements {
                values.push(evaluate(element, env, program, fuel)?.int());
            }
            env.aggregates.push((name.clone(), values));
        }
        Stmt::LetStruct { name, fields, .. } => {
            let mut values = Vec::with_capacity(fields.len());
            for field in fields {
                values.push(evaluate(field, env, program, fuel)?.int());
            }
            env.aggregates.push((name.clone(), values));
        }
        Stmt::LetIndex { name, value } => env.indexes.push((name.clone(), *value)),
        Stmt::LetBuffer { name } => env.aggregates.push((name.clone(), Vec::new())),
        Stmt::Push { name, value } => {
            let computed = evaluate(value, env, program, fuel)?.int();
            env.push(name, computed);
        }
        Stmt::LetArena { name, .. } => env.arenas.push((name.clone(), false)),
        Stmt::Alloc {
            handle,
            arena,
            fields,
            ..
        } => {
            let mut values = Vec::with_capacity(fields.len());
            for field in fields {
                values.push(evaluate(field, env, program, fuel)?.int());
            }
            env.handles.push((handle.clone(), arena.clone(), values));
        }
        Stmt::Release { arena } => {
            if let Some(entry) = env.arenas.iter_mut().rev().find(|(key, _)| key == arena) {
                entry.1 = true;
            }
        }
        Stmt::LetEnum {
            name,
            variant,
            payload,
            ..
        } => {
            let carried = match payload {
                Some(expression) => Some(evaluate(expression, env, program, fuel)?.int()),
                None => None,
            };
            env.enums.push((name.clone(), *variant, carried));
        }
        Stmt::IfStatement {
            condition,
            then_body,
            else_body,
        } => {
            let taken = evaluate(condition, env, program, fuel)?.truthy();
            let branch = if taken {
                Some(then_body)
            } else {
                else_body.as_ref()
            };
            if let Some(branch) = branch {
                // The branch has its own scope; what it assigns to an outer
                // local stays assigned.
                let scope = env.mark();
                for inner in branch {
                    execute(inner, env, program, fuel)?;
                }
                env.restore(scope);
            }
        }
        Stmt::While { condition, body } => {
            while evaluate(condition, env, program, fuel)?.truthy() {
                if *fuel == 0 {
                    return Err(Failure::Unusable(Unusable::DoesNotFinish));
                }
                *fuel -= 1;
                for inner in body {
                    execute(inner, env, program, fuel)?;
                }
            }
        }
    }
    Ok(())
}

fn evaluate(
    expr: &Expr,
    env: &mut Environment,
    program: &Program,
    fuel: &mut u32,
) -> Result<Value, Failure> {
    Ok(match expr {
        Expr::Literal(value) => Value::Int(*value),
        Expr::Var(name) => Value::Int(env.get(name)),
        Expr::Not(inner) => Value::Bool(!evaluate(inner, env, program, fuel)?.truthy()),
        Expr::And(left, right) => {
            // Short-circuit: the right side is not evaluated when the left is false.
            if evaluate(left, env, program, fuel)?.truthy() {
                Value::Bool(evaluate(right, env, program, fuel)?.truthy())
            } else {
                Value::Bool(false)
            }
        }
        Expr::Or(left, right) => {
            if evaluate(left, env, program, fuel)?.truthy() {
                Value::Bool(true)
            } else {
                Value::Bool(evaluate(right, env, program, fuel)?.truthy())
            }
        }
        Expr::Compare(op, left, right) => {
            // Operands run strictly left to right.
            let left = evaluate(left, env, program, fuel)?.int();
            let right = evaluate(right, env, program, fuel)?.int();
            Value::Bool(match *op {
                "==" => left == right,
                "!=" => left != right,
                "<" => left < right,
                "<=" => left <= right,
                ">" => left > right,
                _ => left >= right,
            })
        }
        Expr::Arith(op, left, right) => {
            let left = evaluate(left, env, program, fuel)?.int();
            let right = evaluate(right, env, program, fuel)?.int();
            Value::Int(arithmetic(op, left, right, program.kind)?)
        }
        Expr::Shift(op, left, right) => {
            let left = evaluate(left, env, program, fuel)?.int();
            let right = evaluate(right, env, program, fuel)?.int();
            Value::Int(shift(op, left, right, program.kind)?)
        }
        Expr::Negate(inner) => {
            let value = evaluate(inner, env, program, fuel)?.int();
            Value::Int(negate(value, program.kind)?)
        }
        Expr::Block { body, tail } => {
            let scope = env.mark();
            for statement in body {
                execute(statement, env, program, fuel)?;
            }
            let value = evaluate(tail, env, program, fuel)?;
            env.restore(scope);
            value
        }
        Expr::If(condition, then_branch, else_branch) => {
            let branch = if evaluate(condition, env, program, fuel)?.truthy() {
                then_branch
            } else {
                else_branch
            };
            evaluate(branch, env, program, fuel)?
        }
        Expr::Index { array, index } => {
            let items = env.aggregate(array);
            let position = env.index(index);
            // Reading past the end is a typed trap, never a wrong value.
            match items.get(position as usize) {
                Some(value) => Value::Int(*value),
                None => return Err(Failure::Trapped(Trap::IndexOutOfBounds)),
            }
        }
        Expr::Field { value, field } => Value::Int(
            env.aggregate(value)
                .get(*field)
                .copied()
                .expect("a generated field index is in range"),
        ),
        Expr::ReadEnum {
            declaration, value, ..
        } => {
            let (_, variant, payload) = env
                .enums
                .iter()
                .rev()
                .find(|(key, _, _)| key == value)
                .expect("a generated enum local exists before it is read");
            // The arm that matches the variant decides the answer: its own
            // field, or the fallback literal of a fieldless variant.
            match program.enums[*declaration].variants[*variant] {
                Some(fallback) => Value::Int(fallback),
                None => Value::Int(payload.expect("a variant with a field carries one")),
            }
        }
        Expr::HandleField { handle, field } => match env.handle(handle) {
            // Reading through a handle whose arena is gone is a typed trap.
            None => return Err(Failure::Trapped(Trap::ArenaReleased)),
            Some(fields) => Value::Int(
                fields
                    .get(*field)
                    .copied()
                    .expect("a generated field index is in range"),
            ),
        },
        Expr::Call(name, arguments) => {
            let function = program
                .functions
                .iter()
                .find(|item| &item.name == name)
                .expect("generated call targets a generated function");
            let mut values = Vec::new();
            for argument in arguments {
                values.push(evaluate(argument, env, program, fuel)?.int());
            }
            let mut local = Environment::default();
            for (parameter, value) in function.params.iter().zip(values) {
                local.set(parameter, value);
            }
            run_body(&function.body, &function.tail, &mut local, program, fuel)?
        }
    })
}

// --------------------------------------------------------------------------
// Rendering

pub fn render_expr(expr: &Expr, kind: IntType) -> String {
    match expr {
        Expr::Literal(value) => render_literal(*value, kind),
        Expr::Var(name) => name.clone(),
        Expr::Not(inner) => format!("!{}", render_expr(inner, kind)),
        Expr::And(left, right) => {
            format!(
                "({} && {})",
                render_expr(left, kind),
                render_expr(right, kind)
            )
        }
        Expr::Or(left, right) => {
            format!(
                "({} || {})",
                render_expr(left, kind),
                render_expr(right, kind)
            )
        }
        Expr::Negate(inner) => format!("-({})", render_expr(inner, kind)),
        Expr::Compare(op, left, right)
        | Expr::Arith(op, left, right)
        | Expr::Shift(op, left, right) => {
            format!(
                "({} {op} {})",
                render_expr(left, kind),
                render_expr(right, kind)
            )
        }
        Expr::If(condition, then_branch, else_branch) => format!(
            "(if {} {{ {} }} else {{ {} }})",
            render_expr(condition, kind),
            render_expr(then_branch, kind),
            render_expr(else_branch, kind)
        ),
        Expr::Call(name, arguments) => format!(
            "{name}({})",
            arguments
                .iter()
                .map(|argument| render_expr(argument, kind))
                .collect::<Vec<_>>()
                .join(", ")
        ),
        Expr::Block { body, tail } => {
            // Rendered on one line: a block used as a value is small, and the
            // statement renderer already ends each statement with `;`.
            let mut lines = Vec::new();
            render_statements(body, kind, "", &mut lines);
            lines.push(render_expr(tail, kind));
            format!("{{ {} }}", lines.join(" "))
        }
        Expr::Index { array, index } => format!("{array}[{index}]"),
        Expr::Field { value, field } => format!("{value}.f{field}"),
        Expr::HandleField { handle, field } => format!("{handle}.f{field}"),
        Expr::ReadEnum { reader, value, .. } => format!("{reader}({value})"),
    }
}

pub fn render_literal(value: i128, kind: IntType) -> String {
    let name = kind.name;
    if value >= 0 {
        return format!("{value}{name}");
    }
    if -value > kind.high {
        // The minimum has no positive literal of its own.
        return format!("(0{name} - {}{name} - 1{name})", kind.high);
    }
    format!("(0{name} - {}{name})", -value)
}

fn render_statements(body: &[Stmt], kind: IntType, indent: &str, lines: &mut Vec<String>) {
    for (position, statement) in body.iter().enumerate() {
        let last = position + 1 == body.len();
        match statement {
            Stmt::Let {
                name,
                mutable,
                value,
            } => lines.push(format!(
                "{indent}let {}{name}: {} = {};",
                if *mutable { "mut " } else { "" },
                kind.name,
                render_expr(value, kind)
            )),
            Stmt::Assign { name, value } => {
                lines.push(format!("{indent}{name} = {};", render_expr(value, kind)))
            }
            Stmt::Compound { name, op, value } => lines.push(format!(
                "{indent}{name} {op}= {};",
                render_expr(value, kind)
            )),
            Stmt::While { condition, body } => {
                lines.push(format!("{indent}while {} {{", render_expr(condition, kind)));
                render_statements(body, kind, &format!("{indent}    "), lines);
                lines.push(format!("{indent}}}"));
            }
            Stmt::IfStatement {
                condition,
                then_body,
                else_body,
            } => {
                // The last statement of a body is followed by the block's
                // tail expression, which usually starts with `(`. The
                // frontend then parses `if c { .. } (tail)` as a call of the
                // `if` and rejects the program, so that position takes the
                // `;` form the grammar's `expression_stmt` spells out. Both
                // forms are generated, and the divergence between them is
                // reported, not worked around silently.
                let terminator = if last { ";" } else { "" };
                lines.push(format!("{indent}if {} {{", render_expr(condition, kind)));
                render_statements(then_body, kind, &format!("{indent}    "), lines);
                match else_body {
                    None => lines.push(format!("{indent}}}{terminator}")),
                    Some(body) => {
                        lines.push(format!("{indent}}} else {{"));
                        render_statements(body, kind, &format!("{indent}    "), lines);
                        lines.push(format!("{indent}}}{terminator}"));
                    }
                }
            }
            Stmt::LetArray { name, elements } => lines.push(format!(
                "{indent}let {name}: Array<{}, {}> = [{}];",
                kind.name,
                elements.len(),
                elements
                    .iter()
                    .map(|element| render_expr(element, kind))
                    .collect::<Vec<_>>()
                    .join(", ")
            )),
            Stmt::LetIndex { name, value } => {
                lines.push(format!("{indent}let {name}: u64 = {value}u64;"))
            }
            Stmt::LetStruct {
                name,
                type_name,
                fields,
            } => lines.push(format!(
                "{indent}let {name}: {type_name} = {type_name} {{ {} }};",
                fields
                    .iter()
                    .enumerate()
                    .map(|(position, field)| format!("f{position}: {}", render_expr(field, kind)))
                    .collect::<Vec<_>>()
                    .join(", ")
            )),
            Stmt::LetBuffer { name } => lines.push(format!(
                "{indent}let mut {name}: Buffer<{}> = Buffer::new();",
                kind.name
            )),
            Stmt::Push { name, value } => lines.push(format!(
                "{indent}{name}.push({});",
                render_expr(value, kind)
            )),
            Stmt::LetArena { name, type_name } => lines.push(format!(
                "{indent}let mut {name}: Arena<{type_name}> = Arena::new();"
            )),
            Stmt::Alloc {
                handle,
                arena,
                type_name,
                fields,
            } => lines.push(format!(
                "{indent}let {handle}: Handle<{type_name}> = {arena}.alloc({type_name} {{ {} }});",
                fields
                    .iter()
                    .enumerate()
                    .map(|(position, field)| format!("f{position}: {}", render_expr(field, kind)))
                    .collect::<Vec<_>>()
                    .join(", ")
            )),
            Stmt::Release { arena } => lines.push(format!("{indent}{arena}.release();")),
            Stmt::LetEnum {
                name,
                type_name,
                variant,
                payload,
            } => {
                let variant_name = format!("V{variant}");
                match payload {
                    Some(value) => lines.push(format!(
                        "{indent}let {name}: {type_name} = {type_name}::{variant_name} {{ p0: {} }};",
                        render_expr(value, kind)
                    )),
                    None => lines.push(format!(
                        "{indent}let {name}: {type_name} = {type_name}::{variant_name};"
                    )),
                }
            }
        }
    }
}

pub fn render_program(program: &Program) -> String {
    let kind = program.kind;
    let mut lines = vec![
        "core 0.1;".to_string(),
        format!("module fuzz.{};", program.id),
        String::new(),
    ];
    for (name, fields) in &program.structs {
        let declared = (0..*fields)
            .map(|position| format!("f{position}: {}", kind.name))
            .collect::<Vec<_>>()
            .join(", ");
        lines.push(format!("struct {name} {{ {declared}, }}"));
        lines.push(String::new());
    }
    for declaration in &program.enums {
        let variants = declaration
            .variants
            .iter()
            .enumerate()
            .map(|(index, fallback)| match fallback {
                Some(_) => format!("V{index}"),
                None => format!("V{index} {{ p0: {}, }}", kind.name),
            })
            .collect::<Vec<_>>()
            .join(", ");
        lines.push(format!("enum {} {{ {variants}, }}", declaration.name));
        lines.push(String::new());
        // The reader lists every variant: Core requires an exhaustive match.
        lines.push(format!(
            "fn {}(value: {}) -> {} {{",
            declaration.reader, declaration.name, kind.name
        ));
        lines.push("    match value {".to_string());
        for (index, fallback) in declaration.variants.iter().enumerate() {
            let arm = match fallback {
                Some(value) => format!(
                    "        {}::V{index} => {},",
                    declaration.name,
                    render_literal(*value, kind)
                ),
                None => format!("        {}::V{index} {{ p0 }} => p0,", declaration.name),
            };
            lines.push(arm);
        }
        lines.push("    }".to_string());
        lines.push("}".to_string());
        lines.push(String::new());
    }
    for function in &program.functions {
        let params = function
            .params
            .iter()
            .map(|name| format!("{name}: {}", kind.name))
            .collect::<Vec<_>>()
            .join(", ");
        lines.push(format!(
            "fn {}({params}) -> {} {{",
            function.name, kind.name
        ));
        render_statements(&function.body, kind, "    ", &mut lines);
        lines.push(format!("    {}", render_expr(&function.tail, kind)));
        lines.push("}".to_string());
        lines.push(String::new());
    }
    lines.push(format!("pub fn argorix_main() -> {} {{", kind.name));
    render_statements(&program.main.body, kind, "    ", &mut lines);
    lines.push(format!("    {}", render_expr(&program.main.tail, kind)));
    lines.push("}".to_string());
    lines.join("\n") + "\n"
}

// --------------------------------------------------------------------------
// Deterministic random source (SplitMix64), so a seed reproduces its corpus

pub struct Rng {
    state: u64,
}

impl Rng {
    pub fn new(seed: u64) -> Self {
        Rng {
            state: seed
                .wrapping_mul(0x9e37_79b9_7f4a_7c15)
                .wrapping_add(0x1234_5678),
        }
    }
    pub fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        z ^ (z >> 31)
    }
    pub fn below(&mut self, limit: u64) -> u64 {
        if limit == 0 {
            0
        } else {
            self.next_u64() % limit
        }
    }
    pub fn range(&mut self, low: i128, high: i128) -> i128 {
        if high <= low {
            return low;
        }
        low + self.below((high - low + 1) as u64) as i128
    }
    /// A percentage chance, so call sites read as odds rather than floats.
    pub fn chance(&mut self, percent: u64) -> bool {
        self.below(100) < percent
    }
    pub fn pick<'a, T>(&mut self, items: &'a [T]) -> &'a T {
        &items[self.below(items.len() as u64) as usize]
    }
}

// --------------------------------------------------------------------------
// Generation

const ARITH: [&str; 8] = ["+", "-", "*", "/", "%", "&", "|", "^"];
const COMPARE: [&str; 6] = ["==", "!=", "<", "<=", ">", ">="];

struct Generator {
    rng: Rng,
    kind: IntType,
    counter: usize,
}

impl Generator {
    fn fresh(&mut self, prefix: &str) -> String {
        self.counter += 1;
        format!("{prefix}{}", self.counter)
    }

    fn literal(&mut self) -> Expr {
        let kind = self.kind;
        let value = if self.rng.chance(10) {
            *self.rng.pick(&[kind.low, kind.high, kind.high / 2, 0, 1])
        } else {
            self.rng.range(kind.low.max(-20), kind.high.min(20))
        };
        Expr::Literal(value)
    }

    fn value(
        &mut self,
        names: &[String],
        depth: u32,
        functions: &[Function],
        allow_if: bool,
    ) -> Expr {
        if depth == 0 || self.rng.chance(30) {
            if !names.is_empty() && self.rng.chance(60) {
                return Expr::Var(self.rng.pick(names).clone());
            }
            return self.literal();
        }
        let choice = self.rng.below(100);
        if choice < 7 {
            return self.shift(names, depth, functions, allow_if);
        }
        if choice < 12 && !unsigned(self.kind) {
            return self.negation(names, depth, functions, allow_if);
        }
        if choice < 17 && allow_if && !names.is_empty() {
            return self.block_value(names);
        }
        if choice < 55 {
            let op = *self.rng.pick(&ARITH);
            let left = self.value(names, depth - 1, functions, allow_if);
            let mut right = self.value(names, depth - 1, functions, allow_if);
            if matches!(op, "/" | "%") && right == Expr::Literal(0) {
                // A literal zero divisor is a static error in Core, not a trap.
                right = Expr::Literal(self.rng.range(1, 20));
            }
            let left = if op == "^"
                && matches!(left, Expr::Literal(_))
                && matches!(right, Expr::Literal(_))
            {
                // GCC reads a literal `10 ^ 6` in the generated C as a mistyped
                // power (-Wxor-used-as-pow), so one side is always computed.
                Expr::Arith("+", Box::new(left), Box::new(Expr::Literal(0)))
            } else {
                left
            };
            return Expr::Arith(op, Box::new(left), Box::new(right));
        }
        if choice < 75 && allow_if {
            return Expr::If(
                Box::new(self.condition(names, depth - 1, functions)),
                Box::new(self.value(names, depth - 1, functions, allow_if)),
                Box::new(self.value(names, depth - 1, functions, allow_if)),
            );
        }
        if choice < 90 && !functions.is_empty() {
            let function = self.rng.pick(functions).clone();
            let arguments = function
                .params
                .iter()
                .map(|_| self.value(names, depth - 1, functions, allow_if))
                .collect();
            return Expr::Call(function.name, arguments);
        }
        self.literal()
    }

    /// `left << amount` or `left >> amount`.
    ///
    /// The amount is usually inside the width, and sometimes past it, which
    /// `spec/core/evaluation.md` makes a trap rather than a wrapped amount.
    fn shift(
        &mut self,
        names: &[String],
        depth: u32,
        functions: &[Function],
        allow_if: bool,
    ) -> Expr {
        let op = if self.rng.chance(50) { "<<" } else { ">>" };
        let left = self.value(names, depth - 1, functions, allow_if);
        let bits = i128::from(width(self.kind));
        let amount = if self.rng.chance(70) {
            Expr::Literal(self.rng.range(0, bits - 1))
        } else if self.rng.chance(50) {
            Expr::Literal(self.rng.range(bits, bits + 8))
        } else {
            // A computed amount, which the oracle and the runtime must judge
            // by its value and not by its shape.
            self.value(names, depth - 1, functions, allow_if)
        };
        Expr::Shift(op, Box::new(left), Box::new(amount))
    }

    /// `-value`, only in a signed program. The backend has no representable
    /// result for an unsigned one, and the minimum is the single input that
    /// overflows.
    fn negation(
        &mut self,
        names: &[String],
        depth: u32,
        functions: &[Function],
        allow_if: bool,
    ) -> Expr {
        if self.rng.chance(15) {
            return Expr::Negate(Box::new(Expr::Literal(self.kind.low)));
        }
        Expr::Negate(Box::new(self.value(names, depth - 1, functions, allow_if)))
    }

    /// `{ let x: T = x + k; x }` used as a value.
    ///
    /// The local shadows an outer name and is initialised from it: the shape
    /// of gap g04, where the block's local used to land in the enclosing C
    /// scope and outlive the block. Initialising from the shadowed name also
    /// keeps that outer local genuinely read.
    fn block_value(&mut self, names: &[String]) -> Expr {
        let shadowed = self.rng.pick(names).clone();
        let op = *self.rng.pick(&["+", "-", "^"]);
        let value = Expr::Arith(
            op,
            Box::new(Expr::Var(shadowed.clone())),
            Box::new(self.literal()),
        );
        // The tail always reads the block's local: an unused one is valid
        // Core, but its C fails the declared -Werror profile.
        let tail = if self.rng.chance(50) {
            Expr::Var(shadowed.clone())
        } else {
            Expr::Arith(
                self.rng.pick(&["+", "-", "^"]),
                Box::new(Expr::Var(shadowed.clone())),
                Box::new(self.literal()),
            )
        };
        Expr::Block {
            body: vec![Stmt::Let {
                name: shadowed,
                mutable: false,
                value,
            }],
            tail: Box::new(tail),
        }
    }

    /// `if condition { .. }`, with an `else` part of the time: gap g12, where
    /// an `if` with no value was rejected outright.
    fn if_statement(
        &mut self,
        names: &[String],
        mutable: &[String],
        functions: &[Function],
    ) -> Stmt {
        let condition = self.condition(names, 1, functions);
        let then_body = self.branch_body(names, mutable, functions);
        let else_body = self
            .rng
            .chance(40)
            .then(|| self.branch_body(names, mutable, functions));
        Stmt::IfStatement {
            condition,
            then_body,
            else_body,
        }
    }

    /// The body of a branch: assignments to locals that already exist, so the
    /// branch has an observable effect without declaring anything that would
    /// escape its scope.
    fn branch_body(
        &mut self,
        names: &[String],
        mutable: &[String],
        functions: &[Function],
    ) -> Vec<Stmt> {
        let mut body = Vec::new();
        for _ in 0..=self.rng.below(2) {
            let name = self.rng.pick(mutable).clone();
            if self.rng.chance(50) {
                let value = self.value(names, 1, functions, true);
                body.push(Stmt::Assign { name, value });
            } else {
                let op = *self.rng.pick(&["+", "-", "*"]);
                let value = self.literal();
                body.push(Stmt::Compound { name, op, value });
            }
        }
        body
    }

    fn condition(&mut self, names: &[String], depth: u32, functions: &[Function]) -> Expr {
        let choice = self.rng.below(100);
        if choice < 60 || depth == 0 {
            // No `if` inside a comparison operand: that is gap g16.
            let mut left = self.value(names, depth, functions, false);
            let mut right = self.value(names, depth, functions, false);
            if render_expr(&left, self.kind) == render_expr(&right, self.kind) {
                right = Expr::Arith("+", Box::new(right), Box::new(Expr::Literal(1)));
            }
            let op = *self.rng.pick(&COMPARE);
            if unsigned(self.kind) && matches!(op, "<" | "<=" | ">" | ">=") {
                // `unsigned < 0` is always false, which -Wtype-limits rejects.
                if left == Expr::Literal(0) {
                    left = Expr::Literal(self.rng.range(1, 20));
                }
                if right == Expr::Literal(0) {
                    right = Expr::Literal(self.rng.range(1, 20));
                }
            }
            let edge = |expr: &Expr, kind: IntType| matches!(expr, Expr::Literal(value) if *value == kind.low || *value == kind.high);
            if matches!(op, "<" | "<=" | ">" | ">=") {
                if edge(&left, self.kind) {
                    left = Expr::Literal(self.rng.range(1, 20));
                }
                if edge(&right, self.kind) {
                    right = Expr::Literal(self.rng.range(1, 20));
                }
            }
            return Expr::Compare(op, Box::new(left), Box::new(right));
        }
        if choice < 80 {
            let left = Box::new(self.condition(names, depth - 1, functions));
            let right = Box::new(self.condition(names, depth - 1, functions));
            return if self.rng.chance(50) {
                Expr::And(left, right)
            } else {
                Expr::Or(left, right)
            };
        }
        Expr::Not(Box::new(self.condition(names, depth - 1, functions)))
    }

    fn statements(
        &mut self,
        names: &mut Vec<String>,
        mutable: &mut Vec<String>,
        functions: &[Function],
        allow_loop: bool,
    ) -> Vec<Stmt> {
        let mut body = Vec::new();
        for _ in 0..=self.rng.below(4) {
            let choice = self.rng.below(100);
            if choice < 50 || mutable.is_empty() {
                let name = self.fresh("v");
                let is_mutable = self.rng.chance(50);
                let value = self.value(names, 2, functions, true);
                body.push(Stmt::Let {
                    name: name.clone(),
                    mutable: is_mutable,
                    value,
                });
                names.push(name.clone());
                if is_mutable {
                    mutable.push(name);
                }
            } else if choice < 70 {
                let name = self.rng.pick(mutable).clone();
                let value = self.value(names, 2, functions, true);
                body.push(Stmt::Assign { name, value });
            } else if choice < 80 {
                let name = self.rng.pick(mutable).clone();
                let op = *self.rng.pick(&["+", "-", "*"]);
                let value = self.literal();
                body.push(Stmt::Compound { name, op, value });
            } else if choice < 90 || !allow_loop {
                let statement = self.if_statement(names, mutable, functions);
                body.push(statement);
            } else {
                body.extend(self.loop_statement(names, mutable, functions));
            }
        }
        body
    }

    /// A counted loop: the counter is declared just before the `while`.
    fn loop_statement(
        &mut self,
        names: &mut Vec<String>,
        mutable: &mut Vec<String>,
        functions: &[Function],
    ) -> Vec<Stmt> {
        let counter = self.fresh("i");
        let limit = self.rng.range(1, 6);
        let mut inner = Vec::new();
        if !mutable.is_empty() {
            let target = self.rng.pick(mutable).clone();
            let mut visible = names.clone();
            visible.push(counter.clone());
            let value = self.value(&visible, 1, functions, true);
            inner.push(Stmt::Assign {
                name: target,
                value,
            });
        }
        inner.push(Stmt::Compound {
            name: counter.clone(),
            op: "+",
            value: Expr::Literal(1),
        });
        names.push(counter.clone());
        mutable.push(counter.clone());
        vec![
            Stmt::Let {
                name: counter.clone(),
                mutable: true,
                value: Expr::Literal(0),
            },
            Stmt::While {
                condition: Expr::Compare(
                    "<",
                    Box::new(Expr::Var(counter)),
                    Box::new(Expr::Literal(limit)),
                ),
                body: inner,
            },
        ]
    }
}

fn reads(expr: &Expr, seen: &mut BTreeSet<String>) {
    match expr {
        Expr::Var(name) => {
            seen.insert(name.clone());
        }
        Expr::Literal(_) => {}
        Expr::Not(inner) | Expr::Negate(inner) => reads(inner, seen),
        Expr::And(left, right)
        | Expr::Or(left, right)
        | Expr::Arith(_, left, right)
        | Expr::Shift(_, left, right)
        | Expr::Compare(_, left, right) => {
            reads(left, seen);
            reads(right, seen);
        }
        Expr::Block { body, tail } => {
            // A block local that shadows an outer name is always initialised
            // from it, so counting the outer name as read is exact.
            body.iter().for_each(|inner| statement_reads(inner, seen));
            reads(tail, seen);
        }
        Expr::If(condition, then_branch, else_branch) => {
            reads(condition, seen);
            reads(then_branch, seen);
            reads(else_branch, seen);
        }
        Expr::Call(_, arguments) => arguments.iter().for_each(|argument| reads(argument, seen)),
        Expr::Index { array, index } => {
            seen.insert(array.clone());
            seen.insert(index.clone());
        }
        Expr::Field { value, .. } => {
            seen.insert(value.clone());
        }
        Expr::HandleField { handle, .. } => {
            seen.insert(handle.clone());
        }
        Expr::ReadEnum { value, .. } => {
            seen.insert(value.clone());
        }
    }
}

fn statement_reads(statement: &Stmt, seen: &mut BTreeSet<String>) {
    match statement {
        Stmt::Let { value, .. } | Stmt::Assign { value, .. } => reads(value, seen),
        Stmt::LetArray {
            elements: items, ..
        }
        | Stmt::LetStruct { fields: items, .. }
        | Stmt::Alloc { fields: items, .. } => items.iter().for_each(|item| reads(item, seen)),
        Stmt::Push { name, value } => {
            seen.insert(name.clone());
            reads(value, seen);
        }
        Stmt::Release { arena } => {
            seen.insert(arena.clone());
        }
        Stmt::LetEnum { payload, .. } => {
            if let Some(value) = payload {
                reads(value, seen);
            }
        }
        Stmt::LetIndex { .. } | Stmt::LetBuffer { .. } | Stmt::LetArena { .. } => {}
        Stmt::Compound { name, value, .. } => {
            seen.insert(name.clone());
            reads(value, seen);
        }
        Stmt::While { condition, body } => {
            reads(condition, seen);
            body.iter().for_each(|inner| statement_reads(inner, seen));
        }
        Stmt::IfStatement {
            condition,
            then_body,
            else_body,
        } => {
            reads(condition, seen);
            then_body
                .iter()
                .for_each(|inner| statement_reads(inner, seen));
            if let Some(body) = else_body {
                body.iter().for_each(|inner| statement_reads(inner, seen));
            }
        }
    }
}

fn declared(body: &[Stmt], names: &mut Vec<String>) {
    for statement in body {
        match statement {
            Stmt::Let { name, .. } => names.push(name.clone()),
            Stmt::While { body, .. } => declared(body, names),
            _ => {}
        }
    }
}

fn calls(expr: &Expr, seen: &mut BTreeSet<String>) {
    match expr {
        Expr::Call(name, arguments) => {
            seen.insert(name.clone());
            arguments.iter().for_each(|argument| calls(argument, seen));
        }
        Expr::Not(inner) | Expr::Negate(inner) => calls(inner, seen),
        Expr::And(left, right)
        | Expr::Or(left, right)
        | Expr::Arith(_, left, right)
        | Expr::Shift(_, left, right)
        | Expr::Compare(_, left, right) => {
            calls(left, seen);
            calls(right, seen);
        }
        Expr::Block { body, tail } => {
            // A block local that shadows an outer name is always initialised
            // from it, so counting the outer name as read is exact.
            body.iter().for_each(|inner| statement_calls(inner, seen));
            calls(tail, seen);
        }
        Expr::If(condition, then_branch, else_branch) => {
            calls(condition, seen);
            calls(then_branch, seen);
            calls(else_branch, seen);
        }
        Expr::Literal(_)
        | Expr::Var(_)
        | Expr::Index { .. }
        | Expr::Field { .. }
        | Expr::HandleField { .. }
        // The reader is generated with the enum, not through the call graph.
        | Expr::ReadEnum { .. } => {}
    }
}

fn statement_calls(statement: &Stmt, seen: &mut BTreeSet<String>) {
    match statement {
        Stmt::Let { value, .. } | Stmt::Assign { value, .. } | Stmt::Compound { value, .. } => {
            calls(value, seen)
        }
        Stmt::LetArray {
            elements: items, ..
        }
        | Stmt::LetStruct { fields: items, .. }
        | Stmt::Alloc { fields: items, .. } => items.iter().for_each(|item| calls(item, seen)),
        Stmt::Push { value, .. } => calls(value, seen),
        Stmt::LetIndex { .. }
        | Stmt::LetBuffer { .. }
        | Stmt::LetArena { .. }
        | Stmt::Release { .. } => {}
        Stmt::LetEnum { payload, .. } => {
            if let Some(value) = payload {
                calls(value, seen);
            }
        }
        Stmt::While { condition, body } => {
            calls(condition, seen);
            body.iter().for_each(|inner| statement_calls(inner, seen));
        }
        Stmt::IfStatement {
            condition,
            then_body,
            else_body,
        } => {
            calls(condition, seen);
            then_body
                .iter()
                .for_each(|inner| statement_calls(inner, seen));
            if let Some(body) = else_body {
                body.iter().for_each(|inner| statement_calls(inner, seen));
            }
        }
    }
}

/// Fold unread locals into the tail: an unused local is valid Core but its C
/// fails the declared -Werror profile (issue #27), which would drown real
/// divergences in noise.
fn use_every_local(body: &[Stmt], tail: Expr) -> Expr {
    let mut seen = BTreeSet::new();
    body.iter()
        .for_each(|statement| statement_reads(statement, &mut seen));
    reads(&tail, &mut seen);
    let mut names = Vec::new();
    declared(body, &mut names);
    let mut result = tail;
    for name in names {
        if !seen.contains(&name) {
            result = Expr::Arith("^", Box::new(result), Box::new(Expr::Var(name)));
        }
    }
    result
}

pub fn generate_program(seed: u64, index: u64) -> Program {
    let mut rng = Rng::new(seed.wrapping_shl(20).wrapping_add(index));
    let kind = TYPES[rng.below(TYPES.len() as u64) as usize];
    let mut generator = Generator {
        rng,
        kind,
        counter: 0,
    };
    let mut functions: Vec<Function> = Vec::new();
    for position in 0..generator.rng.below(3) {
        let function = generator.function(position, &functions.clone());
        functions.push(function);
    }
    let mut names = Vec::new();
    let mut mutable = Vec::new();
    let mut body = generator.statements(&mut names, &mut mutable, &functions, true);

    // Aggregates are declared at function level only: inside an `if` or a loop
    // body they hit gap g06, and nesting them hits g01, g02 and g03.
    let mut structs = Vec::new();
    let mut reads: Vec<Expr> = Vec::new();
    if generator.rng.chance(55) {
        let (statements, expression) = generator.array(&names, &functions);
        body.extend(statements);
        reads.push(expression);
    }
    if generator.rng.chance(45) {
        let (declaration, statement, expression) = generator.flat_struct(&names, &functions);
        structs.push(declaration);
        body.push(statement);
        reads.push(expression);
    }
    if generator.rng.chance(40) {
        let (statements, expression) = generator.buffer(&names, &functions);
        body.extend(statements);
        reads.push(expression);
    }
    if generator.rng.chance(35) {
        let (declaration, statements, expression) = generator.arena(&names, &functions);
        structs.push(declaration);
        body.extend(statements);
        reads.push(expression);
    }
    let mut enums = Vec::new();
    if generator.rng.chance(45) {
        let (declaration, statement, expression) =
            generator.enumeration(&names, &functions, enums.len());
        enums.push(declaration);
        body.push(statement);
        reads.push(expression);
    }
    let mut tail = generator.value(&names, 3, &functions, true);
    for expression in reads {
        tail = Expr::Arith("^", Box::new(tail), Box::new(expression));
    }

    let mut called = BTreeSet::new();
    body.iter()
        .for_each(|statement| statement_calls(statement, &mut called));
    calls(&tail, &mut called);
    for function in &functions {
        if !called.contains(&function.name) {
            // An uncalled private function fails -Wunused-function.
            let arguments = function
                .params
                .iter()
                .map(|_| generator.literal())
                .collect();
            tail = Expr::Arith(
                "^",
                Box::new(tail),
                Box::new(Expr::Call(function.name.clone(), arguments)),
            );
        }
    }
    tail = use_every_local(&body, tail);
    Program {
        id: format!("fuzz_{seed}_{index:04}"),
        kind,
        structs,
        enums,
        functions,
        main: Function {
            name: "argorix_main".into(),
            params: Vec::new(),
            body,
            tail,
        },
    }
}

impl Generator {
    /// A fixed array plus the `u64` index local that reads it. The index is in
    /// range most of the time and past the end sometimes, which must trap.
    fn array(&mut self, names: &[String], functions: &[Function]) -> (Vec<Stmt>, Expr) {
        let name = self.fresh("a");
        let index_name = self.fresh("x");
        let length = 1 + self.rng.below(4) as usize;
        let elements = (0..length)
            .map(|_| self.value(names, 1, functions, false))
            .collect();
        let index = if self.rng.chance(20) {
            length as u64 + self.rng.below(3)
        } else {
            self.rng.below(length as u64)
        };
        (
            vec![
                Stmt::LetArray {
                    name: name.clone(),
                    elements,
                },
                Stmt::LetIndex {
                    name: index_name.clone(),
                    value: index,
                },
            ],
            Expr::Index {
                array: name,
                index: index_name,
            },
        )
    }

    /// A buffer filled by pushes, and a read that may be past the end.
    fn buffer(&mut self, names: &[String], functions: &[Function]) -> (Vec<Stmt>, Expr) {
        let name = self.fresh("b");
        let index_name = self.fresh("x");
        let pushes = 1 + self.rng.below(4) as usize;
        let mut statements = vec![Stmt::LetBuffer { name: name.clone() }];
        for _ in 0..pushes {
            statements.push(Stmt::Push {
                name: name.clone(),
                value: self.value(names, 1, functions, false),
            });
        }
        let index = if self.rng.chance(20) {
            pushes as u64 + self.rng.below(3)
        } else {
            self.rng.below(pushes as u64)
        };
        statements.push(Stmt::LetIndex {
            name: index_name.clone(),
            value: index,
        });
        (
            statements,
            Expr::Index {
                array: name,
                index: index_name,
            },
        )
    }

    /// An arena, one allocation, and a read through its handle. The arena is
    /// sometimes released first, and then the read must trap.
    fn arena(
        &mut self,
        names: &[String],
        functions: &[Function],
    ) -> ((String, usize), Vec<Stmt>, Expr) {
        let type_name = format!("N{}", self.counter + 1);
        let name = self.fresh("r");
        let handle = self.fresh("h");
        let count = 1 + self.rng.below(2) as usize;
        let fields = (0..count)
            .map(|_| self.value(names, 1, functions, false))
            .collect();
        let mut statements = vec![
            Stmt::LetArena {
                name: name.clone(),
                type_name: type_name.clone(),
            },
            Stmt::Alloc {
                handle: handle.clone(),
                arena: name.clone(),
                type_name: type_name.clone(),
                fields,
            },
        ];
        // Releasing before the read is the ARENA_RELEASED trap; releasing is
        // also what keeps the arena from sitting in the runtime's registry.
        if self.rng.chance(25) {
            statements.push(Stmt::Release { arena: name });
        }
        (
            (type_name, count),
            statements,
            Expr::HandleField {
                handle,
                field: self.rng.below(count as u64) as usize,
            },
        )
    }

    /// An enum, the exhaustive `match` that reads it, and one value of it.
    fn enumeration(
        &mut self,
        names: &[String],
        functions: &[Function],
        position: usize,
    ) -> (EnumDecl, Stmt, Expr) {
        let name = format!("E{}", self.counter + 1);
        let local = self.fresh("e");
        let count = 2 + self.rng.below(2) as usize;
        // At least one variant carries a field and at least one does not, so
        // both kinds of arm are exercised.
        let with_payload = self.rng.below(count as u64) as usize;
        let variants: Vec<Option<i128>> = (0..count)
            .map(|index| {
                if index == with_payload {
                    None
                } else {
                    Some(self.rng.range(0.max(self.kind.low), self.kind.high.min(30)))
                }
            })
            .collect();
        let chosen = self.rng.below(count as u64) as usize;
        let declaration = EnumDecl {
            name: name.clone(),
            reader: format!("read{position}"),
            variants,
        };
        let payload = declaration.variants[chosen]
            .is_none()
            .then(|| self.value(names, 1, functions, false));
        let statement = Stmt::LetEnum {
            name: local.clone(),
            type_name: name,
            variant: chosen,
            payload,
        };
        let read = Expr::ReadEnum {
            declaration: position,
            reader: declaration.reader.clone(),
            value: local,
        };
        (declaration, statement, read)
    }

    /// A struct of scalar fields, and a read of one of them.
    fn flat_struct(
        &mut self,
        names: &[String],
        functions: &[Function],
    ) -> ((String, usize), Stmt, Expr) {
        let type_name = format!("S{}", self.counter + 1);
        let name = self.fresh("s");
        let count = 1 + self.rng.below(3) as usize;
        let fields = (0..count)
            .map(|_| self.value(names, 1, functions, false))
            .collect();
        let read = Expr::Field {
            value: name.clone(),
            field: self.rng.below(count as u64) as usize,
        };
        (
            (type_name.clone(), count),
            Stmt::LetStruct {
                name,
                type_name,
                fields,
            },
            read,
        )
    }

    fn function(&mut self, index: u64, functions: &[Function]) -> Function {
        let name = format!("helper{index}");
        let count = 1 + self.rng.below(2);
        let params: Vec<String> = (0..count)
            .map(|position| format!("p{index}_{position}"))
            .collect();
        let mut names = params.clone();
        let mut mutable = Vec::new();
        let body = self.statements(&mut names, &mut mutable, functions, false);
        // Every parameter is read, so the generated C survives -Wunused-parameter.
        let mut tail = Expr::Var(params[0].clone());
        for extra in params.iter().skip(1) {
            tail = Expr::Arith("^", Box::new(tail), Box::new(Expr::Var(extra.clone())));
        }
        let extra = self.value(&names, 2, functions, true);
        tail = Expr::Arith("^", Box::new(tail), Box::new(extra));
        let tail = use_every_local(&body, tail);
        Function {
            name,
            params,
            body,
            tail,
        }
    }
}

// --------------------------------------------------------------------------
// Corpus

#[derive(Debug, Serialize)]
pub struct GeneratedManifest {
    pub schema_version: u32,
    pub core_version: String,
    pub generator: String,
    pub oracle: String,
    pub seed: u64,
    pub attempts: u64,
    pub cases: Vec<Case>,
}

pub fn expected_case(program: &Program) -> Option<(String, String, i32)> {
    match evaluate_program(program) {
        Ok(value) => Some((format!("ARGORIX_RESULT:{value}"), String::new(), 0)),
        Err(Failure::Trapped(trap)) => {
            Some((String::new(), format!("ARGORIX_TRAP:{}", trap.code()), 70))
        }
        Err(Failure::Unusable(_)) => None,
    }
}

pub fn build_corpus(seed: u64, count: usize, out: &Path) -> Result<GeneratedManifest> {
    std::fs::create_dir_all(out)?;
    for entry in std::fs::read_dir(out)? {
        let path = entry?.path();
        if path
            .extension()
            .is_some_and(|extension| extension == "argx")
        {
            std::fs::remove_file(path)?;
        }
    }
    let mut cases = Vec::new();
    let mut index = 0u64;
    let mut attempts = 0u64;
    while cases.len() < count && attempts < (count as u64) * 20 {
        attempts += 1;
        index += 1;
        let program = generate_program(seed, index);
        let Some((stdout, stderr, exit)) = expected_case(&program) else {
            continue;
        };
        std::fs::write(
            out.join(format!("{}.argx", program.id)),
            render_program(&program),
        )?;
        cases.push(Case {
            id: program.id.clone(),
            file: format!("{}.argx", program.id),
            expected_exit: exit,
            expected_stdout: stdout,
            expected_stderr: stderr,
        });
    }
    let manifest = GeneratedManifest {
        schema_version: 1,
        core_version: "0.1".into(),
        generator: "crates/argorix_core_c (core-c-harness generate)".into(),
        oracle:
            "spec/core/evaluation.md and spec/core/types.md, evaluated independently of argorixc"
                .into(),
        seed,
        attempts,
        cases,
    };
    std::fs::write(
        out.join("cases.json"),
        serde_json::to_string_pretty(&manifest)? + "\n",
    )?;
    Ok(manifest)
}

#[cfg(test)]
mod tests {
    use super::*;

    const I32: IntType = TYPES[6];
    const U8: IntType = TYPES[0];

    fn trap_of(result: Result<i128, Failure>) -> Option<Trap> {
        match result {
            Err(Failure::Trapped(trap)) => Some(trap),
            _ => None,
        }
    }

    #[test]
    fn exact_width_overflow_traps() {
        assert_eq!(
            trap_of(arithmetic("+", 255, 1, U8)),
            Some(Trap::IntegerOverflow)
        );
        assert_eq!(
            trap_of(arithmetic("-", 0, 1, U8)),
            Some(Trap::IntegerOverflow)
        );
        assert_eq!(
            trap_of(arithmetic("*", 16, 16, U8)),
            Some(Trap::IntegerOverflow)
        );
        assert_eq!(arithmetic("*", 16, 15, U8).ok(), Some(240));
    }

    #[test]
    fn division_by_zero_and_minimum_over_minus_one_trap() {
        for op in ["/", "%"] {
            assert_eq!(
                trap_of(arithmetic(op, 7, 0, I32)),
                Some(Trap::DivisionByZero)
            );
            assert_eq!(
                trap_of(arithmetic(op, I32.low, -1, I32)),
                Some(Trap::IntegerOverflow)
            );
        }
    }

    #[test]
    fn division_truncates_toward_zero() {
        assert_eq!(arithmetic("/", -7, 2, I32).ok(), Some(-3));
        assert_eq!(arithmetic("/", 7, -2, I32).ok(), Some(-3));
        assert_eq!(arithmetic("%", -7, 2, I32).ok(), Some(-1));
        assert_eq!(arithmetic("%", 7, -2, I32).ok(), Some(1));
    }

    #[test]
    fn bitwise_operators_do_not_trap() {
        assert_eq!(arithmetic("&", 12, 10, I32).ok(), Some(8));
        assert_eq!(arithmetic("|", 12, 10, I32).ok(), Some(14));
        assert_eq!(arithmetic("^", 12, 10, I32).ok(), Some(6));
    }

    fn program_with(body: Vec<Stmt>, tail: Expr) -> Program {
        Program {
            id: "t".into(),
            kind: I32,
            structs: Vec::new(),
            enums: Vec::new(),
            functions: Vec::new(),
            main: Function {
                name: "argorix_main".into(),
                params: Vec::new(),
                body,
                tail,
            },
        }
    }

    fn divide_by_zero() -> Expr {
        Expr::Arith("/", Box::new(Expr::Literal(1)), Box::new(Expr::Literal(0)))
    }

    #[test]
    fn the_left_operand_traps_first() {
        let overflow = Expr::Arith(
            "+",
            Box::new(Expr::Literal(I32.high)),
            Box::new(Expr::Literal(1)),
        );
        let tail = Expr::Arith("+", Box::new(divide_by_zero()), Box::new(overflow));
        assert_eq!(
            trap_of(evaluate_program(&program_with(Vec::new(), tail))),
            Some(Trap::DivisionByZero)
        );
    }

    #[test]
    fn and_or_short_circuit_before_a_trap() {
        let trapping = Expr::Compare("==", Box::new(divide_by_zero()), Box::new(Expr::Literal(0)));
        let never = Expr::Compare("==", Box::new(Expr::Literal(1)), Box::new(Expr::Literal(2)));
        let tail = Expr::If(
            Box::new(Expr::And(
                Box::new(never.clone()),
                Box::new(trapping.clone()),
            )),
            Box::new(Expr::Literal(1)),
            Box::new(Expr::Literal(7)),
        );
        assert_eq!(
            evaluate_program(&program_with(Vec::new(), tail)).ok(),
            Some(7)
        );
        let always = Expr::Compare("==", Box::new(Expr::Literal(1)), Box::new(Expr::Literal(1)));
        let tail = Expr::If(
            Box::new(Expr::Or(Box::new(always), Box::new(trapping))),
            Box::new(Expr::Literal(5)),
            Box::new(Expr::Literal(1)),
        );
        assert_eq!(
            evaluate_program(&program_with(Vec::new(), tail)).ok(),
            Some(5)
        );
    }

    #[test]
    fn if_evaluates_only_the_taken_branch() {
        let tail = Expr::If(
            Box::new(Expr::Compare(
                "<",
                Box::new(Expr::Literal(1)),
                Box::new(Expr::Literal(2)),
            )),
            Box::new(Expr::Literal(5)),
            Box::new(divide_by_zero()),
        );
        assert_eq!(
            evaluate_program(&program_with(Vec::new(), tail)).ok(),
            Some(5)
        );
    }

    #[test]
    fn a_loop_runs_until_the_condition_is_false() {
        let body = vec![
            Stmt::Let {
                name: "i".into(),
                mutable: true,
                value: Expr::Literal(0),
            },
            Stmt::Let {
                name: "total".into(),
                mutable: true,
                value: Expr::Literal(0),
            },
            Stmt::While {
                condition: Expr::Compare(
                    "<",
                    Box::new(Expr::Var("i".into())),
                    Box::new(Expr::Literal(4)),
                ),
                body: vec![
                    Stmt::Compound {
                        name: "total".into(),
                        op: "+",
                        value: Expr::Var("i".into()),
                    },
                    Stmt::Compound {
                        name: "i".into(),
                        op: "+",
                        value: Expr::Literal(1),
                    },
                ],
            },
        ];
        assert_eq!(
            evaluate_program(&program_with(body, Expr::Var("total".into()))).ok(),
            Some(6)
        );
    }

    #[test]
    fn a_loop_that_never_finishes_makes_the_program_unusable() {
        let body = vec![Stmt::While {
            condition: Expr::Compare("==", Box::new(Expr::Literal(1)), Box::new(Expr::Literal(1))),
            body: Vec::new(),
        }];
        let result = evaluate_program(&program_with(body, Expr::Literal(0)));
        assert!(matches!(result, Err(Failure::Unusable(_))));
    }

    #[test]
    fn negative_literals_avoid_an_out_of_range_literal() {
        assert_eq!(render_literal(-5, TYPES[4]), "(0i8 - 5i8)");
        assert_eq!(render_literal(-128, TYPES[4]), "(0i8 - 127i8 - 1i8)");
        assert_eq!(render_literal(7, TYPES[0]), "7u8");
    }

    #[test]
    fn if_expressions_are_parenthesised() {
        let expr = Expr::If(
            Box::new(Expr::Compare(
                "<",
                Box::new(Expr::Literal(1)),
                Box::new(Expr::Literal(2)),
            )),
            Box::new(Expr::Literal(3)),
            Box::new(Expr::Literal(4)),
        );
        assert!(render_expr(&expr, I32).starts_with("(if "));
    }

    #[test]
    fn a_seed_reproduces_the_same_program() {
        let first = render_program(&generate_program(9, 3));
        let second = render_program(&generate_program(9, 3));
        assert_eq!(first, second);
        assert_ne!(first, render_program(&generate_program(9, 4)));
    }

    #[test]
    fn generated_programs_avoid_the_known_gap_shapes() {
        for index in 1..40 {
            let program = generate_program(5, index);
            let source = render_program(&program);
            // Flat arrays and flat structs are generated; the nested forms
            // are not generated yet.
            assert!(
                !source.contains("Array<Array"),
                "nested array in {}",
                program.id
            );
        }
    }

    /// `if` as a statement and a block with its own scope were gaps g12 and
    /// g04. The generator has to produce both.
    #[test]
    fn generated_programs_cover_if_statements_and_blocks() {
        let mut if_statements = 0;
        let mut blocks = 0;
        for index in 1..60 {
            for seed in [5u64, 23] {
                let source = render_program(&generate_program(seed, index));
                for line in source.lines() {
                    let trimmed = line.trim();
                    if trimmed.starts_with("if ") && trimmed.ends_with('{') {
                        if_statements += 1;
                    }
                }
                blocks += source.matches("{ let ").count();
            }
        }
        assert!(if_statements > 0, "no `if` statement was generated");
        assert!(blocks > 0, "no block expression was generated");
    }

    /// A block's local shadows an outer one; the outer value must come back
    /// once the block ends.
    #[test]
    fn a_block_local_does_not_outlive_its_block() {
        let program = program_with(
            vec![
                Stmt::Let {
                    name: "v1".into(),
                    mutable: false,
                    value: Expr::Literal(10),
                },
                Stmt::Let {
                    name: "v2".into(),
                    mutable: false,
                    value: Expr::Block {
                        body: vec![Stmt::Let {
                            name: "v1".into(),
                            mutable: false,
                            value: Expr::Literal(7),
                        }],
                        tail: Box::new(Expr::Var("v1".into())),
                    },
                },
            ],
            Expr::Arith(
                "+",
                Box::new(Expr::Var("v1".into())),
                Box::new(Expr::Var("v2".into())),
            ),
        );
        assert_eq!(evaluate_program(&program).ok(), Some(17));
    }

    /// A branch assigns to an outer local; the assignment survives the
    /// branch, and the branch not taken changes nothing.
    #[test]
    fn an_if_statement_keeps_what_its_branch_assigned() {
        let taken = |condition: bool| {
            program_with(
                vec![
                    Stmt::Let {
                        name: "v1".into(),
                        mutable: true,
                        value: Expr::Literal(1),
                    },
                    Stmt::IfStatement {
                        condition: Expr::Compare(
                            "==",
                            Box::new(Expr::Literal(i128::from(condition))),
                            Box::new(Expr::Literal(1)),
                        ),
                        then_body: vec![Stmt::Assign {
                            name: "v1".into(),
                            value: Expr::Literal(42),
                        }],
                        else_body: None,
                    },
                ],
                Expr::Var("v1".into()),
            )
        };
        assert_eq!(evaluate_program(&taken(true)).ok(), Some(42));
        assert_eq!(evaluate_program(&taken(false)).ok(), Some(1));
    }

    /// Shifts and signed negation were gaps g13 and g14 until #37. The
    /// generator now has to produce them, so a regression is caught by the
    /// oracle and not only by the fixed corpus.
    #[test]
    fn generated_programs_cover_shifts_and_negation() {
        let mut shifts_left = 0;
        let mut shifts_right = 0;
        let mut negations = 0;
        for index in 1..60 {
            for seed in [5u64, 23] {
                let source = render_program(&generate_program(seed, index));
                shifts_left += source.matches("<<").count();
                shifts_right += source.matches(">>").count();
                negations += source.matches("-(").count();
            }
        }
        assert!(shifts_left > 0, "no left shift was generated");
        assert!(shifts_right > 0, "no right shift was generated");
        assert!(negations > 0, "no negation was generated");
    }

    /// An unsigned program never negates: the backend has no representable
    /// result for it, and the frontend would accept the program.
    #[test]
    fn unsigned_programs_never_negate() {
        for index in 1..80 {
            let program = generate_program(31, index);
            if !unsigned(program.kind) {
                continue;
            }
            let source = render_program(&program);
            for line in source.lines() {
                assert!(
                    !line.contains("-("),
                    "unsigned negation in {}: {line}",
                    program.id
                );
            }
        }
    }

    #[test]
    fn shifts_trap_only_on_the_amount() {
        // `spec/core/evaluation.md`: the amount must be below the width.
        assert_eq!(shift("<<", 1, 7, TYPES[0]).ok(), Some(128));
        assert!(matches!(
            shift("<<", 1, 8, TYPES[0]),
            Err(Failure::Trapped(Trap::ShiftOutOfRange))
        ));
        assert!(matches!(
            shift(">>", 1, 64, TYPES[3]),
            Err(Failure::Trapped(Trap::ShiftOutOfRange))
        ));
        assert!(matches!(
            shift("<<", 1, -1, TYPES[6]),
            Err(Failure::Trapped(Trap::ShiftOutOfRange))
        ));
        // Bits that leave the width are dropped; they are not an overflow.
        assert_eq!(shift("<<", 255, 1, TYPES[0]).ok(), Some(254));
        assert_eq!(shift("<<", 1, 7, TYPES[4]).ok(), Some(-128));
        // The signed right shift keeps the sign.
        assert_eq!(shift(">>", -8, 2, TYPES[6]).ok(), Some(-2));
        assert_eq!(shift(">>", -1, 63, TYPES[7]).ok(), Some(-1));
        // The unsigned right shift does not.
        assert_eq!(shift(">>", 255, 4, TYPES[0]).ok(), Some(15));
    }

    #[test]
    fn negation_overflows_only_at_the_minimum() {
        assert_eq!(negate(42, TYPES[6]).ok(), Some(-42));
        assert_eq!(negate(0, TYPES[6]).ok(), Some(0));
        assert!(matches!(
            negate(-128, TYPES[4]),
            Err(Failure::Trapped(Trap::IntegerOverflow))
        ));
        assert!(matches!(
            negate(i64::MIN.into(), TYPES[7]),
            Err(Failure::Trapped(Trap::IntegerOverflow))
        ));
    }

    #[test]
    fn a_shift_amount_reaching_the_width_is_a_trap_in_a_whole_program() {
        let program = program_with(
            vec![],
            Expr::Shift(
                "<<",
                Box::new(Expr::Literal(1)),
                Box::new(Expr::Literal(32)),
            ),
        );
        let (_, stderr, exit) = expected_case(&program).expect("the program finishes");
        assert_eq!(exit, 70);
        assert_eq!(stderr, "ARGORIX_TRAP:SHIFT_OUT_OF_RANGE");
    }

    #[test]
    fn every_generated_program_evaluates_to_a_result_or_a_trap() {
        let mut results = 0;
        let mut traps = 0;
        for index in 1..60 {
            match expected_case(&generate_program(11, index)) {
                Some((stdout, _, 0)) => {
                    assert!(stdout.starts_with("ARGORIX_RESULT:"));
                    results += 1;
                }
                Some((_, stderr, 70)) => {
                    assert!(stderr.starts_with("ARGORIX_TRAP:"));
                    traps += 1;
                }
                Some(_) => panic!("unexpected exit code"),
                None => {}
            }
        }
        assert!(
            results > 0 && traps > 0,
            "corpus should hold both results and traps"
        );
    }
}

#[cfg(test)]
mod aggregate_tests {
    use super::*;

    const U32: IntType = TYPES[2];

    fn program(structs: Vec<(String, usize)>, body: Vec<Stmt>, tail: Expr) -> Program {
        Program {
            id: "t".into(),
            kind: U32,
            structs,
            enums: Vec::new(),
            functions: Vec::new(),
            main: Function {
                name: "argorix_main".into(),
                params: Vec::new(),
                body,
                tail,
            },
        }
    }

    fn array_body(length: usize, index: u64) -> Vec<Stmt> {
        vec![
            Stmt::LetArray {
                name: "a1".into(),
                elements: (0..length)
                    .map(|item| Expr::Literal(10 + item as i128))
                    .collect(),
            },
            Stmt::LetIndex {
                name: "x1".into(),
                value: index,
            },
        ]
    }

    fn read_array() -> Expr {
        Expr::Index {
            array: "a1".into(),
            index: "x1".into(),
        }
    }

    #[test]
    fn an_index_inside_the_array_reads_that_element() {
        let result = evaluate_program(&program(Vec::new(), array_body(3, 2), read_array()));
        assert_eq!(result.ok(), Some(12));
    }

    #[test]
    fn an_index_past_the_end_traps() {
        for index in [3, 4, 99] {
            let result = evaluate_program(&program(Vec::new(), array_body(3, index), read_array()));
            assert!(
                matches!(result, Err(Failure::Trapped(Trap::IndexOutOfBounds))),
                "index {index} should trap"
            );
        }
    }

    #[test]
    fn an_element_that_traps_is_reported_before_the_index_is_used() {
        let body = vec![
            Stmt::LetArray {
                name: "a1".into(),
                elements: vec![Expr::Arith(
                    "/",
                    Box::new(Expr::Literal(1)),
                    Box::new(Expr::Literal(0)),
                )],
            },
            Stmt::LetIndex {
                name: "x1".into(),
                value: 9,
            },
        ];
        let result = evaluate_program(&program(Vec::new(), body, read_array()));
        assert!(matches!(
            result,
            Err(Failure::Trapped(Trap::DivisionByZero))
        ));
    }

    #[test]
    fn a_struct_field_reads_its_own_value() {
        let body = vec![Stmt::LetStruct {
            name: "s1".into(),
            type_name: "S1".into(),
            fields: vec![Expr::Literal(7), Expr::Literal(35)],
        }];
        let tail = Expr::Field {
            value: "s1".into(),
            field: 1,
        };
        let result = evaluate_program(&program(vec![("S1".into(), 2)], body, tail));
        assert_eq!(result.ok(), Some(35));
    }

    #[test]
    fn aggregates_render_as_core_declarations() {
        let body = [
            array_body(2, 0).as_slice(),
            &[Stmt::LetStruct {
                name: "s1".into(),
                type_name: "S1".into(),
                fields: vec![Expr::Literal(1), Expr::Literal(2)],
            }],
        ]
        .concat();
        let source = render_program(&program(vec![("S1".into(), 2)], body, read_array()));
        assert!(source.contains("struct S1 { f0: u32, f1: u32, }"));
        assert!(source.contains("let a1: Array<u32, 2> = [10u32, 11u32];"));
        assert!(source.contains("let x1: u64 = 0u64;"));
        assert!(source.contains("let s1: S1 = S1 { f0: 1u32, f1: 2u32 };"));
        assert!(source.contains("a1[x1]"));
    }

    #[test]
    fn generated_programs_keep_aggregates_flat_and_out_of_blocks() {
        for index in 1..60 {
            let program = generate_program(7, index);
            let source = render_program(&program);
            assert!(
                !source.contains("Array<Array"),
                "nested array in {}",
                program.id
            );
            for line in source.lines() {
                let trimmed = line.trim();
                if trimmed.starts_with("struct ") {
                    // A struct of scalars only: no field takes another struct.
                    assert!(
                        !program
                            .structs
                            .iter()
                            .any(|(name, _)| trimmed.contains(&format!(": {name}"))),
                        "struct inside struct in {}: {line}",
                        program.id
                    );
                }
                if trimmed.starts_with("let a") && trimmed.contains("Array<") {
                    assert!(
                        !line.starts_with("        "),
                        "array declared inside a block in {}: {line}",
                        program.id
                    );
                }
            }
        }
    }
}

#[cfg(test)]
mod memory_tests {
    use super::*;

    const U32: IntType = TYPES[2];

    fn program(structs: Vec<(String, usize)>, body: Vec<Stmt>, tail: Expr) -> Program {
        Program {
            id: "t".into(),
            kind: U32,
            structs,
            enums: Vec::new(),
            functions: Vec::new(),
            main: Function {
                name: "argorix_main".into(),
                params: Vec::new(),
                body,
                tail,
            },
        }
    }

    fn buffer_body(pushes: &[i128], index: u64) -> Vec<Stmt> {
        let mut body = vec![Stmt::LetBuffer { name: "b1".into() }];
        for value in pushes {
            body.push(Stmt::Push {
                name: "b1".into(),
                value: Expr::Literal(*value),
            });
        }
        body.push(Stmt::LetIndex {
            name: "x1".into(),
            value: index,
        });
        body
    }

    fn read_buffer() -> Expr {
        Expr::Index {
            array: "b1".into(),
            index: "x1".into(),
        }
    }

    fn arena_body(release: bool) -> Vec<Stmt> {
        let mut body = vec![
            Stmt::LetArena {
                name: "r1".into(),
                type_name: "N1".into(),
            },
            Stmt::Alloc {
                handle: "h1".into(),
                arena: "r1".into(),
                type_name: "N1".into(),
                fields: vec![Expr::Literal(41), Expr::Literal(42)],
            },
        ];
        if release {
            body.push(Stmt::Release { arena: "r1".into() });
        }
        body
    }

    fn read_handle(field: usize) -> Expr {
        Expr::HandleField {
            handle: "h1".into(),
            field,
        }
    }

    #[test]
    fn a_buffer_grows_with_each_push() {
        let result = evaluate_program(&program(
            Vec::new(),
            buffer_body(&[10, 20, 30], 2),
            read_buffer(),
        ));
        assert_eq!(result.ok(), Some(30));
    }

    #[test]
    fn reading_past_the_last_push_traps() {
        let result = evaluate_program(&program(
            Vec::new(),
            buffer_body(&[10, 20], 2),
            read_buffer(),
        ));
        assert!(matches!(
            result,
            Err(Failure::Trapped(Trap::IndexOutOfBounds))
        ));
    }

    #[test]
    fn a_handle_reads_its_allocation_while_the_arena_lives() {
        let result = evaluate_program(&program(
            vec![("N1".into(), 2)],
            arena_body(false),
            read_handle(1),
        ));
        assert_eq!(result.ok(), Some(42));
    }

    #[test]
    fn a_handle_into_a_released_arena_traps() {
        let result = evaluate_program(&program(
            vec![("N1".into(), 2)],
            arena_body(true),
            read_handle(0),
        ));
        assert!(matches!(result, Err(Failure::Trapped(Trap::ArenaReleased))));
    }

    #[test]
    fn memory_statements_render_as_core_declarations() {
        let mut body = buffer_body(&[7], 0);
        body.extend(arena_body(true));
        let source = render_program(&program(vec![("N1".into(), 2)], body, read_buffer()));
        assert!(source.contains("let mut b1: Buffer<u32> = Buffer::new();"));
        assert!(source.contains("b1.push(7u32);"));
        assert!(source.contains("let mut r1: Arena<N1> = Arena::new();"));
        assert!(source.contains("let h1: Handle<N1> = r1.alloc(N1 { f0: 41u32, f1: 42u32 });"));
        assert!(source.contains("r1.release();"));
    }

    #[test]
    fn generated_programs_exercise_buffers_and_arenas() {
        let mut buffers = 0;
        let mut arenas = 0;
        let mut releases = 0;
        for index in 1..80 {
            let source = render_program(&generate_program(13, index));
            if source.contains("Buffer::new()") {
                buffers += 1;
            }
            if source.contains("Arena::new()") {
                arenas += 1;
            }
            if source.contains(".release();") {
                releases += 1;
            }
        }
        assert!(
            buffers > 0 && arenas > 0,
            "buffers {buffers}, arenas {arenas}"
        );
        assert!(releases > 0, "some arena should be released");
    }
}

#[cfg(test)]
mod enum_tests {
    use super::*;

    const U32: IntType = TYPES[2];

    fn declaration(variants: Vec<Option<i128>>) -> EnumDecl {
        EnumDecl {
            name: "E1".into(),
            reader: "read0".into(),
            variants,
        }
    }

    fn program(declaration: EnumDecl, variant: usize, payload: Option<Expr>) -> Program {
        Program {
            id: "t".into(),
            kind: U32,
            structs: Vec::new(),
            enums: vec![declaration],
            functions: Vec::new(),
            main: Function {
                name: "argorix_main".into(),
                params: Vec::new(),
                body: vec![Stmt::LetEnum {
                    name: "e1".into(),
                    type_name: "E1".into(),
                    variant,
                    payload,
                }],
                tail: Expr::ReadEnum {
                    declaration: 0,
                    reader: "read0".into(),
                    value: "e1".into(),
                },
            },
        }
    }

    #[test]
    fn the_arm_of_the_variant_with_a_field_answers_with_it() {
        let result = evaluate_program(&program(
            declaration(vec![Some(5), None]),
            1,
            Some(Expr::Literal(31)),
        ));
        assert_eq!(result.ok(), Some(31));
    }

    #[test]
    fn a_fieldless_variant_answers_with_its_own_arm() {
        let result = evaluate_program(&program(declaration(vec![Some(5), None]), 0, None));
        assert_eq!(result.ok(), Some(5));
    }

    #[test]
    fn a_payload_that_traps_is_reported_when_the_value_is_built() {
        let trapping = Expr::Arith("/", Box::new(Expr::Literal(1)), Box::new(Expr::Literal(0)));
        let result = evaluate_program(&program(
            declaration(vec![Some(5), None]),
            1,
            Some(trapping),
        ));
        assert!(matches!(
            result,
            Err(Failure::Trapped(Trap::DivisionByZero))
        ));
    }

    #[test]
    fn the_reader_matches_every_variant() {
        let source = render_program(&program(
            declaration(vec![Some(5), None, Some(9)]),
            1,
            Some(Expr::Literal(2)),
        ));
        assert!(source.contains("enum E1 { V0, V1 { p0: u32, }, V2, }"));
        assert!(source.contains("fn read0(value: E1) -> u32 {"));
        assert!(source.contains("E1::V0 => 5u32,"));
        assert!(source.contains("E1::V1 { p0 } => p0,"));
        assert!(source.contains("E1::V2 => 9u32,"));
        assert!(source.contains("let e1: E1 = E1::V1 { p0: 2u32 };"));
        assert!(source.contains("read0(e1)"));
    }

    #[test]
    fn a_fieldless_variant_renders_without_a_payload() {
        let source = render_program(&program(declaration(vec![Some(5), None]), 0, None));
        assert!(source.contains("let e1: E1 = E1::V0;"));
    }

    #[test]
    fn generated_enums_always_have_both_kinds_of_arm() {
        let mut seen = 0;
        for index in 1..80 {
            let program = generate_program(23, index);
            for declaration in &program.enums {
                seen += 1;
                assert!(
                    declaration.variants.iter().any(|item| item.is_none()),
                    "{} has no variant with a field",
                    declaration.name
                );
                assert!(
                    declaration.variants.iter().any(|item| item.is_some()),
                    "{} has no fieldless variant",
                    declaration.name
                );
            }
        }
        assert!(seen > 0, "some program should declare an enum");
    }
}
