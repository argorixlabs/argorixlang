//! ESP-018.B: the keyed-block declarations of the Argorix agent parser,
//! generated from `crates/argorix_parser/src/parser.rs`.
//!
//! Most stage0 declarations are one shape: `<keyword> <name> { key value ...
//! }`, each key set at most once through `set_<block>_field`, with one of a
//! few kinds of value, and defaults for the keys left out. This test reads
//! each such `parse_*` function and writes its Argorix port, calling the
//! helpers of `compiler/agent_parser.argx` (`field_enum`, `field_string`,
//! `default_unknown`, ...), between the markers below in that file.
//!
//! A function it cannot read exactly (an unknown kind of value, another
//! message, a default it does not know) is left out, with the reason, for a
//! hand port. The differential checks what is generated like everything
//! else. The test fails when the file's region is not what this gives;
//! `ARGORIX_BLESS=1` rewrites it.

use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::{env, fs};

const BEGIN: &str = "// ------------------------------------------------------------------ generated keyed blocks\n";
const END: &str = "// ------------------------------------------------------------------ end of generated keyed blocks\n";

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

/// The text without whitespace outside string literals, so rustfmt's layout
/// does not matter.
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

/// The top-level pieces of `text` split at commas outside any bracket.
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
            pieces.push(text[start..at].trim().to_string());
            start = at + 1;
        }
        at += 1;
    }
    let last = text[start..].trim();
    if !last.is_empty() {
        pieces.push(last.to_string());
    }
    pieces
}

/// The arms of a `match` body: `pattern => block` needs no comma after it,
/// `pattern => expression,` does.
fn split_arms(text: &str) -> Vec<String> {
    let mut arms = Vec::new();
    let bytes = text.as_bytes();
    let mut at = 0;
    while at < bytes.len() {
        while at < bytes.len() && bytes[at] == b',' {
            at += 1;
        }
        if at >= bytes.len() {
            break;
        }
        let arrow = at + text[at..].find("=>").expect("an arm without `=>`");
        let mut end = arrow + 2;
        if bytes.get(end) == Some(&b'{') {
            end = balanced(text, end).1;
        } else {
            let mut depth = 0;
            let mut in_string = false;
            while end < bytes.len() {
                let byte = bytes[end];
                if in_string {
                    if byte == b'\\' {
                        end += 2;
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
                    break;
                }
                end += 1;
            }
        }
        arms.push(text[at..end].to_string());
        at = end;
    }
    arms
}

fn quoted(text: &str) -> Option<String> {
    let text = text.trim();
    text.strip_prefix('"')?
        .strip_suffix('"')
        .map(str::to_string)
}

/// The structs and enums of ast.rs: struct fields with their types, and enum
/// variants.
struct Ast {
    structs: Vec<(String, Vec<(String, String)>)>,
    enums: Vec<(String, Vec<String>)>,
}

fn read_ast() -> Ast {
    let text = fs::read_to_string(root().join("crates/argorix_parser/src/ast.rs")).unwrap();
    let lines: Vec<&str> = text.lines().collect();
    let mut structs = Vec::new();
    let mut enums = Vec::new();
    let mut index = 0;
    while index < lines.len() {
        let line = lines[index];
        if let Some(rest) = line.strip_prefix("source_enum!(") {
            let name = rest.trim_end_matches(" {").to_string();
            let mut variants = Vec::new();
            index += 1;
            while lines[index] != "});" {
                variants.push(
                    lines[index]
                        .trim()
                        .split(" => ")
                        .next()
                        .unwrap()
                        .to_string(),
                );
                index += 1;
            }
            variants.push("Unknown".to_string());
            enums.push((name, variants));
            index += 1;
            continue;
        }
        if let Some(rest) = line.strip_prefix("pub struct ") {
            let name = rest.trim_end_matches(" {").to_string();
            let mut fields = Vec::new();
            index += 1;
            while lines[index] != "}" {
                let field = lines[index].trim().trim_start_matches("pub ");
                let (field_name, field_type) = field.split_once(':').unwrap();
                fields.push((
                    field_name.trim().to_string(),
                    field_type.trim().trim_end_matches(',').to_string(),
                ));
                index += 1;
            }
            structs.push((name, fields));
        } else if let Some(rest) = line.strip_prefix("pub enum ") {
            let name = rest.trim_end_matches(" {").to_string();
            let mut variants = Vec::new();
            index += 1;
            while lines[index] != "}" {
                let variant = lines[index].trim();
                if !variant.starts_with(|c: char| c.is_ascii_uppercase()) {
                    index += 1;
                    continue;
                }
                let variant: String = variant
                    .chars()
                    .take_while(|c| c.is_ascii_alphanumeric())
                    .collect();
                variants.push(variant);
                index += 1;
            }
            enums.push((name, variants));
        }
        index += 1;
    }
    Ast { structs, enums }
}

/// The body of every `fn name(...) -> Result<Type, Diagnostic> { ... }` in
/// the parser, by name.
fn functions(text: &str) -> Vec<(String, String, String)> {
    let mut found = Vec::new();
    let mut at = 0;
    while let Some(offset) = text[at..].find("fn parse_") {
        let start = at + offset + 3;
        let name: String = text[start..]
            .chars()
            .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
            .collect();
        let brace = start + text[start..].find('{').unwrap();
        let signature = &text[start..brace];
        let (body, end) = balanced(text, brace);
        if let Some(result) = signature.split("-> Result<").nth(1) {
            let returned: String = result
                .chars()
                .take_while(|c| c.is_ascii_alphanumeric() || *c == '_' || *c == '<' || *c == '>')
                .collect();
            let returned = returned.trim_end_matches(',').to_string();
            let flat = squeeze(signature);
            if flat.contains("(&mutself)") || flat.contains("(&mutself,)") {
                found.push((name, returned, body));
            }
        }
        at = end;
    }
    found
}

/// How a key's value is read.
enum Kind {
    Ident(String),
    Str(String),
    Int(String),
    Bool(String),
    StrArray(String),
    IdentBlock(String),
    /// A list of entries, read by a generated list function.
    List(String),
    /// An identifier mapped to a variant; others become `fallback`.
    Enum {
        description: String,
        enum_name: String,
        fallback: String,
        /// The function whose word table maps it.
        table_function: Option<String>,
    },
}

struct Field {
    key: String,
    variable: String,
    block: String,
    kind: Kind,
}

/// The kind of value a `set_*_field` closure reads, from its squeezed body.
fn closure_kind(body: &str, parser_text: &str) -> Result<Kind, String> {
    let body = body.trim();
    let inner = body
        .strip_prefix('{')
        .and_then(|rest| rest.strip_suffix('}'))
        .map(str::trim)
        .unwrap_or(body);
    let simple = [
        ("parser.expect_identifier(", 0),
        ("parser.expect_string(", 1),
        ("parser.expect_integer(", 2),
        ("parser.expect_bool(", 3),
        ("parser.parse_string_array(", 4),
        ("parser.parse_identifier_block(", 5),
    ];
    for (prefix, which) in simple {
        if let Some(rest) = inner.strip_prefix(prefix) {
            let argument = rest.strip_suffix(')').ok_or("not a single call")?;
            let description =
                quoted(argument.trim_end_matches(',')).ok_or("not a literal description")?;
            return Ok(match which {
                0 => Kind::Ident(description),
                1 => Kind::Str(description),
                2 => Kind::Int(description),
                3 => Kind::Bool(description),
                4 => Kind::StrArray(description),
                _ => Kind::IdentBlock(description),
            });
        }
    }
    // A list of entries: `parser.parse_<list>()`.
    if let Some(function) = inner
        .strip_prefix("parser.")
        .and_then(|rest| rest.strip_suffix("()"))
    {
        if function.starts_with("parse_")
            && function
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '_')
        {
            return Ok(Kind::List(function.to_string()));
        }
    }
    // A word mapped by a function of its own: `parse_mapped_identifier(d,
    // f)`, or `let token = expect_identifier(d)?; Ok(Spanned::new(f(&token.value),
    // token.span))`.
    let mapped = if let Some(rest) = inner.strip_prefix("parser.parse_mapped_identifier(") {
        let arguments = split_top(rest.strip_suffix(')').ok_or("not a single call")?);
        Some((arguments[0].clone(), arguments[1].clone()))
    } else if let Some(rest) = inner.strip_prefix("lettoken=parser.expect_identifier(") {
        let (description, rest) = rest.split_once(")?;").ok_or("no description")?;
        rest.strip_prefix("Ok(Spanned::new(")
            .and_then(|value| value.split_once("(&token.value),token.span"))
            .map(|(function, _)| (description.to_string(), function.to_string()))
    } else {
        None
    };
    if let Some((description, function)) = mapped {
        let description = quoted(&description).ok_or("not a literal description")?;
        let squeezed = squeeze(parser_text);
        let header = format!("fn{function}(value:&str)->");
        let at = squeezed
            .find(&header)
            .ok_or_else(|| format!("no function `{function}`"))?;
        let rest = &squeezed[at + header.len()..];
        let enum_name: String = rest
            .chars()
            .take_while(|c| c.is_ascii_alphanumeric())
            .collect();
        let (body, _) = balanced(rest, rest.find('{').unwrap());
        let fallback = body
            .split("other=>")
            .nth(1)
            .and_then(|value| value.split_once("(other.to_owned())"))
            .and_then(|(value, _)| value.split_once("::"))
            .map(|(_, variant)| variant.to_string())
            .ok_or_else(|| format!("`{function}` has no fallback"))?;
        return Ok(Kind::Enum {
            description,
            enum_name,
            fallback,
            table_function: Some(function),
        });
    }
    // let token = parser.expect_identifier("d")?; let value = match
    // token.value.as_str() { arms, other => E::V(other.to_owned()), };
    // Ok(Spanned::new(value, token.span))
    let rest = inner
        .strip_prefix("lettoken=parser.expect_identifier(")
        .ok_or_else(|| format!("unknown value `{inner}`"))?;
    let (description, rest) = rest.split_once(")?;").ok_or("no description")?;
    let description = quoted(description).ok_or("not a literal description")?;
    let rest = rest
        .trim()
        .strip_prefix("letvalue=matchtoken.value.as_str(){")
        .ok_or("no match")?;
    let (arms, rest) = rest.rsplit_once("};").ok_or("no end of match")?;
    if rest.trim() != "Ok(Spanned::new(value,token.span))" {
        return Err(format!("unknown result `{rest}`"));
    }
    let arms = split_top(arms);
    let last = arms.last().ok_or("no arms")?;
    let fallback = last
        .strip_prefix("other=>")
        .and_then(|value| value.strip_suffix("(other.to_owned())"))
        .ok_or_else(|| format!("unknown fallback `{last}`"))?;
    let (enum_name, variant) = fallback.split_once("::").ok_or("no enum")?;
    for arm in &arms[..arms.len() - 1] {
        let (_, target) = arm.split_once("=>").ok_or("no arm")?;
        if !target.starts_with(&format!("{enum_name}::")) || target.contains('(') {
            return Err(format!("unknown arm `{arm}`"));
        }
    }
    Ok(Kind::Enum {
        description,
        enum_name: enum_name.to_string(),
        fallback: variant.to_string(),
        table_function: None,
    })
}

/// A default for a field left out.
enum Default {
    None,
    Array,
    Unknown(String, String),
    Variant(String, String),
    EmptyString,
}

/// The port of one function, or why it has none.
fn port(
    name: &str,
    returned: &str,
    body: &str,
    ast: &Ast,
    parser_text: &str,
) -> Result<String, String> {
    let body = squeeze(body);
    // The header: `<keyword> <name> {`, or an anonymous `{` whose span is the
    // defaults' span.
    let anonymous_tail = "=self.peek().span;self.expect_symbol(TokenKind::LeftBrace,\"`{`\")?;";
    let anonymous = body
        .strip_prefix("let")
        .and_then(|rest| rest.split_once(anonymous_tail))
        .filter(|(variable, _)| {
            variable
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '_')
        });
    let mut open_variable = String::new();
    let (header, rest) = if let Some((variable, rest)) = anonymous {
        open_variable = variable.to_string();
        (None, rest)
    } else {
        let rest = body
            .strip_prefix("self.expect_keyword(")
            .ok_or("no keyword")?;
        let (keyword, rest) = rest.split_once(")?;").ok_or("no keyword end")?;
        let keyword = quoted(keyword).ok_or("keyword not literal")?;
        let rest = rest
            .strip_prefix("letname=self.expect_identifier(")
            .ok_or("no name")?;
        let (name_description, rest) = rest.split_once(")?;").ok_or("no name end")?;
        let name_description = quoted(name_description).ok_or("name description not literal")?;
        let rest = rest
            .strip_prefix("self.expect_symbol(TokenKind::LeftBrace,\"`{`\")?;")
            .ok_or("no `{`")?;
        (Some((keyword, name_description)), rest)
    };
    // The slots.
    let loop_start = rest
        .find("while!self.check(&TokenKind::RightBrace){")
        .ok_or("no loop")?;
    let slots_text = &rest[..loop_start];
    for statement in slots_text
        .split(';')
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        if !(statement.starts_with("letmut") && statement.ends_with("=None")) {
            return Err(format!("unknown statement `{statement}`"));
        }
    }
    let rest = &rest[loop_start + "while!self.check(&TokenKind::RightBrace)".len()..];
    let (loop_body, loop_end) = balanced(rest, 0);
    let after = &rest[loop_end..];
    let loop_body = loop_body.trim();
    let loop_body = loop_body
        .strip_prefix("self.ensure_not_eof(")
        .ok_or("no end-of-file check")?;
    let (unterminated, loop_body) = loop_body.split_once(")?;").ok_or("no eof end")?;
    let unterminated = quoted(unterminated).ok_or("eof message not literal")?;
    let loop_body = loop_body
        .trim()
        .strip_prefix("matchself.peek_identifier()")
        .ok_or("no match on the key")?;
    let (arms_text, _) = balanced(loop_body, 0);
    let arms = split_arms(&arms_text);
    let mut fields = Vec::new();
    let mut block_name: Option<String> = None;
    let mut article = String::from("a");
    for arm in &arms {
        if let Some(rest) = arm.strip_prefix("Some(\"") {
            let (key, rest) = rest.split_once("\")=>").ok_or("no key")?;
            let rest = rest.trim();
            let rest = rest
                .strip_prefix('{')
                .and_then(|value| value.strip_suffix('}'))
                .map(str::trim)
                .unwrap_or(rest);
            let rest = rest
                .strip_prefix("self.")
                .ok_or_else(|| format!("key `{key}`: not a setter"))?;
            let open = rest.find('(').ok_or("no call")?;
            let setter = &rest[..open];
            if !(setter.starts_with("set_") && setter.ends_with("_field")) {
                return Err(format!("key `{key}`: calls `{setter}`"));
            }
            let (arguments, tail_at) = balanced(rest, open);
            let tail = rest[tail_at..]
                .trim()
                .trim_end_matches(';')
                .trim_end_matches(',');
            if tail != "?" {
                return Err(format!("key `{key}`: `{tail}` after the setter"));
            }
            let arguments = split_top(&arguments);
            let variable = arguments[0]
                .strip_prefix("&mut")
                .ok_or("no slot")?
                .to_string();
            let (block, stated_key) = if setter == "set_block_field" {
                (
                    quoted(&arguments[1]).ok_or("block not literal")?,
                    quoted(&arguments[2]).ok_or("key not literal")?,
                )
            } else {
                let block = setter
                    .strip_prefix("set_")
                    .and_then(|value| value.strip_suffix("_field"))
                    .unwrap()
                    .to_string();
                (block, quoted(&arguments[1]).ok_or("key not literal")?)
            };
            if stated_key != key {
                return Err(format!("key `{key}` reported as `{stated_key}`"));
            }
            let closure = arguments.last().unwrap();
            let closure = closure
                .strip_prefix('|')
                .and_then(|rest| rest.split_once('|'))
                .ok_or_else(|| format!("key `{key}`: not a closure"))?;
            let (parameter, closure) = closure;
            let closure = closure.replace(&format!("{parameter}."), "parser.");
            let closure = closure.as_str();
            let kind =
                closure_kind(closure, parser_text).map_err(|why| format!("key `{key}`: {why}"))?;
            fields.push(Field {
                key: key.to_string(),
                variable,
                block,
                kind,
            });
        } else if arm.starts_with("Some(other)=>") {
            let message = arm
                .split("format!(\"")
                .nth(1)
                .and_then(|value| value.split_once("\"),"))
                .map(|(message, _)| message)
                .ok_or("no message for another key")?;
            let block = message
                .strip_prefix("unexpected ")
                .and_then(|value| value.strip_suffix(" item `{other}`"))
                .ok_or_else(|| format!("unknown message `{message}`"))?;
            block_name = Some(block.to_string());
        } else if arm.starts_with("None=>") {
            let block = block_name.clone().ok_or("`None` before `Some(other)`")?;
            if arm.contains(&format!("\"expected an {block} field\"")) {
                article = String::from("an");
            } else if arm.contains(&format!("\"expected {block} field\"")) {
                article = String::new();
            } else if !arm.contains(&format!("\"expected a {block} field\"")) {
                return Err(format!("unknown message for no key `{arm}`"));
            }
        } else {
            return Err(format!("unknown arm `{arm}`"));
        }
    }
    let block_name = block_name.ok_or("no message for another key")?;
    // After the loop: the defaults.
    let after = after.trim();
    let after = after.strip_prefix("self.advance();").ok_or("no advance")?;
    let (lets, literal) = after
        .split_once(&format!("Ok({returned}{{"))
        .ok_or("no literal")?;
    let mut closures: Vec<(String, String)> = Vec::new();
    // The variable that holds the defaults' span.
    let mut span_variable = if header.is_some() {
        String::new()
    } else {
        open_variable.clone()
    };
    for statement in lets.split(';').map(str::trim).filter(|s| !s.is_empty()) {
        if let Some(variable) = statement
            .strip_prefix("let")
            .and_then(|rest| rest.strip_suffix("=name.span"))
        {
            if header.is_none() {
                return Err("a name span in an anonymous block".into());
            }
            span_variable = variable.to_string();
            continue;
        }
        let rest = statement
            .strip_prefix("let")
            .ok_or_else(|| format!("unknown `{statement}`"))?;
        let (variable, value) = rest
            .split_once("=||")
            .ok_or_else(|| format!("unknown `{statement}`"))?;
        closures.push((variable.trim().to_string(), value.trim().to_string()));
    }
    let literal = literal.trim().strip_suffix("})").ok_or("no literal end")?;
    let (_, struct_fields) = ast
        .structs
        .iter()
        .find(|(struct_name, _)| struct_name == returned)
        .ok_or("no struct")?;
    let mut defaults: Vec<(String, Default)> = Vec::new();
    for (index, piece) in split_top(literal).iter().enumerate() {
        let (field, expression) = match piece.split_once(':') {
            Some((field, expression)) if !field.contains('(') => {
                (field.trim().to_string(), expression.trim().to_string())
            }
            _ => (piece.trim().to_string(), piece.trim().to_string()),
        };
        if struct_fields.get(index).map(|(field_name, _)| field_name) != Some(&field) {
            return Err(format!("field `{field}` out of order"));
        }
        if index == 0 && header.is_some() {
            if field != "name" || expression != "name" {
                return Err("the first field is not the name".into());
            }
            continue;
        }
        let variable: String = expression
            .chars()
            .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
            .collect();
        if !fields.iter().any(|known| known.variable == variable) {
            return Err(format!("field `{field}` from `{expression}`"));
        }
        let default_text = expression[variable.len()..].trim();
        let default = if default_text.is_empty() {
            Default::None
        } else if default_text == ".unwrap_or_default()" {
            if !struct_fields[index].1.starts_with("Vec<") {
                return Err(format!(
                    "field `{field}`: default of `{}`",
                    struct_fields[index].1
                ));
            }
            Default::Array
        } else if let Some(value) = default_text
            .strip_prefix(".unwrap_or_else(")
            .and_then(|value| value.strip_suffix(')'))
        {
            let made = if let Some(inline) = value.strip_prefix("||") {
                inline.trim().to_string()
            } else {
                closures
                    .iter()
                    .find(|(name, _)| name == value)
                    .map(|(_, made)| made.clone())
                    .ok_or_else(|| format!("field `{field}`: unknown default `{value}`"))?
            };
            let made = made
                .trim()
                .trim_start_matches('{')
                .trim_end_matches('}')
                .trim()
                .to_string();
            let spanned = made
                .strip_prefix("Spanned::new(")
                .and_then(|value| {
                    value
                        .strip_suffix(&format!(",{span_variable})"))
                        .or_else(|| value.strip_suffix(&format!(",{span_variable},)")))
                })
                .ok_or_else(|| format!("field `{field}`: unknown default `{made}`"))?;
            if spanned == "String::new()" {
                Default::EmptyString
            } else if let Some(variant) = spanned.strip_suffix("(String::new())") {
                let (enum_name, variant) = variant.split_once("::").ok_or("no enum")?;
                Default::Unknown(enum_name.to_string(), variant.to_string())
            } else if let Some((enum_name, variant)) = spanned.split_once("::") {
                Default::Variant(enum_name.to_string(), variant.to_string())
            } else {
                return Err(format!("field `{field}`: unknown default `{made}`"));
            }
        } else {
            return Err(format!("field `{field}`: unknown default `{default_text}`"));
        };
        defaults.push((field.clone(), default));
    }
    // Every variable must be a field of the same name.
    for known in &fields {
        if !struct_fields
            .iter()
            .any(|(field_name, _)| *field_name == known.variable)
        {
            return Err(format!("slot `{}` is no field", known.variable));
        }
    }

    // The Argorix port.
    let upper = upper_snake(returned);
    let field_constant =
        |field: &str| format!("agent_schema.{upper}_{}", field.to_ascii_uppercase());
    let variant_constant = |enum_name: &str, variant: &str| {
        format!(
            "agent_schema.{}_{}",
            upper_snake(enum_name),
            upper_snake(variant)
        )
    };
    let enum_constant = |enum_name: &str| -> Result<String, String> {
        if !ast.enums.iter().any(|(known, _)| known == enum_name) {
            return Err(format!("no enum `{enum_name}`"));
        }
        Ok(format!("agent_schema.{}", upper_snake(enum_name)))
    };
    let mut out = String::new();
    let _ = writeln!(out, "\n// `{name}`.");
    let _ = writeln!(
        out,
        "fn {name}(s: Slice<u8>, t: Slice<AgentToken>, parser: Tree) -> Tree {{"
    );
    match &header {
        Some((keyword, name_description)) => {
            let _ = writeln!(
                out,
                "    let mut p: Tree = block_start(s, t, parser, \"{keyword}\".as_slice(), \"{name_description}\".as_slice(), agent_schema.{upper});"
            );
            out.push_str("    if p.failed { return p; }\n    let node: u64 = p.result;\n");
        }
        None => {
            out.push_str("    let open: u64 = parser.current;\n");
            out.push_str("    let mut p: Tree = expect_symbol(t, parser, agent_lexer.LEFT_BRACE, \"`{`\".as_slice());\n");
            let _ = writeln!(out, "    if p.failed {{ return p; }}\n    p = agent_ast.object(p, agent_schema.{upper});\n    let node: u64 = p.result;");
        }
    }
    out.push_str("    while peek_kind(t, p.current) != agent_lexer.RIGHT_BRACE {\n");
    let _ = writeln!(
        out,
        "        p = ensure_not_eof(t, p, \"{unterminated}\".as_slice());"
    );
    out.push_str("        if p.failed { return p; }\n");
    for (index, field) in fields.iter().enumerate() {
        let lead = if index == 0 {
            "        if"
        } else {
            "        } else if"
        };
        let _ = writeln!(
            out,
            "{lead} peek_is(s, t, p.current, \"{}\".as_slice()) {{",
            field.key
        );
        let target = field_constant(&field.variable);
        let block = &field.block;
        let call = match &field.kind {
            Kind::Ident(description) => format!("field_ident(s, t, p, node, {target}, \"{block}\".as_slice(), \"{description}\".as_slice())"),
            Kind::Str(description) => format!("field_string(s, t, p, node, {target}, \"{block}\".as_slice(), \"{description}\".as_slice())"),
            Kind::Int(description) => format!("field_integer(s, t, p, node, {target}, \"{block}\".as_slice(), \"{description}\".as_slice())"),
            Kind::Bool(description) => format!("field_bool(s, t, p, node, {target}, \"{block}\".as_slice(), \"{description}\".as_slice())"),
            Kind::StrArray(description) => format!("field_string_array(s, t, p, node, {target}, \"{block}\".as_slice(), \"{description}\".as_slice())"),
            Kind::IdentBlock(description) => format!("field_identifier_block(s, t, p, node, {target}, \"{block}\".as_slice(), \"{description}\".as_slice())"),
            Kind::List(function) => format!("field_{function}(s, t, p, node, {target}, \"{block}\".as_slice())"),
            Kind::Enum { description, enum_name, fallback, table_function } => format!(
                "field_enum(s, t, p, node, {target}, \"{block}\".as_slice(), \"{description}\".as_slice(), {}, agent_words.T_{}__{}, {})",
                enum_constant(enum_name)?,
                table_function.as_deref().unwrap_or(name).to_ascii_uppercase(),
                upper_snake(enum_name),
                variant_constant(enum_name, fallback)
            ),
        };
        let _ = writeln!(out, "            p = {call};");
    }
    out.push_str("        } else {\n");
    let _ = writeln!(out, "            return block_unexpected(s, t, p, \"{block_name}\".as_slice(), \"{article}\".as_slice());");
    out.push_str("        }\n        if p.failed { return p; }\n    }\n");
    let spanned_default = defaults.iter().any(|(_, default)| {
        matches!(
            default,
            Default::EmptyString | Default::Unknown(..) | Default::Variant(..)
        )
    });
    if spanned_default {
        if header.is_some() {
            out.push_str("    let fallback: u64 = name_span(p.slots.as_slice(), p.nodes.as_slice(), node);\n");
        } else {
            out.push_str("    p = span_of(t, p, open);\n    let fallback: u64 = p.result;\n");
        }
    }
    for (field, default) in &defaults {
        let target = field_constant(field);
        match default {
            Default::None => {}
            Default::Array => {
                let _ = writeln!(out, "    p = default_array(p, node, {target});");
            }
            Default::EmptyString => {
                let _ = writeln!(out, "    p = default_string(p, node, {target}, fallback);");
            }
            Default::Unknown(enum_name, variant) => {
                let _ = writeln!(
                    out,
                    "    p = default_unknown(p, node, {target}, {}, {}, fallback);",
                    enum_constant(enum_name)?,
                    variant_constant(enum_name, variant)
                );
            }
            Default::Variant(enum_name, variant) => {
                let _ = writeln!(
                    out,
                    "    p = default_variant(p, node, {target}, {}, {}, fallback);",
                    enum_constant(enum_name)?,
                    variant_constant(enum_name, variant)
                );
            }
        }
    }
    out.push_str("    block_end(t, p, node)\n}\n");
    Ok(out)
}

/// The port of a list function, `[ entry, entry ]` with optional commas, and
/// of the key whose value it is.
fn port_list(name: &str, body: &str) -> Result<String, String> {
    let body = squeeze(body);
    let rest = body
        .strip_prefix("self.expect_symbol(TokenKind::LeftBracket,\"`[`\")?;letmut")
        .ok_or("not a list")?;
    let (variable, rest) = rest.split_once("=Vec::new();").ok_or("not a list")?;
    let rest = rest
        .strip_prefix("while!self.check(&TokenKind::RightBracket){self.ensure_not_eof(")
        .ok_or("not a list")?;
    let (unterminated, rest) = rest.split_once(")?;").ok_or("not a list")?;
    let unterminated = quoted(unterminated).ok_or("not a literal message")?;
    let rest = rest
        .strip_prefix(&format!("{variable}.push(self."))
        .ok_or("not a list")?;
    let (entry, rest) = rest.split_once("()?);").ok_or("not a list")?;
    if rest
        != format!(
            "ifself.check(&TokenKind::Comma){{self.advance();}}}}self.advance();Ok({variable})"
        )
    {
        return Err(format!("not a list: `{rest}`"));
    }
    let mut out = String::new();
    let _ = writeln!(out, "\n// `{name}`.");
    let _ = writeln!(
        out,
        "fn {name}(s: Slice<u8>, t: Slice<AgentToken>, parser: Tree) -> Tree {{"
    );
    out.push_str("    let mut p: Tree = expect_symbol(t, parser, agent_lexer.LEFT_BRACKET, \"`[`\".as_slice());\n");
    out.push_str("    if p.failed { return p; }\n    p = agent_ast.array(p);\n    let values: u64 = p.result;\n");
    out.push_str("    while peek_kind(t, p.current) != agent_lexer.RIGHT_BRACKET {\n");
    let _ = writeln!(
        out,
        "        p = ensure_not_eof(t, p, \"{unterminated}\".as_slice());"
    );
    out.push_str("        if p.failed { return p; }\n");
    let _ = writeln!(out, "        p = {entry}(s, t, p);");
    out.push_str("        if p.failed { return p; }\n        let value: u64 = p.result;\n        p = agent_ast.push(p, values, value);\n");
    out.push_str("        if peek_kind(t, p.current) == agent_lexer.COMMA {\n            p = advance(t, p);\n        }\n    }\n");
    out.push_str("    p = advance(t, p);\n    p.result = values;\n    p\n}\n");
    let _ = writeln!(out, "\n// A key whose value `{name}` reads.");
    let _ = writeln!(out, "fn field_{name}(s: Slice<u8>, t: Slice<AgentToken>, parser: Tree, node: u64, field: u64, block: Slice<u8>) -> Tree {{");
    out.push_str("    let mut p: Tree = field_start(s, t, parser, node, field, block);\n    if p.failed { return p; }\n");
    let _ = writeln!(out, "    p = {name}(s, t, p);");
    out.push_str("    if p.failed { return p; }\n    agent_ast.set_last(p, node, field)\n}\n");
    Ok(out)
}

/// The generated region, and the functions left for a hand port with why.
fn generate() -> (String, Vec<(String, String)>) {
    let text = fs::read_to_string(root().join("crates/argorix_parser/src/parser.rs")).unwrap();
    let text = text.split("#[cfg(test)]").next().unwrap().to_string();
    let ast = read_ast();
    let mut out = String::from(BEGIN);
    out.push_str(
        "\n// GENERATED by crates/argorixc/tests/agent_blocks.rs from the keyed-block\n// declarations of crates/argorix_parser/src/parser.rs; regenerate with\n// ARGORIX_BLESS=1, do not edit.\n",
    );
    let mut skipped = Vec::new();
    for (name, returned, body) in functions(&text) {
        if returned.starts_with("Vec<") {
            if let Ok(code) = port_list(&name, &body) {
                out.push_str(&code);
            }
            continue;
        }
        if !body.contains("_field(") {
            continue;
        }
        match port(&name, &returned, &body, &ast, &text) {
            Ok(code) => out.push_str(&code),
            Err(why) => skipped.push((name, why)),
        }
    }
    out.push('\n');
    out.push_str(END);
    (out, skipped)
}

#[test]
fn the_generated_keyed_blocks_are_what_the_parser_says() {
    let path = root().join("compiler/agent_parser.argx");
    let file = fs::read_to_string(&path).unwrap();
    let (expected, skipped) = generate();
    for (name, why) in &skipped {
        eprintln!("hand port: {name}: {why}");
    }
    let begin = file
        .find(BEGIN)
        .expect("no generated region in agent_parser.argx");
    let end = file.find(END).expect("no end of the generated region") + END.len();
    let found = &file[begin..end];
    if env::var_os("ARGORIX_BLESS").is_some() && found != expected {
        let updated = format!("{}{}{}", &file[..begin], expected, &file[end..]);
        fs::write(&path, updated).unwrap();
        return;
    }
    assert!(
        found == expected,
        "the generated keyed blocks of compiler/agent_parser.argx are stale; regenerate them with ARGORIX_BLESS=1"
    );
}

#[test]
#[ignore]
fn show_squeezed() {
    let text = fs::read_to_string(root().join("crates/argorix_parser/src/parser.rs")).unwrap();
    for (name, _, body) in functions(&text) {
        if name == "parse_did_method" {
            println!("{}", squeeze(&body));
        }
    }
}
