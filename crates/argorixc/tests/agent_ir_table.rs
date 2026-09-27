//! ESP-018.D: the IR emitters of `compiler/agent_ir.argx`, generated from
//! `crates/argorix_ir/src/ir.rs`.
//!
//! Stage0's `IrProgram::from(&Program)` is one struct literal per IR type,
//! and nearly every field is one of a few shapes: a value, an enum's source
//! name, an optional value, a list of values, a nested list or option of
//! another IR type, or a constant. This test reads the IR types (their
//! fields, in order, with their serde attributes) and the literals, and
//! writes an emitter per IR type between the markers below, calling the
//! helpers of `compiler/agent_ir.argx`. A field of any other shape calls a
//! hand-written `special_<type>_<field>`, and must be listed in `SPECIAL`.
//!
//! The test fails when the region is not what this gives; `ARGORIX_BLESS=1`
//! rewrites it.

use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::{env, fs};

const BEGIN: &str =
    "// ------------------------------------------------------------------ generated IR emitters\n";
const END: &str =
    "// ------------------------------------------------------------------ end of generated IR emitters\n";

/// The fields written by hand, as `IrType.field`.
const SPECIAL: &[&str] = &[
    "IrProgram.modules",
    "IrProgram.imports",
    "IrPolicyRule.effect",
    "IrPolicyRule.rule",
    "IrTool.provider",
    "IrAgent.approval",
    "IrHandler.instructions",
];

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// `ATrustBoundaryDecl` as `A_TRUST_BOUNDARY_DECL`, as agent_schema.rs does.
fn upper_snake(name: &str) -> String {
    let characters: Vec<char> = name.chars().collect();
    let mut out = String::new();
    for (index, character) in characters.iter().enumerate() {
        if character.is_ascii_uppercase() && index > 0 {
            let previous = characters[index - 1];
            let next_lower = characters
                .get(index + 1)
                .is_some_and(|next| next.is_ascii_lowercase());
            if previous.is_ascii_lowercase()
                || previous.is_ascii_digit()
                || (previous.is_ascii_uppercase() && next_lower)
            {
                out.push('_');
            }
        }
        out.push(character.to_ascii_uppercase());
    }
    out
}

/// The text without whitespace outside string literals.
fn squeeze(text: &str) -> String {
    let mut out = String::new();
    let mut in_string = false;
    let mut escaped = false;
    for character in text.chars() {
        if in_string {
            out.push(character);
            if escaped {
                escaped = false;
            } else if character == '\\' {
                escaped = true;
            } else if character == '"' {
                in_string = false;
            }
            continue;
        }
        if character == '"' {
            in_string = true;
        }
        if !character.is_whitespace() {
            out.push(character);
        }
    }
    out
}

/// The text from `open` (a `(`, `[` or `{`) to its match, exclusive, and the
/// index after the match.
fn balanced(text: &str, open: usize) -> (String, usize) {
    let bytes = text.as_bytes();
    let mut depth = 0;
    let mut at = open;
    let mut in_string = false;
    while at < bytes.len() {
        let byte = bytes[at];
        if in_string {
            if byte == b'\\' {
                at += 2;
                continue;
            }
            if byte == b'"' {
                in_string = false;
            }
        } else if byte == b'"' {
            in_string = true;
        } else if matches!(byte, b'(' | b'[' | b'{') {
            depth += 1;
        } else if matches!(byte, b')' | b']' | b'}') {
            depth -= 1;
            if depth == 0 {
                return (text[open + 1..at].to_string(), at + 1);
            }
        }
        at += 1;
    }
    panic!("unbalanced text");
}

/// The top-level pieces of `text` split at commas outside any bracket or
/// closure bars.
fn split_top(text: &str) -> Vec<String> {
    let mut pieces = Vec::new();
    let mut depth = 0;
    let mut in_string = false;
    let mut start = 0;
    let bytes = text.as_bytes();
    let mut at = 0;
    while at < bytes.len() {
        let byte = bytes[at];
        if in_string {
            if byte == b'\\' {
                at += 2;
                continue;
            }
            if byte == b'"' {
                in_string = false;
            }
        } else if byte == b'"' {
            in_string = true;
        } else if matches!(byte, b'(' | b'[' | b'{') {
            depth += 1;
        } else if matches!(byte, b')' | b']' | b'}') {
            depth -= 1;
        } else if byte == b',' && depth == 0 {
            pieces.push(text[start..at].to_string());
            start = at + 1;
        }
        at += 1;
    }
    if start < text.len() {
        pieces.push(text[start..].to_string());
    }
    pieces
}

/// An IR field: its Rust name, the JSON key, and its serde attributes.
#[derive(Clone)]
struct IrField {
    name: String,
    key: String,
    skip_none: bool,
    skip_empty: bool,
}

/// The IR structs, in order, with their fields.
fn ir_structs(text: &str) -> Vec<(String, Vec<IrField>)> {
    let lines: Vec<&str> = text.lines().collect();
    let mut structs = Vec::new();
    let mut index = 0;
    while index < lines.len() {
        if let Some(rest) = lines[index].strip_prefix("pub struct ") {
            let name = rest.trim_end_matches(" {").to_string();
            let mut fields = Vec::new();
            let (mut skip_none, mut skip_empty, mut rename) = (false, false, None);
            index += 1;
            while lines[index] != "}" {
                let line = lines[index].trim();
                if line.starts_with("#[serde(") {
                    skip_none |= line.contains("Option::is_none");
                    skip_empty |= line.contains("Vec::is_empty");
                    if let Some(rest) = line.strip_prefix("#[serde(rename = \"") {
                        rename = Some(rest.split('"').next().unwrap().to_string());
                    }
                } else if let Some(field) = line.strip_prefix("pub ") {
                    let field_name = field.split(':').next().unwrap().trim().to_string();
                    fields.push(IrField {
                        key: rename.take().unwrap_or_else(|| field_name.clone()),
                        name: field_name,
                        skip_none,
                        skip_empty,
                    });
                    skip_none = false;
                    skip_empty = false;
                }
                index += 1;
            }
            structs.push((name, fields));
        }
        index += 1;
    }
    structs
}

/// The structs of ast.rs with their fields and types, by name.
fn ast_structs() -> Vec<(String, Vec<(String, String)>)> {
    let text = fs::read_to_string(root().join("crates/argorix_parser/src/ast.rs")).unwrap();
    let lines: Vec<&str> = text.lines().collect();
    let mut structs = Vec::new();
    let mut index = 0;
    while index < lines.len() {
        if let Some(rest) = lines[index].strip_prefix("pub struct ") {
            let name = rest.trim_end_matches(" {").to_string();
            let mut fields = Vec::new();
            index += 1;
            while lines[index] != "}" {
                let field = lines[index].trim().trim_start_matches("pub ");
                if let Some((field_name, field_type)) = field.split_once(':') {
                    fields.push((
                        field_name.trim().to_string(),
                        field_type.trim().trim_end_matches(',').to_string(),
                    ));
                }
                index += 1;
            }
            structs.push((name, fields));
        }
        index += 1;
    }
    structs
}

/// `Vec<X>` or `Option<X>` as `X`.
fn element(field_type: &str) -> String {
    field_type
        .trim_start_matches("Vec<")
        .trim_start_matches("Option<")
        .trim_end_matches('>')
        .to_string()
}

/// How one IR field is written.
enum Shape {
    Constant(String),
    Empty,
    Value(String),
    Raw(String),
    Name(String),
    OptionValue(String),
    OptionName(String),
    Values(String),
    List(String, String, String, String),
    Optional(String, String, String, String),
    Special,
}

/// The shape of `expression`, a field of an IR literal over `binding`.
fn shape(expression: &str, binding: &str) -> Shape {
    let field = |rest: &str| -> Option<String> {
        let rest = rest.strip_prefix(binding)?.strip_prefix('.')?;
        let name: String = rest
            .chars()
            .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
            .collect();
        Some(name)
    };
    if let Some(rest) = expression.strip_prefix('"') {
        let text = rest.split('"').next().unwrap().to_string();
        let tail = &rest[text.len() + 1..];
        if matches!(tail, ".into()" | ".to_owned()" | ".to_string()") {
            return Shape::Constant(text);
        }
    }
    if expression == "Vec::new()" {
        return Shape::Empty;
    }
    if let Some(inner) = expression
        .strip_prefix("spanned_values(&")
        .and_then(|rest| rest.strip_suffix(')'))
    {
        if let Some(name) = field(inner) {
            if inner == format!("{binding}.{name}") {
                return Shape::Values(name);
            }
        }
    }
    let Some(name) = field(expression) else {
        return Shape::Special;
    };
    let rest = &expression[binding.len() + 1 + name.len()..];
    match rest {
        "" => return Shape::Raw(name),
        ".value" | ".value.clone()" => return Shape::Value(name),
        ".value.source_name()"
        | ".value.source_name().to_owned()"
        | ".value.source_name().to_string()"
        | ".value.source_name().into()"
        | ".value.as_str().into()"
        | ".value.as_str().to_owned()" => return Shape::Name(name),
        _ => {}
    }
    if let Some(closure) = rest
        .strip_prefix(".as_ref().map(|")
        .and_then(|value| value.strip_suffix(')'))
    {
        let (variable, body) = closure.split_once('|').unwrap();
        if body == format!("{variable}.value") || body == format!("{variable}.value.clone()") {
            return Shape::OptionValue(name);
        }
        if body == format!("{variable}.value.source_name().to_owned()") {
            return Shape::OptionName(name);
        }
        if let Some((ir, literal)) = literal_of(body) {
            return Shape::Optional(name, variable.to_string(), ir, literal);
        }
        return Shape::Special;
    }
    if let Some(closure) = rest
        .strip_prefix(".iter().map(|")
        .and_then(|value| value.strip_suffix(").collect()"))
    {
        let (variable, body) = closure.split_once('|').unwrap();
        if body == format!("{variable}.value.clone()") {
            return Shape::Values(name);
        }
        if let Some((ir, literal)) = literal_of(body) {
            return Shape::List(name, variable.to_string(), ir, literal);
        }
    }
    Shape::Special
}

/// `IrX{...}` as `IrX` and the literal's body.
fn literal_of(body: &str) -> Option<(String, String)> {
    literal_with(body, "Ir")
}

/// `<prefix>X{...}` as `<prefix>X` and the literal's body.
fn literal_with(body: &str, prefix: &str) -> Option<(String, String)> {
    if !body.starts_with(prefix) || !body.ends_with('}') {
        return None;
    }
    let brace = body.find('{')?;
    let ir = &body[..brace];
    if !ir.chars().all(|c| c.is_ascii_alphanumeric()) {
        return None;
    }
    let (inner, end) = balanced(body, brace);
    (end == body.len()).then(|| (ir.to_string(), inner))
}

/// `IrATrustBoundary` as `a_trust_boundary`.
fn function_name(ir: &str) -> String {
    upper_snake(ir.trim_start_matches("Ir")).to_ascii_lowercase()
}

struct Generator {
    ir: Vec<(String, Vec<IrField>)>,
    ast: Vec<(String, Vec<(String, String)>)>,
    /// Each IR literal once, by type: its AST struct, binding and body.
    literals: Vec<(String, String, String, String)>,
    specials: Vec<String>,
}

impl Generator {
    fn ast_field_type(&self, ast: &str, field: &str) -> String {
        let (_, fields) = self
            .ast
            .iter()
            .find(|(name, _)| name == ast)
            .unwrap_or_else(|| panic!("no AST struct `{ast}`"));
        fields
            .iter()
            .find(|(name, _)| name == field)
            .unwrap_or_else(|| panic!("no field `{field}` in `{ast}`"))
            .1
            .clone()
    }

    fn collect(&mut self, ir: &str, ast: &str, binding: &str, body: &str) {
        if self.literals.iter().any(|(name, ..)| name == ir) {
            return;
        }
        self.literals.push((
            ir.to_string(),
            ast.to_string(),
            binding.to_string(),
            body.to_string(),
        ));
        for piece in split_top(body) {
            let Some((_, expression)) = piece.split_once(':') else {
                continue;
            };
            match shape(expression, binding) {
                Shape::List(field, variable, inner, literal)
                | Shape::Optional(field, variable, inner, literal) => {
                    let element = element(&self.ast_field_type(ast, &field));
                    self.collect(&inner, &element, &variable, &literal);
                }
                _ => {}
            }
        }
    }

    fn emitter(&mut self, ir: &str, ast: &str, binding: &str, body: &str) -> String {
        let fields = self
            .ir
            .iter()
            .find(|(name, _)| name == ir)
            .unwrap_or_else(|| panic!("no IR struct `{ir}`"))
            .1
            .clone();
        let mut values = Vec::new();
        for piece in split_top(body) {
            let (name, expression) = piece.split_once(':').unwrap();
            values.push((name.to_string(), expression.to_string()));
        }
        let prefix = upper_snake(ast);
        let mut out = String::new();
        let _ = writeln!(
            out,
            "fn emit_{}(state: Pretty, sl: Slice<u64>, n: Slice<AgentNode>, st: Slice<u8>, node: u64) -> Pretty {{",
            function_name(ir)
        );
        out.push_str("    let mut y: Pretty = open(state, 123u8);\n");
        for field in &fields {
            let (_, expression) = values
                .iter()
                .find(|(name, _)| *name == field.name)
                .unwrap_or_else(|| panic!("`{ir}.{}` is not set", field.name));
            let found = shape(expression, binding);
            let nested = match &found {
                Shape::List(_, _, inner, _) | Shape::Optional(_, _, inner, _) => {
                    format!("IR_{}", upper_snake(inner.trim_start_matches("Ir")))
                }
                _ => String::new(),
            };
            self.write_field(&mut out, ir, &field.name, found, field, &prefix, &nested);
        }
        out.push_str("    close(y, 125u8)\n}\n");
        out
    }

    /// The lines writing one field of IR shape `found`, under `field`'s key
    /// and serde attributes; `nested` names the emitter of a list or option.
    /// A special is the hand-written `special_<owner>_<name>`.
    #[allow(clippy::too_many_arguments)]
    fn write_field(
        &mut self,
        out: &mut String,
        owner: &str,
        name: &str,
        found: Shape,
        field: &IrField,
        prefix: &str,
        nested: &str,
    ) {
        let key = format!("\"{}\".as_slice()", field.key);
        let constant = |name: &str| format!("agent_schema.{prefix}_{}", name.to_ascii_uppercase());
        let skip_none = if field.skip_none { "skip" } else { "null" };
        let skip_empty = if field.skip_empty { "_skip" } else { "" };
        match found {
            Shape::Constant(text) => {
                let _ = writeln!(out, "    y = key(y, {key});");
                let _ = writeln!(out, "    y = text(y, \"{text}\".as_slice());");
            }
            Shape::Empty => {
                assert!(field.skip_empty, "`{owner}.{name}` is always empty");
                self.special(owner, name, out, &key);
            }
            Shape::Value(ast) => {
                let _ = writeln!(out, "    y = key(y, {key});");
                let _ = writeln!(
                    out,
                    "    y = value(y, sl, n, st, get(sl, n, node, {}));",
                    constant(&ast)
                );
            }
            Shape::Raw(ast) => {
                let _ = writeln!(out, "    y = key(y, {key});");
                let _ = writeln!(
                    out,
                    "    y = scalar(y, sl, n, st, get(sl, n, node, {}));",
                    constant(&ast)
                );
            }
            Shape::Name(ast) => {
                let _ = writeln!(out, "    y = key(y, {key});");
                let _ = writeln!(
                    out,
                    "    y = name(y, sl, n, st, get(sl, n, node, {}));",
                    constant(&ast)
                );
            }
            Shape::OptionValue(ast) => {
                let _ = writeln!(
                    out,
                    "    y = optional_value_{skip_none}(y, sl, n, st, {key}, get(sl, n, node, {}));",
                    constant(&ast)
                );
            }
            Shape::OptionName(ast) => {
                let _ = writeln!(
                    out,
                    "    y = optional_name_{skip_none}(y, sl, n, st, {key}, get(sl, n, node, {}));",
                    constant(&ast)
                );
            }
            Shape::Values(ast) => {
                let _ = writeln!(
                    out,
                    "    y = values{skip_empty}(y, sl, n, st, {key}, get(sl, n, node, {}));",
                    constant(&ast)
                );
            }
            Shape::List(ast, ..) => {
                let _ = writeln!(
                    out,
                    "    y = list{skip_empty}(y, sl, n, st, {key}, get(sl, n, node, {}), {nested});",
                    constant(&ast)
                );
            }
            Shape::Optional(ast, ..) => {
                let _ = writeln!(
                    out,
                    "    y = optional_{skip_none}(y, sl, n, st, {key}, get(sl, n, node, {}), {nested});",
                    constant(&ast)
                );
            }
            Shape::Special => self.special(owner, name, out, &key),
        }
    }

    fn special(&mut self, ir: &str, field: &str, out: &mut String, key: &str) {
        let name = format!("{ir}.{field}");
        self.specials.push(name);
        let _ = writeln!(
            out,
            "    y = special_{}_{field}(y, sl, n, st, {key}, node);",
            function_name(ir)
        );
    }
}

fn generate() -> (String, Vec<String>) {
    let text = fs::read_to_string(root().join("crates/argorix_ir/src/ir.rs")).unwrap();
    let start = text.find("impl From<&Program> for IrProgram").unwrap();
    // The literal, not the `-> Self {` of the signature.
    let literal = "\n        Self {";
    let open = start + text[start..].find(literal).unwrap() + literal.len() - 1;
    let (body, _) = balanced(&text, open);
    let body = squeeze(&body);
    let mut generator = Generator {
        ir: ir_structs(&text),
        ast: ast_structs(),
        literals: Vec::new(),
        specials: Vec::new(),
    };
    generator.collect("IrProgram", "Program", "program", &body);
    let literals = generator.literals.clone();

    let mut out = String::new();
    out.push_str(BEGIN);
    out.push_str(
        "// GENERATED by crates/argorixc/tests/agent_ir_table.rs from the IR types and\n\
         // `IrProgram::from` of crates/argorix_ir/src/ir.rs. Do not edit by hand;\n\
         // ARGORIX_BLESS=1 regenerates it.\n\n",
    );
    for (index, (ir, ..)) in literals.iter().enumerate() {
        let _ = writeln!(
            out,
            "pub const IR_{}: u64 = {index}u64;",
            upper_snake(ir.trim_start_matches("Ir"))
        );
    }
    out.push_str(
        "\n// The emitter of IR type `s` over the tree node `node`.\n\
         fn emit(state: Pretty, sl: Slice<u64>, n: Slice<AgentNode>, st: Slice<u8>, s: u64, node: u64) -> Pretty {\n",
    );
    for (ir, ..) in &literals {
        let _ = writeln!(
            out,
            "    if s == IR_{} {{\n        return emit_{}(state, sl, n, st, node);\n    }}",
            upper_snake(ir.trim_start_matches("Ir")),
            function_name(ir)
        );
    }
    // A bytecode type's emitter otherwise, since both share these helpers.
    out.push_str("    emit_bytecode(state, sl, n, st, s, node)\n}\n");
    for (ir, ast, binding, body) in &literals {
        out.push('\n');
        out.push_str(&generator.emitter(ir, ast, binding, body));
    }
    out.push_str(END);
    (out, generator.specials)
}

#[test]
fn the_generated_ir_emitters_are_what_stage0_lowers() {
    let (region, specials) = generate();
    let mut unlisted: Vec<&String> = specials
        .iter()
        .filter(|name| !SPECIAL.contains(&name.as_str()))
        .collect();
    unlisted.dedup();
    assert!(
        unlisted.is_empty(),
        "fields of no known shape, to write by hand and list in SPECIAL: {unlisted:?}"
    );
    for listed in SPECIAL {
        assert!(
            specials.iter().any(|name| name == listed),
            "`{listed}` is in SPECIAL but has a known shape now"
        );
    }
    let path = root().join("compiler/agent_ir.argx");
    let text = fs::read_to_string(&path).unwrap();
    let begin = text.find(BEGIN).expect("the region's first marker");
    let end = text.find(END).expect("the region's last marker") + END.len();
    let current = &text[begin..end];
    if current != region {
        if env::var_os("ARGORIX_BLESS").is_some() {
            let updated = format!("{}{region}{}", &text[..begin], &text[end..]);
            fs::write(&path, updated).unwrap();
            return;
        }
        panic!(
            "compiler/agent_ir.argx's generated IR emitters are stale; run with ARGORIX_BLESS=1"
        );
    }
}

// ------------------------------------------------------------------ bytecode

const BC_BEGIN: &str =
    "// ------------------------------------------------------------------ generated bytecode emitters\n";
const BC_END: &str =
    "// ------------------------------------------------------------------ end of generated bytecode emitters\n";

/// The bytecode fields written by hand, as `BytecodeType.field`.
const BC_SPECIAL: &[&str] = &[
    "BytecodeProgram.source_digest",
    "BytecodeProgram.enums",
    "BytecodeProgram.instructions",
];

/// How one field of `lower_ir`'s `BytecodeProgram` literal is written.
enum BcShape {
    Constant(String),
    Literal(String),
    Null,
    /// A copy of the IR field.
    Copy(String),
    /// The IR field's list or option, each element as a bytecode type.
    CopyList(String, String, String, String),
    CopyOptional(String, String, String, String),
    Special,
}

fn bc_shape(expression: &str, binding: &str) -> BcShape {
    if expression == "None" {
        return BcShape::Null;
    }
    if expression == "true" || expression == "false" {
        return BcShape::Literal(expression.to_string());
    }
    if let Some(rest) = expression.strip_prefix('"') {
        let text = rest.split('"').next().unwrap().to_string();
        let tail = &rest[text.len() + 1..];
        if matches!(tail, ".into()" | ".to_owned()" | ".to_string()") {
            return BcShape::Constant(text);
        }
    }
    let Some(rest) = expression
        .strip_prefix(binding)
        .and_then(|rest| rest.strip_prefix('.'))
    else {
        return BcShape::Special;
    };
    let name: String = rest
        .chars()
        .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
        .collect();
    let tail = &rest[name.len()..];
    if tail.is_empty() || tail == ".clone()" {
        return BcShape::Copy(name);
    }
    if let Some(closure) = tail
        .strip_prefix(".iter().map(|")
        .and_then(|value| value.strip_suffix(").collect()"))
    {
        let (variable, body) = closure.split_once('|').unwrap();
        if let Some((bc, literal)) = literal_with(body, "Bytecode") {
            return BcShape::CopyList(name, variable.to_string(), bc, literal);
        }
    }
    if let Some(closure) = tail
        .strip_prefix(".as_ref().map(|")
        .and_then(|value| value.strip_suffix(')'))
    {
        let (variable, body) = closure.split_once('|').unwrap();
        let body = body
            .strip_prefix('{')
            .and_then(|inner| inner.strip_suffix('}'))
            .unwrap_or(body);
        if let Some((bc, literal)) = literal_with(body, "Bytecode") {
            return BcShape::CopyOptional(name, variable.to_string(), bc, literal);
        }
    }
    BcShape::Special
}

fn bc_constant(bc: &str) -> String {
    format!("BC_{}", upper_snake(bc.trim_start_matches("Bytecode")))
}

fn bc_function(bc: &str) -> String {
    upper_snake(bc.trim_start_matches("Bytecode")).to_ascii_lowercase()
}

/// The pieces of a literal's body as `(field, expression)`; a shorthand
/// field has an empty expression.
fn fields_of(body: &str) -> Vec<(String, String)> {
    split_top(body)
        .into_iter()
        .map(|piece| match piece.split_once(':') {
            Some((name, expression)) => (name.to_string(), expression.to_string()),
            None => (piece.clone(), String::new()),
        })
        .collect()
}

/// A bytecode literal to emit: its type, the IR type it lowers, its
/// binding and its body.
type BcLiteral = (String, String, String, String);

impl Generator {
    fn ir_literal(&self, ir: &str) -> (String, String, String) {
        let (_, ast, binding, body) = self
            .literals
            .iter()
            .find(|(name, ..)| name == ir)
            .unwrap_or_else(|| panic!("no IR literal `{ir}`"));
        (ast.clone(), binding.clone(), body.clone())
    }

    fn ir_shape(&self, ir: &str, field: &str) -> Shape {
        let (_, binding, body) = self.ir_literal(ir);
        let (_, expression) = fields_of(&body)
            .into_iter()
            .find(|(name, _)| name == field)
            .unwrap_or_else(|| panic!("`{ir}.{field}` is not set"));
        shape(&expression, &binding)
    }

    fn collect_bc(
        &self,
        found: &mut Vec<BcLiteral>,
        bc: &str,
        ir: &str,
        binding: &str,
        body: &str,
    ) {
        if found.iter().any(|(name, ..)| name == bc) {
            return;
        }
        found.push((bc.into(), ir.into(), binding.into(), body.into()));
        for (_, expression) in fields_of(body) {
            if let BcShape::CopyList(field, variable, inner, literal)
            | BcShape::CopyOptional(field, variable, inner, literal) =
                bc_shape(&expression, binding)
            {
                match self.ir_shape(ir, &field) {
                    Shape::List(_, _, ir_inner, _) | Shape::Optional(_, _, ir_inner, _) => {
                        self.collect_bc(found, &inner, &ir_inner, &variable, &literal);
                    }
                    // Written by hand in the IR (a package's modules).
                    Shape::Empty => {}
                    _ => panic!("`{bc}.{field}` maps over an IR field that is no list or option"),
                }
            }
        }
    }

    fn bc_emitter(
        &mut self,
        structs: &[(String, Vec<IrField>)],
        bc: &str,
        ir: &str,
        binding: &str,
        body: &str,
    ) -> String {
        let fields = &structs
            .iter()
            .find(|(name, _)| name == bc)
            .unwrap_or_else(|| panic!("no bytecode struct `{bc}`"))
            .1;
        let values = fields_of(body);
        let (ast, _, _) = self.ir_literal(ir);
        let prefix = upper_snake(&ast);
        let mut out = String::new();
        let _ = writeln!(
            out,
            "fn emit_bc_{}(state: Pretty, sl: Slice<u64>, n: Slice<AgentNode>, st: Slice<u8>, node: u64) -> Pretty {{",
            bc_function(bc)
        );
        out.push_str("    let mut y: Pretty = open(state, 123u8);\n");
        for field in fields {
            let (_, expression) = values
                .iter()
                .find(|(name, _)| *name == field.name)
                .unwrap_or_else(|| panic!("`{bc}.{}` is not set", field.name));
            let key = format!("\"{}\".as_slice()", field.key);
            // The source digest is `None` here and set by `emit-bytecode`.
            let listed = BC_SPECIAL.contains(&format!("{bc}.{}", field.name).as_str());
            let found = if expression.is_empty() || listed {
                BcShape::Special
            } else {
                bc_shape(expression, binding)
            };
            match found {
                BcShape::Constant(text) => {
                    let _ = writeln!(out, "    y = key(y, {key});");
                    let _ = writeln!(out, "    y = text(y, \"{text}\".as_slice());");
                }
                BcShape::Literal(value) => {
                    let _ = writeln!(out, "    y = key(y, {key});");
                    let _ = writeln!(out, "    y = word(y, \"{value}\".as_slice());");
                }
                BcShape::Null => {
                    if !field.skip_none {
                        let _ = writeln!(out, "    y = key(y, {key});");
                        out.push_str("    y = word(y, \"null\".as_slice());\n");
                    }
                }
                BcShape::Copy(name) => {
                    let ir_found = self.ir_shape(ir, &name);
                    let nested = match &ir_found {
                        Shape::List(_, _, inner, _) | Shape::Optional(_, _, inner, _) => {
                            format!("IR_{}", upper_snake(inner.trim_start_matches("Ir")))
                        }
                        _ => String::new(),
                    };
                    self.write_field(&mut out, ir, &name, ir_found, field, &prefix, &nested);
                }
                BcShape::CopyList(name, _, inner, _) | BcShape::CopyOptional(name, _, inner, _) => {
                    let ir_found = self.ir_shape(ir, &name);
                    let nested = bc_constant(&inner);
                    self.write_field(&mut out, ir, &name, ir_found, field, &prefix, &nested);
                }
                BcShape::Special => {
                    self.specials.push(format!("{bc}.{}", field.name));
                    let _ = writeln!(
                        out,
                        "    y = special_bc_{}_{}(y, sl, n, st, {key}, node);",
                        bc_function(bc),
                        field.name
                    );
                }
            }
        }
        out.push_str("    close(y, 125u8)\n}\n");
        out
    }
}

/// The bytecode region, from `lower_ir`'s `BytecodeProgram` literal.
fn generate_bytecode(generator: &mut Generator) -> String {
    let lower = fs::read_to_string(root().join("crates/argorix_bytecode/src/lower.rs")).unwrap();
    let start = lower.find("pub fn lower_ir").unwrap();
    let literal = "\n    BytecodeProgram {";
    let open = start + lower[start..].find(literal).unwrap() + literal.len() - 1;
    let (body, _) = balanced(&lower, open);
    let body = squeeze(&body);
    let text = fs::read_to_string(root().join("crates/argorix_bytecode/src/bytecode.rs")).unwrap();
    let structs = ir_structs(&text);
    let mut found = Vec::new();
    generator.collect_bc(&mut found, "BytecodeProgram", "IrProgram", "ir", &body);

    let mut out = String::new();
    out.push_str(BC_BEGIN);
    out.push_str(
        "// GENERATED by crates/argorixc/tests/agent_ir_table.rs from the bytecode types\n\
         // and `lower_ir` of crates/argorix_bytecode/src, composed with the IR emitters\n\
         // above. Do not edit by hand; ARGORIX_BLESS=1 regenerates it.\n\n",
    );
    for (index, (bc, ..)) in found.iter().enumerate() {
        let _ = writeln!(
            out,
            "pub const {}: u64 = {}u64;",
            bc_constant(bc),
            1000 + index
        );
    }
    out.push_str(
        "\n// The emitter of bytecode type `s` over the tree node `node`.\n\
         fn emit_bytecode(state: Pretty, sl: Slice<u64>, n: Slice<AgentNode>, st: Slice<u8>, s: u64, node: u64) -> Pretty {\n",
    );
    for (bc, ..) in &found {
        let _ = writeln!(
            out,
            "    if s == {} {{\n        return emit_bc_{}(state, sl, n, st, node);\n    }}",
            bc_constant(bc),
            bc_function(bc)
        );
    }
    out.push_str("    state\n}\n");
    for (bc, ir, binding, body) in &found {
        out.push('\n');
        out.push_str(&generator.bc_emitter(&structs, bc, ir, binding, body));
    }
    out.push_str(BC_END);
    out
}

#[test]
fn the_generated_bytecode_emitters_are_what_stage0_lowers() {
    let text = fs::read_to_string(root().join("crates/argorix_ir/src/ir.rs")).unwrap();
    let start = text.find("impl From<&Program> for IrProgram").unwrap();
    let literal = "\n        Self {";
    let open = start + text[start..].find(literal).unwrap() + literal.len() - 1;
    let (body, _) = balanced(&text, open);
    let mut generator = Generator {
        ir: ir_structs(&text),
        ast: ast_structs(),
        literals: Vec::new(),
        specials: Vec::new(),
    };
    generator.collect("IrProgram", "Program", "program", &squeeze(&body));
    let region = generate_bytecode(&mut generator);
    let unlisted: Vec<&String> = generator
        .specials
        .iter()
        .filter(|name| name.starts_with("Bytecode") && !BC_SPECIAL.contains(&name.as_str()))
        .collect();
    assert!(
        unlisted.is_empty(),
        "bytecode fields of no known shape, to write by hand and list in BC_SPECIAL: {unlisted:?}"
    );
    let path = root().join("compiler/agent_ir.argx");
    let text = fs::read_to_string(&path).unwrap();
    let begin = text
        .find(BC_BEGIN)
        .expect("the bytecode region's first marker");
    let end = text
        .find(BC_END)
        .expect("the bytecode region's last marker")
        + BC_END.len();
    if text[begin..end] != region {
        if env::var_os("ARGORIX_BLESS").is_some() {
            let updated = format!("{}{region}{}", &text[..begin], &text[end..]);
            fs::write(&path, updated).unwrap();
            return;
        }
        panic!(
            "compiler/agent_ir.argx's generated bytecode emitters are stale; run with ARGORIX_BLESS=1"
        );
    }
}
