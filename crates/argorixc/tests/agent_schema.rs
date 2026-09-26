//! ESP-018.B: `compiler/agent_schema.argx` is the shape of the stage0 agent
//! syntax tree, generated here from `crates/argorix_parser/src/ast.rs` and
//! `span.rs`, so the Argorix parser writes every struct's fields in the
//! order serde writes them without anyone copying them by hand.
//!
//! - Every struct is numbered in declaration order, with a constant for
//!   itself and one per field, and its field names.
//! - Every enum is numbered too, with a constant per variant and the
//!   variant's shape: unit, one value, or fields. A variant with fields is
//!   also a struct of the schema, named `<Enum>__<Variant>`.
//!
//! The test fails when the file is not what the sources give;
//! `ARGORIX_BLESS=1` rewrites it.

use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::{env, fs};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

struct Struct {
    name: String,
    fields: Vec<String>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Shape {
    Unit,
    Value,
    Fields,
}

struct Variant {
    name: String,
    shape: Shape,
    /// The struct that holds a variant's fields.
    fields_struct: Option<usize>,
}

struct Enum {
    name: String,
    variants: Vec<Variant>,
}

/// `ATrustBoundaryDecl` as `A_TRUST_BOUNDARY_DECL`.
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

fn field_of(line: &str) -> Option<String> {
    let rest = line.trim().strip_prefix("pub ")?;
    let (name, _) = rest.split_once(':')?;
    Some(name.trim().to_string())
}

/// The structs and enums of one source file, as rustfmt writes them.
fn read(file: &str, structs: &mut Vec<Struct>, enums: &mut Vec<Enum>) {
    let text = fs::read_to_string(root().join(file)).unwrap();
    let lines: Vec<&str> = text.lines().collect();
    let mut index = 0;
    while index < lines.len() {
        let line = lines[index];
        let header = line
            .strip_prefix("pub struct ")
            .map(|rest| (true, rest))
            .or_else(|| line.strip_prefix("pub enum ").map(|rest| (false, rest)));
        let Some((is_struct, rest)) = header else {
            index += 1;
            continue;
        };
        let name = rest
            .trim_end_matches(" {")
            .split('<')
            .next()
            .unwrap()
            .to_string();
        assert!(
            rest.ends_with(" {"),
            "{file}: unexpected declaration `{line}`"
        );
        index += 1;
        let mut body = Vec::new();
        while lines[index] != "}" {
            body.push(lines[index]);
            index += 1;
        }
        if is_struct {
            let fields = body
                .iter()
                .map(|line| {
                    field_of(line).unwrap_or_else(|| panic!("{file}: unexpected field `{line}`"))
                })
                .collect();
            structs.push(Struct { name, fields });
        } else {
            let mut variants = Vec::new();
            let mut at = 0;
            while at < body.len() {
                let line = body[at].trim();
                at += 1;
                if let Some(variant) = line.strip_suffix(" {") {
                    // A variant with fields, one per line, closed by `},`.
                    let mut fields = Vec::new();
                    while body[at].trim() != "}," {
                        let field = body[at].trim();
                        let (name, _) = field
                            .split_once(':')
                            .unwrap_or_else(|| panic!("{file}: unexpected field `{field}`"));
                        fields.push(name.trim().to_string());
                        at += 1;
                    }
                    at += 1;
                    structs.push(Struct {
                        name: format!("{name}__{variant}"),
                        fields,
                    });
                    variants.push(Variant {
                        name: variant.into(),
                        shape: Shape::Fields,
                        fields_struct: Some(structs.len() - 1),
                    });
                    continue;
                }
                if let Some(variant) = line
                    .strip_suffix(',')
                    .filter(|v| v.chars().all(|c| c.is_ascii_alphanumeric()))
                {
                    variants.push(Variant {
                        name: variant.into(),
                        shape: Shape::Unit,
                        fields_struct: None,
                    });
                } else if let Some((variant, _)) = line.split_once('(') {
                    assert!(line.ends_with("),"), "{file}: unexpected variant `{line}`");
                    variants.push(Variant {
                        name: variant.into(),
                        shape: Shape::Value,
                        fields_struct: None,
                    });
                } else if let Some((variant, rest)) = line.split_once(" { ") {
                    let inner = rest
                        .strip_suffix(" },")
                        .unwrap_or_else(|| panic!("{file}: unexpected variant `{line}`"));
                    let fields = inner
                        .split(", ")
                        .map(|field| field.split(':').next().unwrap().trim().to_string())
                        .collect();
                    structs.push(Struct {
                        name: format!("{name}__{variant}"),
                        fields,
                    });
                    variants.push(Variant {
                        name: variant.into(),
                        shape: Shape::Fields,
                        fields_struct: Some(structs.len() - 1),
                    });
                } else {
                    panic!("{file}: unexpected variant `{line}`");
                }
            }
            enums.push(Enum { name, variants });
        }
        index += 1;
    }
}

fn module() -> String {
    let mut structs = Vec::new();
    let mut enums = Vec::new();
    read(
        "crates/argorix_parser/src/span.rs",
        &mut structs,
        &mut enums,
    );
    read("crates/argorix_parser/src/ast.rs", &mut structs, &mut enums);
    let mut out = String::new();
    out.push_str(
        "core 0.1;
module compiler.agent_schema;

// The shape of the agent-language syntax tree (ESP-018.B). GENERATED by
// crates/argorixc/tests/agent_schema.rs from crates/argorix_parser/src/ast.rs
// and span.rs; regenerate with ARGORIX_BLESS=1, do not edit.
//
// Structs and enums are numbered in declaration order, span.rs first. A
// field's constant is its index in serde's order. An enum variant with
// fields holds them in the struct `<ENUM>__<VARIANT>`.

import stdlib.bytes;

pub const UNIT: u64 = 0u64;
pub const VALUE: u64 = 1u64;
pub const FIELDS: u64 = 2u64;

// ------------------------------------------------------------------ structs

",
    );
    for (number, item) in structs.iter().enumerate() {
        let upper = upper_snake(&item.name);
        let _ = writeln!(out, "pub const {upper}: u64 = {number}u64;");
        for (field, name) in item.fields.iter().enumerate() {
            let _ = writeln!(
                out,
                "pub const {upper}_{}: u64 = {field}u64;",
                name.to_ascii_uppercase()
            );
        }
    }
    let _ = writeln!(out, "pub const STRUCTS: u64 = {}u64;", structs.len());
    out.push_str(
        "\n// ------------------------------------------------------------------ enums\n\n",
    );
    for (number, item) in enums.iter().enumerate() {
        let upper = upper_snake(&item.name);
        let _ = writeln!(out, "pub const {upper}: u64 = {number}u64;");
        for (variant, found) in item.variants.iter().enumerate() {
            let _ = writeln!(
                out,
                "pub const {upper}_{}: u64 = {variant}u64;",
                upper_snake(&found.name)
            );
        }
    }
    let _ = writeln!(out, "pub const ENUMS: u64 = {}u64;", enums.len());

    out.push_str(
        "\n// ------------------------------------------------------------------ names\n\n",
    );
    out.push_str("// The number of fields of struct `s`.\npub fn field_count(s: u64) -> u64 {\n");
    for (number, item) in structs.iter().enumerate() {
        let _ = writeln!(
            out,
            "    if s == {number}u64 {{ return {}u64; }}",
            item.fields.len()
        );
    }
    out.push_str("    0u64\n}\n\n");
    out.push_str("// `\"<name>\"`: the name of field `index` of struct `s`, quoted.\npub fn append_field(out: Buffer<u8>, s: u64, index: u64) -> Buffer<u8> {\n");
    for (number, item) in structs.iter().enumerate() {
        let _ = writeln!(out, "    if s == {number}u64 {{");
        for (field, name) in item.fields.iter().enumerate() {
            let _ = writeln!(out, "        if index == {field}u64 {{ return bytes.append(out, \"\\\"{name}\\\"\".as_slice()); }}");
        }
        out.push_str("    }\n");
    }
    out.push_str("    out\n}\n\n");
    out.push_str("// The shape of variant `v` of enum `e`: UNIT, VALUE or FIELDS.\npub fn variant_shape(e: u64, v: u64) -> u64 {\n");
    for (number, item) in enums.iter().enumerate() {
        let valued: Vec<String> = item
            .variants
            .iter()
            .enumerate()
            .filter(|(_, variant)| variant.shape != Shape::Unit)
            .map(|(index, variant)| {
                let shape = if variant.shape == Shape::Value {
                    "VALUE"
                } else {
                    "FIELDS"
                };
                format!("        if v == {index}u64 {{ return {shape}; }}\n")
            })
            .collect();
        if !valued.is_empty() {
            let _ = writeln!(out, "    if e == {number}u64 {{");
            for line in valued {
                out.push_str(&line);
            }
            out.push_str("    }\n");
        }
    }
    out.push_str("    UNIT\n}\n\n");
    out.push_str("// The struct that holds the fields of variant `v` of enum `e`, if it has\n// fields.\npub fn variant_struct(e: u64, v: u64) -> u64 {\n");
    for (number, item) in enums.iter().enumerate() {
        for (index, variant) in item.variants.iter().enumerate() {
            if let Some(holder) = variant.fields_struct {
                let _ = writeln!(
                    out,
                    "    if e == {number}u64 && v == {index}u64 {{ return {holder}u64; }}"
                );
            }
        }
    }
    out.push_str("    STRUCTS\n}\n\n");
    out.push_str("// `\"<Variant>\"`: the name of variant `v` of enum `e`, quoted.\npub fn append_variant(out: Buffer<u8>, e: u64, v: u64) -> Buffer<u8> {\n");
    for (number, item) in enums.iter().enumerate() {
        let _ = writeln!(out, "    if e == {number}u64 {{");
        for (index, variant) in item.variants.iter().enumerate() {
            let _ = writeln!(out, "        if v == {index}u64 {{ return bytes.append(out, \"\\\"{}\\\"\".as_slice()); }}", variant.name);
        }
        out.push_str("    }\n");
    }
    out.push_str("    out\n}\n");
    out
}

#[test]
fn the_agent_schema_is_what_the_syntax_tree_says() {
    let path = root().join("compiler/agent_schema.argx");
    let expected = module();
    if env::var_os("ARGORIX_BLESS").is_some() {
        fs::write(&path, &expected).unwrap();
    }
    let found = fs::read_to_string(&path).unwrap_or_default();
    assert!(
        found == expected,
        "compiler/agent_schema.argx is not what ast.rs and span.rs give; regenerate it with ARGORIX_BLESS=1"
    );
}

#[test]
fn constant_names_are_unique() {
    let text = module();
    let mut names: Vec<&str> = text
        .lines()
        .filter_map(|line| line.strip_prefix("pub const "))
        .map(|rest| rest.split(':').next().unwrap())
        .collect();
    let total = names.len();
    names.sort_unstable();
    names.dedup();
    assert_eq!(names.len(), total, "two schema constants share a name");
}

/// The enums of the syntax tree, for the word tables.
fn schema_enums() -> Vec<Enum> {
    let mut structs = Vec::new();
    let mut enums = Vec::new();
    read(
        "crates/argorix_parser/src/span.rs",
        &mut structs,
        &mut enums,
    );
    read("crates/argorix_parser/src/ast.rs", &mut structs, &mut enums);
    enums
}

/// `lower_snake` of a function or enum name, for Argorix identifiers.
fn lower_snake(name: &str) -> String {
    upper_snake(name).to_ascii_lowercase()
}

/// Every `"word" => Enum::Variant` arm of `parser.rs`, grouped by the
/// function it is in and the enum it names, in source order.
fn word_tables() -> Vec<(String, String, Vec<(String, String)>)> {
    let text = fs::read_to_string(root().join("crates/argorix_parser/src/parser.rs")).unwrap();
    // Only the parser itself, not its tests.
    let text = text.split("#[cfg(test)]").next().unwrap();
    let bytes = text.as_bytes();
    let mut tables: Vec<(String, String, Vec<(String, String)>)> = Vec::new();
    let mut function = String::new();
    let mut at = 0;
    while at < bytes.len() {
        if text[at..].starts_with("fn ") {
            let rest = &text[at + 3..];
            let name: String = rest
                .chars()
                .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
                .collect();
            if !name.is_empty() {
                function = name;
            }
        }
        if bytes[at] == b'"' {
            let end = at + 1 + text[at + 1..].find('"').unwrap();
            let word = &text[at + 1..end];
            let after = text[end + 1..].trim_start();
            if let Some(arm) = after.strip_prefix("=>") {
                let arm = arm.trim_start();
                let arm = arm.strip_prefix('{').map_or(arm, str::trim_start);
                let path: String = arm
                    .chars()
                    .take_while(|c| c.is_ascii_alphanumeric() || *c == '_' || *c == ':')
                    .collect();
                if let Some((enum_name, variant)) = path.split_once("::") {
                    let plain = !word.is_empty()
                        && word
                            .chars()
                            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '.' || c == '-');
                    if plain && !variant.contains("::") {
                        match tables
                            .iter_mut()
                            .find(|(f, e, _)| *f == function && e == enum_name)
                        {
                            Some((_, _, words)) => {
                                // Two `match`es of one function may map the same
                                // words; they must agree.
                                match words.iter().find(|(known, _)| known == word) {
                                    Some((_, known)) => assert_eq!(
                                        known, variant,
                                        "{function}: `{word}` names two `{enum_name}` variants"
                                    ),
                                    None => words.push((word.into(), variant.into())),
                                }
                            }
                            None => tables.push((
                                function.clone(),
                                enum_name.into(),
                                vec![(word.into(), variant.into())],
                            )),
                        }
                    }
                }
            }
            at = end + 1;
            continue;
        }
        at += 1;
    }
    tables
}

fn words_module() -> String {
    let enums = schema_enums();
    let mut out = String::new();
    out.push_str(
        "core 0.1;
module compiler.agent_words;

// The words the agent-language parser maps to enum variants (ESP-018.B).
// GENERATED by crates/argorixc/tests/agent_schema.rs from every
// `\"word\" => Enum::Variant` arm of crates/argorix_parser/src/parser.rs;
// regenerate with ARGORIX_BLESS=1, do not edit.
//
// `words_<function>__<enum>(word)` is the variant of the enum
// (`compiler.agent_schema`) that `word` names in that stage0 function, or
// NO_VARIANT. What stage0 does with any other word (an error, or an
// `Unknown` variant) stays in compiler/agent_parser.argx.

import stdlib.bytes;

pub const NO_VARIANT: u64 = 1000000u64;
",
    );
    for (function, enum_name, words) in word_tables() {
        let Some(found) = enums.iter().find(|item| item.name == enum_name) else {
            continue;
        };
        let _ = write!(
            out,
            "\n// `{function}`: the `{enum_name}` a word names.\npub fn words_{function}__{}(word: Slice<u8>) -> u64 {{\n",
            lower_snake(&enum_name)
        );
        for (word, variant) in words {
            let index = found
                .variants
                .iter()
                .position(|candidate| candidate.name == variant)
                .unwrap_or_else(|| panic!("{function}: `{enum_name}::{variant}` is not in ast.rs"));
            let _ = writeln!(
                out,
                "    if bytes.equal(word, \"{word}\".as_slice()) {{ return {index}u64; }}"
            );
        }
        out.push_str("    NO_VARIANT\n}\n");
    }
    out
}

#[test]
fn the_agent_words_are_what_the_parser_says() {
    let path = root().join("compiler/agent_words.argx");
    let expected = words_module();
    if env::var_os("ARGORIX_BLESS").is_some() {
        fs::write(&path, &expected).unwrap();
    }
    let found = fs::read_to_string(&path).unwrap_or_default();
    assert!(
        found == expected,
        "compiler/agent_words.argx is not what parser.rs gives; regenerate it with ARGORIX_BLESS=1"
    );
}
