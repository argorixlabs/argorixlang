//! The canonical AST dump of `spec/core/ast.md`.
//!
//! One line per node, in preorder: two spaces of indentation per level, a label,
//! and the node's byte span as `@start..end`. When parsing fails, the dump is
//! one line per diagnostic instead, in the `line:column: phase[Code]: message`
//! form. The Argorix parser (`compiler/parser.argx`) must produce the same
//! bytes for every input.

use crate::core::{
    parse_core_source, CoreAssignOp, CoreBinaryOp, CoreBlock, CoreExpr, CoreExprKind, CoreItem,
    CoreItemKind, CorePattern, CorePatternKind, CoreProgram, CoreStatement, CoreStatementKind,
    CoreType, CoreTypeKind, CoreUnaryOp,
};
use crate::span::Span;

/// The dump of `source`, which may be any bytes.
pub fn core_ast_dump(source: &[u8]) -> String {
    let text = match std::str::from_utf8(source) {
        Ok(text) => text,
        // The token dump already specifies how invalid UTF-8 is reported.
        Err(_) => return crate::core::core_token_dump(source),
    };
    match parse_core_source(text) {
        Err(diagnostics) => diagnostics
            .iter()
            .map(|diagnostic| format!("{diagnostic}\n"))
            .collect(),
        Ok(program) => {
            let mut dump = Dump::default();
            dump.program(&program);
            dump.out
        }
    }
}

#[derive(Default)]
struct Dump {
    out: String,
    depth: usize,
}

impl Dump {
    fn line(&mut self, label: &str, span: Span) {
        for _ in 0..self.depth {
            self.out.push_str("  ");
        }
        self.out.push_str(label);
        self.out
            .push_str(&format!(" @{}..{}\n", span.start, span.end));
    }

    fn nested(&mut self, label: &str, span: Span, children: impl FnOnce(&mut Self)) {
        self.line(label, span);
        self.depth += 1;
        children(self);
        self.depth -= 1;
    }

    fn program(&mut self, program: &CoreProgram) {
        let label = format!("Program {} {}", program.version.value, program.module.value);
        self.nested(&label, program.span, |dump| {
            for import in &program.imports {
                let mut label = format!("Import {}", import.path.value);
                if let Some(alias) = &import.alias {
                    label.push_str(&format!(" as {}", alias.value));
                }
                dump.line(&label, import.span);
            }
            for item in &program.items {
                dump.item(item);
            }
        });
    }

    fn item(&mut self, item: &CoreItem) {
        let public = if item.public { "pub " } else { "" };
        match &item.kind {
            CoreItemKind::Function(function) => {
                let label = format!("{public}Fn {}", function.name.value);
                self.nested(&label, item.span, |dump| {
                    for parameter in &function.parameters {
                        let label = format!("Param {}", parameter.name.value);
                        dump.nested(&label, parameter.span, |dump| dump.ty(&parameter.ty));
                    }
                    dump.ty(&function.return_type);
                    dump.block(&function.body);
                });
            }
            CoreItemKind::Struct(value) => {
                let label = format!("{public}Struct {}", value.name.value);
                self.nested(&label, item.span, |dump| {
                    for field in &value.fields {
                        let label = format!("Field {}", field.name.value);
                        dump.nested(&label, field.span, |dump| dump.ty(&field.ty));
                    }
                });
            }
            CoreItemKind::Enum(value) => {
                let label = format!("{public}Enum {}", value.name.value);
                self.nested(&label, item.span, |dump| {
                    for variant in &value.variants {
                        let label = format!("Variant {}", variant.name.value);
                        dump.nested(&label, variant.span, |dump| {
                            for field in &variant.fields {
                                let label = format!("Field {}", field.name.value);
                                dump.nested(&label, field.span, |dump| dump.ty(&field.ty));
                            }
                        });
                    }
                });
            }
            CoreItemKind::Const(value) => {
                let label = format!("{public}Const {}", value.name.value);
                self.nested(&label, item.span, |dump| {
                    dump.ty(&value.ty);
                    dump.expr(&value.value);
                });
            }
        }
    }

    fn ty(&mut self, ty: &CoreType) {
        match &ty.kind {
            CoreTypeKind::Named(name) => self.line(&format!("Type {name}"), ty.span),
            CoreTypeKind::Container {
                name,
                element,
                array_length,
            } => {
                let mut label = format!("Type {name}");
                if let Some(length) = array_length {
                    label.push_str(&format!(" {length}"));
                }
                self.nested(&label, ty.span, |dump| dump.ty(element));
            }
        }
    }

    fn block(&mut self, block: &CoreBlock) {
        self.nested("Block", block.span, |dump| {
            for statement in &block.statements {
                dump.statement(statement);
            }
            if let Some(tail) = &block.tail {
                dump.expr(tail);
            }
        });
    }

    fn statement(&mut self, statement: &CoreStatement) {
        let span = statement.span;
        match &statement.kind {
            CoreStatementKind::Let {
                mutable,
                name,
                annotation,
                value,
            } => {
                let label = if *mutable {
                    format!("Let mut {}", name.value)
                } else {
                    format!("Let {}", name.value)
                };
                self.nested(&label, span, |dump| {
                    if let Some(annotation) = annotation {
                        dump.ty(annotation);
                    }
                    dump.expr(value);
                });
            }
            CoreStatementKind::Assign {
                target,
                operator,
                value,
            } => {
                let symbol = match operator {
                    CoreAssignOp::Assign => "=",
                    CoreAssignOp::Add => "+=",
                    CoreAssignOp::Subtract => "-=",
                    CoreAssignOp::Multiply => "*=",
                    CoreAssignOp::Divide => "/=",
                    CoreAssignOp::Remainder => "%=",
                };
                self.nested(&format!("Assign {symbol}"), span, |dump| {
                    dump.expr(target);
                    dump.expr(value);
                });
            }
            CoreStatementKind::While { condition, body } => {
                self.nested("While", span, |dump| {
                    dump.expr(condition);
                    dump.block(body);
                });
            }
            CoreStatementKind::Break(value) => self.nested("Break", span, |dump| {
                if let Some(value) = value {
                    dump.expr(value);
                }
            }),
            CoreStatementKind::Continue => self.line("Continue", span),
            CoreStatementKind::Return(value) => self.nested("Return", span, |dump| {
                if let Some(value) = value {
                    dump.expr(value);
                }
            }),
            CoreStatementKind::Expr(value) => self.nested("Expr", span, |dump| dump.expr(value)),
        }
    }

    fn expr(&mut self, expression: &CoreExpr) {
        let span = expression.span;
        match &expression.kind {
            CoreExprKind::Integer { value, suffix } => {
                let mut label = format!("Integer {value}");
                if let Some(suffix) = suffix {
                    label.push_str(&format!(" {suffix}"));
                }
                self.line(&label, span);
            }
            CoreExprKind::String(value) => {
                let label = format!("String \"{}\"", crate::core::escape_json(value));
                self.line(&label, span);
            }
            CoreExprKind::Bool(value) => self.line(&format!("Bool {value}"), span),
            CoreExprKind::Unit => self.line("Unit", span),
            CoreExprKind::Path(path) => self.line(&format!("Path {}", path.join("::")), span),
            CoreExprKind::Aggregate { path, fields } => {
                self.nested(&format!("Aggregate {}", path.join("::")), span, |dump| {
                    for (name, value) in fields {
                        dump.nested(&format!("FieldValue {}", name.value), name.span, |dump| {
                            dump.expr(value)
                        });
                    }
                });
            }
            CoreExprKind::Array(values) => self.nested("Array", span, |dump| {
                for value in values {
                    dump.expr(value);
                }
            }),
            CoreExprKind::Block(block) => self.block(block),
            CoreExprKind::If {
                condition,
                then_block,
                else_expr,
            } => self.nested("If", span, |dump| {
                dump.expr(condition);
                dump.block(then_block);
                if let Some(other) = else_expr {
                    dump.expr(other);
                }
            }),
            CoreExprKind::Match { value, arms } => self.nested("Match", span, |dump| {
                dump.expr(value);
                for arm in arms {
                    let label = if arm.guard.is_some() {
                        "Arm guarded"
                    } else {
                        "Arm"
                    };
                    dump.nested(label, arm.span, |dump| {
                        dump.pattern(&arm.pattern);
                        if let Some(guard) = &arm.guard {
                            dump.expr(guard);
                        }
                        dump.expr(&arm.value);
                    });
                }
            }),
            CoreExprKind::Loop(block) => self.nested("Loop", span, |dump| dump.block(block)),
            CoreExprKind::Call { callee, arguments } => self.nested("Call", span, |dump| {
                dump.expr(callee);
                for argument in arguments {
                    dump.expr(argument);
                }
            }),
            CoreExprKind::Index { value, index } => self.nested("Index", span, |dump| {
                dump.expr(value);
                dump.expr(index);
            }),
            CoreExprKind::Field { value, name } => {
                self.nested(&format!("Field {}", name.value), span, |dump| {
                    dump.expr(value)
                })
            }
            CoreExprKind::Unary { operator, value } => {
                let symbol = match operator {
                    CoreUnaryOp::Not => "!",
                    CoreUnaryOp::Negate => "-",
                };
                self.nested(&format!("Unary {symbol}"), span, |dump| dump.expr(value));
            }
            CoreExprKind::Binary {
                left,
                operator,
                right,
            } => {
                let symbol = match operator {
                    CoreBinaryOp::Or => "||",
                    CoreBinaryOp::And => "&&",
                    CoreBinaryOp::Equal => "==",
                    CoreBinaryOp::NotEqual => "!=",
                    CoreBinaryOp::Less => "<",
                    CoreBinaryOp::LessEqual => "<=",
                    CoreBinaryOp::Greater => ">",
                    CoreBinaryOp::GreaterEqual => ">=",
                    CoreBinaryOp::BitOr => "|",
                    CoreBinaryOp::BitXor => "^",
                    CoreBinaryOp::BitAnd => "&",
                    CoreBinaryOp::ShiftLeft => "<<",
                    CoreBinaryOp::ShiftRight => ">>",
                    CoreBinaryOp::Add => "+",
                    CoreBinaryOp::Subtract => "-",
                    CoreBinaryOp::Multiply => "*",
                    CoreBinaryOp::Divide => "/",
                    CoreBinaryOp::Remainder => "%",
                };
                self.nested(&format!("Binary {symbol}"), span, |dump| {
                    dump.expr(left);
                    dump.expr(right);
                });
            }
        }
    }

    fn pattern(&mut self, pattern: &CorePattern) {
        let span = pattern.span;
        match &pattern.kind {
            CorePatternKind::Wildcard => self.line("Pattern _", span),
            CorePatternKind::Bool(value) => self.line(&format!("Pattern {value}"), span),
            CorePatternKind::Integer(value) => self.line(&format!("Pattern {value}"), span),
            CorePatternKind::Binding(name) => self.line(&format!("Pattern binding {name}"), span),
            CorePatternKind::Variant { path, fields } => {
                self.nested(&format!("Pattern {}", path.join("::")), span, |dump| {
                    for (name, nested) in fields {
                        dump.nested(&format!("PatternField {}", name.value), name.span, |dump| {
                            if let Some(nested) = nested {
                                dump.pattern(nested);
                            }
                        });
                    }
                });
            }
        }
    }
}
