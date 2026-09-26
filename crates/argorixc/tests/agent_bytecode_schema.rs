//! ESP-018.D.3: the bytecode types as the Argorix verifier reads them,
//! generated from `crates/argorix_bytecode/src/bytecode.rs`.
//!
//! `compiler/agent_bytecode_schema.argx` holds, for every bytecode struct and
//! every variant of `Instruction`, its fields in declaration order: the name
//! serde reads (after `rename`), the type, and what serde does when the field
//! is missing (an error, `None`, `Default::default()` or `default_provider`).
//! The typed JSON reader of `compiler/agent_verify.argx` deserializes a
//! `.argbc.json` file with these tables, as serde's derived code does. The
//! test fails when the module is not what this gives; `ARGORIX_BLESS=1`
//! rewrites it.

use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::{env, fs};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// `ATrustBoundary` as `A_TRUST_BOUNDARY`, as agent_schema.rs does.
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

struct Field {
    name: String,
    key: String,
    ty: String,
    /// `serde(default)`, `serde(default = "default_provider")`, or neither.
    default: Option<String>,
}

/// A struct, or an `Instruction` variant (`struct_name` is then
/// `Instruction::Variant`), with its fields.
struct Shape {
    name: String,
    fields: Vec<Field>,
}

fn read() -> (Vec<Shape>, Vec<(String, bool)>) {
    let text = fs::read_to_string(root().join("crates/argorix_bytecode/src/bytecode.rs")).unwrap();
    let lines: Vec<&str> = text.lines().collect();
    let mut structs = Vec::new();
    let mut variants = Vec::new();
    let mut index = 0;
    let field_of = |line: &str, attributes: &[String]| -> Option<Field> {
        let body = line.trim().trim_start_matches("pub ");
        let (name, ty) = body.split_once(':')?;
        let name = name.trim().to_string();
        let mut key = name.clone();
        let mut default = None;
        for attribute in attributes {
            if let Some(rest) = attribute.split("rename = \"").nth(1) {
                key = rest.split('"').next().unwrap().to_string();
            }
            if let Some(rest) = attribute.split("default = \"").nth(1) {
                default = Some(rest.split('"').next().unwrap().to_string());
            } else if attribute.contains("default") {
                default = Some(String::new());
            }
        }
        Some(Field {
            name,
            key,
            ty: ty.trim().trim_end_matches(',').to_string(),
            default,
        })
    };
    while index < lines.len() {
        let line = lines[index];
        if let Some(rest) = line.strip_prefix("pub struct ") {
            if !rest.ends_with(" {") {
                index += 1;
                continue;
            }
            let name = rest.trim_end_matches(" {").to_string();
            let mut fields = Vec::new();
            let mut attributes = Vec::new();
            index += 1;
            while lines[index] != "}" {
                let body = lines[index].trim();
                if body.starts_with("#[serde(") {
                    attributes.push(body.to_string());
                } else if body.starts_with("pub ") {
                    fields.extend(field_of(body, &attributes));
                    attributes.clear();
                }
                index += 1;
            }
            structs.push(Shape { name, fields });
        } else if line == "pub enum Instruction {" {
            index += 1;
            let mut attributes: Vec<String> = Vec::new();
            while lines[index] != "}" {
                let body = lines[index].trim();
                if body.starts_with("#[serde(") {
                    attributes.push(body.to_string());
                    index += 1;
                    continue;
                }
                let variant: String = body
                    .chars()
                    .take_while(|c| c.is_ascii_alphanumeric())
                    .collect();
                if body.ends_with('{') {
                    let mut fields = Vec::new();
                    let mut field_attributes = Vec::new();
                    index += 1;
                    while lines[index].trim() != "}," {
                        let field = lines[index].trim();
                        if field.starts_with("#[serde(") {
                            field_attributes.push(field.to_string());
                        } else {
                            fields.extend(field_of(field, &field_attributes));
                            field_attributes.clear();
                        }
                        index += 1;
                    }
                    structs.push(Shape {
                        name: format!("Instruction::{variant}"),
                        fields,
                    });
                    variants.push((variant, false));
                } else if !variant.is_empty() {
                    let other = attributes.iter().any(|a| a.contains("other"));
                    variants.push((variant, other));
                }
                attributes.clear();
                index += 1;
            }
        }
        index += 1;
    }
    (structs, variants)
}

/// The constant of a struct or variant.
fn constant(shape: &str) -> String {
    match shape.strip_prefix("Instruction::") {
        Some(variant) => format!("BS_OP_{}", upper_snake(variant)),
        None => format!("BS_{}", upper_snake(shape.trim_start_matches("Bytecode"))),
    }
}

/// A field's type as `kind + 16 * struct`, the kinds below.
fn type_code(ty: &str) -> String {
    let inner = |name: &str| constant(name);
    match ty {
        "String" => "T_STRING".to_string(),
        "bool" => "T_BOOL".to_string(),
        "Option<String>" => "T_OPTION_STRING".to_string(),
        "Option<bool>" => "T_OPTION_BOOL".to_string(),
        "Option<u64>" => "T_OPTION_U64".to_string(),
        "Vec<String>" => "T_STRINGS".to_string(),
        "Vec<Instruction>" => "T_INSTRUCTIONS".to_string(),
        _ => {
            if let Some(name) = ty.strip_prefix("Vec<").and_then(|t| t.strip_suffix('>')) {
                format!("T_LIST + 16u64 * {}", inner(name))
            } else if let Some(name) = ty.strip_prefix("Option<").and_then(|t| t.strip_suffix('>'))
            {
                format!("T_OPTION_STRUCT + 16u64 * {}", inner(name))
            } else {
                panic!("no type code for `{ty}`")
            }
        }
    }
}

fn module() -> String {
    let (shapes, variants) = read();
    let mut out = String::new();
    out.push_str(
        "core 0.1;\nmodule compiler.agent_bytecode_schema;\n\n\
         // GENERATED by crates/argorixc/tests/agent_bytecode_schema.rs from\n\
         // crates/argorix_bytecode/src/bytecode.rs. Do not edit by hand;\n\
         // ARGORIX_BLESS=1 regenerates it.\n//\n\
         // Every bytecode struct and `Instruction` variant with its fields in\n\
         // declaration order: the key serde reads, the type (`kind + 16 *\n\
         // struct`) and what a missing field becomes. `<STRUCT>__<FIELD>` is\n\
         // a field's index.\n\n\
         import stdlib.bytes;\n\n\
         pub const NONE: u64 = 1000000000u64;\n\n\
         pub const T_STRING: u64 = 1u64;\npub const T_BOOL: u64 = 2u64;\n\
         pub const T_OPTION_STRING: u64 = 3u64;\npub const T_OPTION_BOOL: u64 = 4u64;\n\
         pub const T_OPTION_U64: u64 = 5u64;\npub const T_STRINGS: u64 = 6u64;\n\
         pub const T_LIST: u64 = 7u64;\npub const T_OPTION_STRUCT: u64 = 8u64;\n\
         pub const T_INSTRUCTIONS: u64 = 9u64;\n\n\
         // A missing field: an error, `None`, `Default::default()`, or\n\
         // `default_provider()` (`simulated`).\n\
         pub const REQUIRED: u64 = 0u64;\npub const MISSING_NONE: u64 = 1u64;\n\
         pub const DEFAULT: u64 = 2u64;\npub const DEFAULT_PROVIDER: u64 = 3u64;\n\n",
    );
    for (index, shape) in shapes.iter().enumerate() {
        let _ = writeln!(
            out,
            "pub const {}: u64 = {index}u64;",
            constant(&shape.name)
        );
        for (field_index, field) in shape.fields.iter().enumerate() {
            let _ = writeln!(
                out,
                "pub const {}__{}: u64 = {field_index}u64;",
                constant(&shape.name),
                field.name.to_ascii_uppercase()
            );
        }
    }
    out.push('\n');
    for (index, (variant, other)) in variants.iter().enumerate() {
        let _ = writeln!(
            out,
            "pub const OP_{}: u64 = {index}u64;",
            upper_snake(variant)
        );
        if *other {
            let _ = writeln!(out, "pub const OP_OTHER: u64 = {index}u64;");
        }
    }

    out.push_str("\npub fn field_count(s: u64) -> u64 {\n");
    for shape in &shapes {
        let _ = writeln!(
            out,
            "    if s == {} {{ return {}u64; }}",
            constant(&shape.name),
            shape.fields.len()
        );
    }
    out.push_str("    0u64\n}\n");

    out.push_str("\npub fn field_type(s: u64, i: u64) -> u64 {\n");
    for shape in &shapes {
        let _ = writeln!(out, "    if s == {} {{", constant(&shape.name));
        for (index, field) in shape.fields.iter().enumerate() {
            let _ = writeln!(
                out,
                "        if i == {index}u64 {{ return {}; }}",
                type_code(&field.ty)
            );
        }
        out.push_str("    }\n");
    }
    out.push_str("    0u64\n}\n");

    out.push_str("\npub fn field_default(s: u64, i: u64) -> u64 {\n");
    for shape in &shapes {
        let _ = writeln!(out, "    if s == {} {{", constant(&shape.name));
        for (index, field) in shape.fields.iter().enumerate() {
            let default = match field.default.as_deref() {
                Some("default_provider") => "DEFAULT_PROVIDER",
                Some(_) => "DEFAULT",
                None if field.ty.starts_with("Option<") => "MISSING_NONE",
                None => "REQUIRED",
            };
            let _ = writeln!(out, "        if i == {index}u64 {{ return {default}; }}");
        }
        out.push_str("    }\n");
    }
    out.push_str("    REQUIRED\n}\n");

    out.push_str(
        "\n// The field of `s` serde reads as `key`, or `NONE`.\n\
         pub fn field_index(s: u64, key: Slice<u8>) -> u64 {\n",
    );
    for shape in &shapes {
        let _ = writeln!(out, "    if s == {} {{", constant(&shape.name));
        for (index, field) in shape.fields.iter().enumerate() {
            let _ = writeln!(
                out,
                "        if bytes.equal(key, \"{}\".as_slice()) {{ return {index}u64; }}",
                field.key
            );
        }
        out.push_str("        return NONE;\n    }\n");
    }
    out.push_str("    NONE\n}\n");

    out.push_str("\npub fn append_field_key(out: Buffer<u8>, s: u64, i: u64) -> Buffer<u8> {\n");
    for shape in &shapes {
        let _ = writeln!(out, "    if s == {} {{", constant(&shape.name));
        for (index, field) in shape.fields.iter().enumerate() {
            let _ = writeln!(
                out,
                "        if i == {index}u64 {{ return bytes.append(out, \"{}\".as_slice()); }}",
                field.key
            );
        }
        out.push_str("    }\n");
    }
    out.push_str("    out\n}\n");

    out.push_str(
        "\n// What serde expects of a struct: `struct <Name>`.\n\
         pub fn append_struct_name(out: Buffer<u8>, s: u64) -> Buffer<u8> {\n",
    );
    for shape in &shapes {
        if !shape.name.starts_with("Instruction::") {
            let _ = writeln!(
                out,
                "    if s == {} {{ return bytes.append(out, \"struct {}\".as_slice()); }}",
                constant(&shape.name),
                shape.name
            );
        }
    }
    out.push_str("    out\n}\n");

    out.push_str(
        "\n// The variant `op` names, or `OP_OTHER`.\n\
         pub fn variant(name: Slice<u8>) -> u64 {\n",
    );
    for (variant, other) in &variants {
        if !other {
            let _ = writeln!(
                out,
                "    if bytes.equal(name, \"{variant}\".as_slice()) {{ return OP_{}; }}",
                upper_snake(variant)
            );
        }
    }
    out.push_str("    OP_OTHER\n}\n");

    out.push_str(
        "\n// The struct of a variant's fields, or `NONE` for a unit variant.\n\
         pub fn variant_struct(v: u64) -> u64 {\n",
    );
    for (variant, other) in &variants {
        let name = format!("Instruction::{variant}");
        if !other && shapes.iter().any(|shape| shape.name == name) {
            let _ = writeln!(
                out,
                "    if v == OP_{} {{ return {}; }}",
                upper_snake(variant),
                constant(&name)
            );
        }
    }
    out.push_str("    NONE\n}\n");
    out
}

#[test]
fn the_bytecode_schema_is_what_the_bytecode_types_say() {
    let expected = module();
    let path = root().join("compiler/agent_bytecode_schema.argx");
    let current = fs::read_to_string(&path).unwrap_or_default();
    if current != expected {
        if env::var_os("ARGORIX_BLESS").is_some() {
            fs::write(&path, expected).unwrap();
            return;
        }
        panic!("compiler/agent_bytecode_schema.argx is stale; run with ARGORIX_BLESS=1");
    }
}
