//! Transitional C11 backend for verified Argorix Core IR.
//!
//! ESP-008 owns this Rust stage0 emitter. It intentionally accepts only the
//! proof wrapper created by the Core IR verifier.

use crate::core::{
    CoreIrAssignOp, CoreIrBackend, CoreIrBinaryOp, CoreIrBlock, CoreIrEnum, CoreIrExpr,
    CoreIrFunction, CoreIrItemKind, CoreIrPattern, CoreIrStatement, CoreIrStruct, CoreIrType,
    CoreIrUnaryOp, VerifiedCoreIr,
};
use std::collections::BTreeMap;
use std::error::Error;
use std::fmt::{self, Write};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoreCOutput {
    pub source: String,
    pub entry_function: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoreCError {
    pub code: &'static str,
    pub message: String,
}

impl CoreCError {
    fn unsupported(message: impl Into<String>) -> Self {
        Self {
            code: "CBackendUnsupported",
            message: message.into(),
        }
    }
}

impl fmt::Display for CoreCError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}: {}", self.code, self.message)
    }
}

impl Error for CoreCError {}

#[derive(Debug, Default)]
pub struct CoreCBackend;

impl CoreIrBackend for CoreCBackend {
    type Output = CoreCOutput;
    type Error = CoreCError;

    fn emit(&self, verified: VerifiedCoreIr<'_>) -> Result<Self::Output, Self::Error> {
        Emitter::new(verified).emit()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Signature {
    parameters: Vec<ScalarType>,
    result: ScalarType,
}

#[derive(Debug, Clone)]
struct StructLayout {
    fields: BTreeMap<String, ScalarType>,
}

#[derive(Debug, Clone)]
struct EnumLayout {
    variants: BTreeMap<String, BTreeMap<String, ScalarType>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum ScalarType {
    Unit,
    Bool,
    Bytes,
    String,
    Integer(String),
    User(String),
    Slice(Box<ScalarType>),
    Buffer(Box<ScalarType>),
    Arena(Box<ScalarType>),
    Handle(Box<ScalarType>),
    Array {
        element: Box<ScalarType>,
        length: u64,
    },
}

impl ScalarType {
    fn from_ir(value: &CoreIrType) -> Result<Self, CoreCError> {
        match value {
            CoreIrType::Named { name } => match name.as_str() {
                "unit" => Ok(Self::Unit),
                "bool" => Ok(Self::Bool),
                "bytes" => Ok(Self::Bytes),
                "string" => Ok(Self::String),
                "u8" | "u16" | "u32" | "u64" | "i8" | "i16" | "i32" | "i64" => {
                    Ok(Self::Integer(name.clone()))
                }
                _ => Ok(Self::User(name.clone())),
            },
            CoreIrType::Container {
                name,
                element,
                array_length: Some(length),
            } if name == "Array" => Ok(Self::Array {
                element: Box::new(Self::from_ir(element)?),
                length: *length,
            }),
            CoreIrType::Container {
                name,
                element,
                array_length: None,
            } if name == "Slice" => Ok(Self::Slice(Box::new(Self::from_ir(element)?))),
            CoreIrType::Container {
                name,
                element,
                array_length: None,
            } if name == "Buffer" => Ok(Self::Buffer(Box::new(Self::from_ir(element)?))),
            CoreIrType::Container {
                name,
                element,
                array_length: None,
            } if name == "Arena" => Ok(Self::Arena(Box::new(Self::from_ir(element)?))),
            CoreIrType::Container {
                name,
                element,
                array_length: None,
            } if name == "Handle" => Ok(Self::Handle(Box::new(Self::from_ir(element)?))),
            CoreIrType::Container { .. } => Err(CoreCError::unsupported(format!(
                "container type `{value:?}` is not lowered by this C profile"
            ))),
        }
    }

    fn c_name(&self) -> String {
        match self {
            Self::Unit => "void".into(),
            Self::Bool => "bool".into(),
            Self::Bytes => "argorix_bytes".into(),
            Self::String => "argorix_string".into(),
            Self::Integer(name) => match name.as_str() {
                "u8" => "uint8_t".into(),
                "u16" => "uint16_t".into(),
                "u32" => "uint32_t".into(),
                "u64" => "uint64_t".into(),
                "i8" => "int8_t".into(),
                "i16" => "int16_t".into(),
                "i32" => "int32_t".into(),
                "i64" => "int64_t".into(),
                _ => unreachable!("validated scalar integer"),
            },
            Self::User(name) => format!("argorix_type_{name}"),
            Self::Slice(element) => format!("argorix_slice_{}", element.mangle()),
            Self::Buffer(_) => "argorix_buffer".into(),
            Self::Arena(_) => "argorix_arena".into(),
            Self::Handle(_) => "argorix_handle".into(),
            Self::Array { element, length } => {
                format!("argorix_array_{}_{}", element.mangle(), length)
            }
        }
    }

    fn mangle(&self) -> String {
        match self {
            Self::Unit => "unit".into(),
            Self::Bool => "bool".into(),
            Self::Bytes => "bytes".into(),
            Self::String => "string".into(),
            Self::Integer(name) => name.clone(),
            Self::User(name) => format!("type_{name}"),
            Self::Slice(element) => format!("slice_{}", element.mangle()),
            Self::Buffer(element) => format!("buffer_{}", element.mangle()),
            Self::Arena(element) => format!("arena_{}", element.mangle()),
            Self::Handle(element) => format!("handle_{}", element.mangle()),
            Self::Array { element, length } => format!("array_{}_{}", element.mangle(), length),
        }
    }

    fn helper_suffix(&self) -> Result<&str, CoreCError> {
        match self {
            Self::Integer(name) => Ok(name),
            _ => Err(CoreCError::unsupported(
                "checked arithmetic requires an integer",
            )),
        }
    }
}

struct Emitter<'a> {
    verified: VerifiedCoreIr<'a>,
    signatures: BTreeMap<String, Signature>,
    structs: BTreeMap<String, StructLayout>,
    enums: BTreeMap<String, EnumLayout>,
}

impl<'a> Emitter<'a> {
    fn new(verified: VerifiedCoreIr<'a>) -> Self {
        Self {
            verified,
            signatures: BTreeMap::new(),
            structs: BTreeMap::new(),
            enums: BTreeMap::new(),
        }
    }

    fn emit(mut self) -> Result<CoreCOutput, CoreCError> {
        let program = self.verified.program();
        for item in &program.items {
            match &item.kind {
                CoreIrItemKind::Function(function) => {
                    let parameters = function
                        .parameters
                        .iter()
                        .map(|parameter| ScalarType::from_ir(&parameter.ty))
                        .collect::<Result<Vec<_>, _>>()?;
                    self.signatures.insert(
                        function.name.clone(),
                        Signature {
                            parameters,
                            result: ScalarType::from_ir(&function.return_type)?,
                        },
                    );
                }
                CoreIrItemKind::Struct(value) => {
                    self.structs
                        .insert(value.name.clone(), struct_layout(value)?);
                }
                CoreIrItemKind::Enum(value) => {
                    self.enums.insert(value.name.clone(), enum_layout(value)?);
                }
                CoreIrItemKind::Const(_) => {
                    return Err(CoreCError::unsupported(
                        "constant lowering is not implemented yet",
                    ));
                }
            }
        }
        let entry = self
            .signatures
            .get("argorix_main")
            .ok_or_else(|| CoreCError::unsupported("module must define `argorix_main`"))?;
        if !entry.parameters.is_empty()
            || !matches!(entry.result, ScalarType::Bool | ScalarType::Integer(_))
        {
            return Err(CoreCError::unsupported(
                "`argorix_main` must take no parameters and return a scalar",
            ));
        }

        let mut source = String::new();
        source.push_str(
            "/* generated by Argorix Core C backend 0.1 */\n\
             #include \"argorix_core_runtime.h\"\n\
             #include <inttypes.h>\n\
             #include <stdbool.h>\n\
             #include <stdint.h>\n\
             #include <stdio.h>\n\
             #ifndef ARGORIX_STEP_LIMIT\n\
             #define ARGORIX_STEP_LIMIT 1000000U\n\
             #endif\n\
             #ifndef ARGORIX_BUFFER_LIMIT_BYTES\n\
             #define ARGORIX_BUFFER_LIMIT_BYTES 1048576U\n\
             #endif\n\
             #ifndef ARGORIX_ARENA_LIMIT_BYTES\n\
             #define ARGORIX_ARENA_LIMIT_BYTES 1048576U\n\
             #endif\n\
             #ifndef ARGORIX_ARENA_SLOT_LIMIT\n\
             #define ARGORIX_ARENA_SLOT_LIMIT 1024U\n\
             #endif\n\n",
        );
        let mut arrays = BTreeMap::new();
        for signature in self.signatures.values() {
            for parameter in &signature.parameters {
                collect_array_type(parameter, &mut arrays);
            }
            collect_array_type(&signature.result, &mut arrays);
        }
        for layout in self.structs.values() {
            for field in layout.fields.values() {
                collect_array_type(field, &mut arrays);
            }
        }
        for layout in self.enums.values() {
            for fields in layout.variants.values() {
                for field in fields.values() {
                    collect_array_type(field, &mut arrays);
                }
            }
        }
        for item in &program.items {
            if let CoreIrItemKind::Function(function) = &item.kind {
                collect_block_array_types(&function.body, &mut arrays)?;
            }
        }
        for name in self.structs.keys().chain(self.enums.keys()) {
            writeln!(
                source,
                "typedef struct argorix_type_{name} argorix_type_{name};"
            )
            .unwrap();
        }
        if !self.structs.is_empty() || !self.enums.is_empty() {
            source.push('\n');
        }
        for ty in arrays.values() {
            match ty {
                ScalarType::Array { element, length } => {
                    writeln!(
                        source,
                        "typedef struct {{ {} data[{}]; }} {};",
                        element.c_name(),
                        length,
                        ty.c_name()
                    )
                    .unwrap();
                }
                ScalarType::Slice(element) => {
                    writeln!(
                        source,
                        "typedef struct {{ const {} *data; uint64_t length; }} {};",
                        element.c_name(),
                        ty.c_name()
                    )
                    .unwrap();
                }
                _ => unreachable!(),
            }
        }
        if !arrays.is_empty() {
            source.push('\n');
        }
        for (name, layout) in &self.structs {
            writeln!(source, "struct argorix_type_{name} {{").unwrap();
            for (field, ty) in &layout.fields {
                writeln!(source, "    {} {field};", ty.c_name()).unwrap();
            }
            source.push_str("};\n\n");
        }
        for (name, layout) in &self.enums {
            writeln!(source, "typedef enum argorix_tag_{name} {{").unwrap();
            for variant in layout.variants.keys() {
                writeln!(source, "    argorix_tag_{name}_{variant},").unwrap();
            }
            writeln!(source, "}} argorix_tag_{name};").unwrap();
            writeln!(source, "struct argorix_type_{name} {{").unwrap();
            writeln!(source, "    argorix_tag_{name} tag;").unwrap();
            source.push_str("    union {\n");
            for (variant, fields) in &layout.variants {
                source.push_str("        struct {\n");
                if fields.is_empty() {
                    source.push_str("            uint8_t _unit;\n");
                } else {
                    for (field, ty) in fields {
                        writeln!(source, "            {} {field};", ty.c_name()).unwrap();
                    }
                }
                writeln!(source, "        }} {variant};").unwrap();
            }
            source.push_str("    } data;\n};\n\n");
        }
        for item in &program.items {
            if let CoreIrItemKind::Function(function) = &item.kind {
                self.emit_prototype(function, &mut source)?;
            }
        }
        source.push('\n');
        for item in &program.items {
            if let CoreIrItemKind::Function(function) = &item.kind {
                FunctionEmitter::new(&self.signatures, &self.structs, &self.enums, function)
                    .emit(&mut source)?;
            }
        }
        self.emit_main(entry, &mut source)?;
        Ok(CoreCOutput {
            source,
            entry_function: "argorix_main".into(),
        })
    }

    fn emit_prototype(
        &self,
        function: &CoreIrFunction,
        source: &mut String,
    ) -> Result<(), CoreCError> {
        let signature = &self.signatures[&function.name];
        write!(
            source,
            "static {} argorix_fn_{}(argorix_budget *budget",
            signature.result.c_name(),
            function.name
        )
        .unwrap();
        for (parameter, ty) in function.parameters.iter().zip(&signature.parameters) {
            write!(source, ", {} argorix_v_{}", ty.c_name(), parameter.name).unwrap();
        }
        source.push_str(");\n");
        Ok(())
    }

    fn emit_main(&self, entry: &Signature, source: &mut String) -> Result<(), CoreCError> {
        source.push_str("int main(void) {\n    argorix_budget budget = {ARGORIX_STEP_LIMIT};\n");
        writeln!(
            source,
            "    {} result = argorix_fn_argorix_main(&budget);",
            entry.result.c_name()
        )
        .unwrap();
        match &entry.result {
            ScalarType::Bool => {
                source.push_str(
                    "    (void)printf(\"ARGORIX_RESULT:%s\\n\", result ? \"true\" : \"false\");\n",
                );
            }
            ScalarType::Integer(name) if name.starts_with('u') => {
                source.push_str(
                    "    (void)printf(\"ARGORIX_RESULT:%\" PRIu64 \"\\n\", (uint64_t)result);\n",
                );
            }
            ScalarType::Integer(_) => {
                source.push_str(
                    "    (void)printf(\"ARGORIX_RESULT:%\" PRId64 \"\\n\", (int64_t)result);\n",
                );
            }
            ScalarType::Unit
            | ScalarType::Bytes
            | ScalarType::String
            | ScalarType::User(_)
            | ScalarType::Slice(_)
            | ScalarType::Buffer(_)
            | ScalarType::Arena(_)
            | ScalarType::Handle(_)
            | ScalarType::Array { .. } => unreachable!(),
        }
        source.push_str("    return 0;\n}\n");
        Ok(())
    }
}

struct FunctionEmitter<'a> {
    signatures: &'a BTreeMap<String, Signature>,
    structs: &'a BTreeMap<String, StructLayout>,
    enums: &'a BTreeMap<String, EnumLayout>,
    function: &'a CoreIrFunction,
    locals: BTreeMap<String, ScalarType>,
    temporary: usize,
}

impl<'a> FunctionEmitter<'a> {
    fn new(
        signatures: &'a BTreeMap<String, Signature>,
        structs: &'a BTreeMap<String, StructLayout>,
        enums: &'a BTreeMap<String, EnumLayout>,
        function: &'a CoreIrFunction,
    ) -> Self {
        let locals = function
            .parameters
            .iter()
            .map(|parameter| {
                (
                    parameter.name.clone(),
                    ScalarType::from_ir(&parameter.ty).expect("signature was validated"),
                )
            })
            .collect();
        Self {
            signatures,
            structs,
            enums,
            function,
            locals,
            temporary: 0,
        }
    }

    fn emit(mut self, source: &mut String) -> Result<(), CoreCError> {
        let signature = &self.signatures[&self.function.name];
        write!(
            source,
            "static {} argorix_fn_{}(argorix_budget *budget",
            signature.result.c_name(),
            self.function.name
        )
        .unwrap();
        for (parameter, ty) in self.function.parameters.iter().zip(&signature.parameters) {
            write!(source, ", {} argorix_v_{}", ty.c_name(), parameter.name).unwrap();
        }
        source.push_str(") {\n    argorix_step(budget);\n");
        self.emit_block(&self.function.body, 1, Some(&signature.result), source)?;
        if signature.result == ScalarType::Unit {
            self.emit_buffer_drops(1, source);
            source.push_str("    return;\n");
        }
        source.push_str("}\n\n");
        Ok(())
    }

    fn emit_block(
        &mut self,
        block: &CoreIrBlock,
        indent: usize,
        tail_type: Option<&ScalarType>,
        source: &mut String,
    ) -> Result<(), CoreCError> {
        for statement in &block.statements {
            self.emit_statement(statement, indent, source)?;
        }
        if let Some(tail) = &block.tail {
            let expected = tail_type.ok_or_else(|| {
                CoreCError::unsupported("value block needs an expected scalar type")
            })?;
            let value = self.emit_expr(tail, Some(expected), indent, source)?;
            self.emit_buffer_drops(indent, source);
            line(source, indent, &format!("return {};", value.0));
        }
        Ok(())
    }

    /// Release every `Buffer` local before leaving the function.
    ///
    /// `argorix_buffer_new` allocates on first push, so a buffer that is never
    /// dropped leaks its storage. `argorix_buffer_drop` clears the pointer, so
    /// emitting it on more than one exit path is safe. The result is computed
    /// into a temporary before these calls, and a `Buffer` cannot be returned,
    /// so nothing here can be read after it is freed.
    fn emit_buffer_drops(&self, indent: usize, source: &mut String) {
        for (name, ty) in &self.locals {
            if matches!(ty, ScalarType::Buffer(_)) {
                line(
                    source,
                    indent,
                    &format!("argorix_buffer_drop(&argorix_v_{name});"),
                );
            }
        }
    }

    fn emit_statement(
        &mut self,
        statement: &CoreIrStatement,
        indent: usize,
        source: &mut String,
    ) -> Result<(), CoreCError> {
        match statement {
            CoreIrStatement::Let {
                name,
                annotation,
                value,
                ..
            } => {
                let declared = annotation.as_ref().map(ScalarType::from_ir).transpose()?;
                let emitted = self.emit_expr(value, declared.as_ref(), indent, source)?;
                let ty = declared.unwrap_or(emitted.1);
                line(
                    source,
                    indent,
                    &format!("{} argorix_v_{} = {};", ty.c_name(), name, emitted.0),
                );
                self.locals.insert(name.clone(), ty);
            }
            CoreIrStatement::Assign {
                target,
                operator,
                value,
            } => {
                let CoreIrExpr::Path { segments } = target else {
                    return Err(CoreCError::unsupported(
                        "scalar C assignment target must be a local path",
                    ));
                };
                let name = single_path(segments)?;
                let ty =
                    self.locals.get(name).cloned().ok_or_else(|| {
                        CoreCError::unsupported(format!("unknown local `{name}`"))
                    })?;
                let right = self.emit_expr(value, Some(&ty), indent, source)?;
                let assignment = match operator {
                    CoreIrAssignOp::Assign => right.0,
                    CoreIrAssignOp::Add
                    | CoreIrAssignOp::Subtract
                    | CoreIrAssignOp::Multiply
                    | CoreIrAssignOp::Divide
                    | CoreIrAssignOp::Remainder => {
                        let operation = assign_operation(*operator);
                        format!(
                            "argorix_{}_{}(argorix_v_{}, {})",
                            ty.helper_suffix()?,
                            operation,
                            name,
                            right.0
                        )
                    }
                };
                line(source, indent, &format!("argorix_v_{name} = {assignment};"));
            }
            CoreIrStatement::While { condition, body } => {
                line(source, indent, "while (true) {");
                line(source, indent + 1, "argorix_step(budget);");
                let condition =
                    self.emit_expr(condition, Some(&ScalarType::Bool), indent + 1, source)?;
                line(
                    source,
                    indent + 1,
                    &format!("if (!{}) {{ break; }}", condition.0),
                );
                self.emit_block(body, indent + 1, None, source)?;
                line(source, indent, "}");
            }
            CoreIrStatement::Return { value } => {
                if let Some(value) = value {
                    let result = &self.signatures[&self.function.name].result;
                    let value = self.emit_expr(value, Some(result), indent, source)?;
                    self.emit_buffer_drops(indent, source);
                    line(source, indent, &format!("return {};", value.0));
                } else {
                    self.emit_buffer_drops(indent, source);
                    line(source, indent, "return;");
                }
            }
            CoreIrStatement::Expr { value } => {
                let _ = self.emit_expr(value, None, indent, source)?;
            }
            CoreIrStatement::Break { value: None } => line(source, indent, "break;"),
            CoreIrStatement::Continue => line(source, indent, "continue;"),
            CoreIrStatement::Break { value: Some(_) } => {
                return Err(CoreCError::unsupported(
                    "value breaks are not in scalar C profile",
                ));
            }
        }
        Ok(())
    }

    fn emit_expr(
        &mut self,
        expression: &CoreIrExpr,
        expected: Option<&ScalarType>,
        indent: usize,
        source: &mut String,
    ) -> Result<(String, ScalarType), CoreCError> {
        match expression {
            CoreIrExpr::Integer { value, suffix } => {
                let ty = suffix
                    .as_ref()
                    .map(|name| ScalarType::Integer(name.clone()))
                    .or_else(|| expected.cloned())
                    .ok_or_else(|| CoreCError::unsupported("integer literal needs a type"))?;
                Ok((format!("{value}{}", integer_literal_suffix(&ty)?), ty))
            }
            CoreIrExpr::Bool { value } => Ok((value.to_string(), ScalarType::Bool)),
            CoreIrExpr::Path { segments } => {
                if let [name, variant] = segments.as_slice() {
                    let fields = self
                        .enums
                        .get(name)
                        .and_then(|layout| layout.variants.get(variant))
                        .ok_or_else(|| {
                            CoreCError::unsupported(format!(
                                "unknown enum variant `{name}::{variant}`"
                            ))
                        })?;
                    if !fields.is_empty() {
                        return Err(CoreCError::unsupported(format!(
                            "variant `{name}::{variant}` requires fields"
                        )));
                    }
                    let ty = ScalarType::User(name.clone());
                    let temp = self.next_temp();
                    line(source, indent, &format!("{} {temp};", ty.c_name()));
                    line(
                        source,
                        indent,
                        &format!("{temp}.tag = argorix_tag_{name}_{variant};"),
                    );
                    line(
                        source,
                        indent,
                        &format!("{temp}.data.{variant}._unit = 0U;"),
                    );
                    return Ok((temp, ty));
                }
                let name = single_path(segments)?;
                let ty = self.locals.get(name).cloned().ok_or_else(|| {
                    CoreCError::unsupported(format!("path `{name}` is not a scalar local"))
                })?;
                Ok((format!("argorix_v_{name}"), ty))
            }
            CoreIrExpr::Aggregate { path, fields } => {
                let (name, variant) = match path.as_slice() {
                    [name] => (name.as_str(), None),
                    [name, variant] => (name.as_str(), Some(variant.as_str())),
                    _ => {
                        return Err(CoreCError::unsupported(format!(
                            "aggregate path `{}` is not supported",
                            path.join("::")
                        )));
                    }
                };
                let ty = expected
                    .cloned()
                    .unwrap_or_else(|| ScalarType::User(name.into()));
                if ty != ScalarType::User(name.into()) {
                    return Err(CoreCError::unsupported(
                        "aggregate path does not match its expected type",
                    ));
                }
                let temp = self.next_temp();
                line(source, indent, &format!("{} {temp};", ty.c_name()));
                let layout = if let Some(variant) = variant {
                    let layout = self
                        .enums
                        .get(name)
                        .and_then(|layout| layout.variants.get(variant))
                        .cloned()
                        .ok_or_else(|| {
                            CoreCError::unsupported(format!(
                                "unknown enum variant `{name}::{variant}`"
                            ))
                        })?;
                    line(
                        source,
                        indent,
                        &format!("{temp}.tag = argorix_tag_{name}_{variant};"),
                    );
                    if layout.is_empty() {
                        line(
                            source,
                            indent,
                            &format!("{temp}.data.{variant}._unit = 0U;"),
                        );
                    }
                    layout
                } else {
                    self.structs
                        .get(name)
                        .map(|layout| layout.fields.clone())
                        .ok_or_else(|| {
                            CoreCError::unsupported(format!("`{name}` is not a lowered struct"))
                        })?
                };
                for field in fields {
                    let field_ty = layout.get(&field.name).ok_or_else(|| {
                        CoreCError::unsupported(format!(
                            "unknown field `{}` on `{name}`",
                            field.name
                        ))
                    })?;
                    let value = self.emit_expr(&field.value, Some(field_ty), indent, source)?;
                    let access = variant
                        .map(|variant| format!("data.{variant}.{}", field.name))
                        .unwrap_or_else(|| field.name.clone());
                    line(source, indent, &format!("{temp}.{access} = {};", value.0));
                }
                Ok((temp, ty))
            }
            CoreIrExpr::Array { values } => {
                let ty = expected.cloned().ok_or_else(|| {
                    CoreCError::unsupported("array literal needs an expected array type")
                })?;
                let ScalarType::Array { element, length } = &ty else {
                    return Err(CoreCError::unsupported(
                        "array literal has a non-array expected type",
                    ));
                };
                if values.len() as u64 != *length {
                    return Err(CoreCError::unsupported("array literal length mismatch"));
                }
                let temp = self.next_temp();
                line(source, indent, &format!("{} {temp};", ty.c_name()));
                for (index, value) in values.iter().enumerate() {
                    let value = self.emit_expr(value, Some(element), indent, source)?;
                    line(
                        source,
                        indent,
                        &format!("{temp}.data[{index}] = {};", value.0),
                    );
                }
                Ok((temp, ty))
            }
            CoreIrExpr::Index { value, index } => {
                let value = self.emit_expr(value, None, indent, source)?;
                let (element, length, buffer) = match &value.1 {
                    ScalarType::Array { element, length } => {
                        ((**element).clone(), format!("{}U", length), false)
                    }
                    ScalarType::Slice(element) => {
                        ((**element).clone(), format!("{}.length", value.0), false)
                    }
                    ScalarType::Bytes => (
                        ScalarType::Integer("u8".into()),
                        format!("{}.length", value.0),
                        false,
                    ),
                    ScalarType::Buffer(element) => {
                        ((**element).clone(), format!("{}.length", value.0), true)
                    }
                    _ => {
                        return Err(CoreCError::unsupported(
                            "indexing requires Array, Slice, Buffer, or bytes",
                        ));
                    }
                };
                let index = self.emit_expr(
                    index,
                    Some(&ScalarType::Integer("u64".into())),
                    indent,
                    source,
                )?;
                let access = if buffer {
                    format!(
                        "((const {} *){}.data)[argorix_bounds((uint64_t){}, {})]",
                        element.c_name(),
                        value.0,
                        index.0,
                        length
                    )
                } else {
                    format!(
                        "{}.data[argorix_bounds((uint64_t){}, {})]",
                        value.0, index.0, length
                    )
                };
                self.bind_temp(access, element, indent, source)
            }
            CoreIrExpr::Field { value, name } => {
                let value = self.emit_expr(value, None, indent, source)?;
                let (type_name, access) = match &value.1 {
                    ScalarType::User(type_name) => (type_name, format!("{}.{}", value.0, name)),
                    ScalarType::Handle(element) => {
                        let ScalarType::User(type_name) = element.as_ref() else {
                            return Err(CoreCError::unsupported(
                                "handle field access requires a struct element",
                            ));
                        };
                        (
                            type_name,
                            format!(
                                "((const {} *)argorix_handle_get({}, {}U, sizeof({}), false))->{}",
                                element.c_name(),
                                value.0,
                                core_type_id(element),
                                element.c_name(),
                                name
                            ),
                        )
                    }
                    _ => {
                        return Err(CoreCError::unsupported(
                            "field access currently requires a struct or struct handle",
                        ));
                    }
                };
                let field_ty = self
                    .structs
                    .get(type_name)
                    .and_then(|layout| layout.fields.get(name))
                    .cloned()
                    .ok_or_else(|| {
                        CoreCError::unsupported(format!("unknown field `{name}` on `{type_name}`"))
                    })?;
                self.bind_temp(access, field_ty, indent, source)
            }
            CoreIrExpr::Unary { operator, value } => {
                let value = self.emit_expr(value, expected, indent, source)?;
                let text = match operator {
                    CoreIrUnaryOp::Not => format!("(!{})", value.0),
                    CoreIrUnaryOp::Negate => {
                        return Err(CoreCError::unsupported(
                            "checked signed negation is not implemented yet",
                        ));
                    }
                };
                self.bind_temp(text, value.1, indent, source)
            }
            CoreIrExpr::Binary {
                left,
                operator,
                right,
            } => self.emit_binary(left, *operator, right, expected, indent, source),
            CoreIrExpr::Call { callee, arguments } => {
                if let CoreIrExpr::Path { segments } = callee.as_ref() {
                    if segments == &["Buffer".to_string(), "new".to_string()]
                        && arguments.is_empty()
                    {
                        let ty = expected.cloned().ok_or_else(|| {
                            CoreCError::unsupported("Buffer::new requires an expected Buffer type")
                        })?;
                        let ScalarType::Buffer(element) = &ty else {
                            return Err(CoreCError::unsupported(
                                "Buffer::new has a non-Buffer expected type",
                            ));
                        };
                        return self.bind_temp(
                            format!(
                                "argorix_buffer_new(sizeof({}), ARGORIX_BUFFER_LIMIT_BYTES)",
                                element.c_name()
                            ),
                            ty,
                            indent,
                            source,
                        );
                    }
                    if segments == &["Arena".to_string(), "new".to_string()] && arguments.is_empty()
                    {
                        let ty = expected.cloned().ok_or_else(|| {
                            CoreCError::unsupported("Arena::new requires an expected Arena type")
                        })?;
                        let ScalarType::Arena(element) = &ty else {
                            return Err(CoreCError::unsupported(
                                "Arena::new has a non-Arena expected type",
                            ));
                        };
                        return self.bind_temp(
                            format!(
                                "argorix_arena_new(sizeof({}), {}U, ARGORIX_ARENA_LIMIT_BYTES, ARGORIX_ARENA_SLOT_LIMIT)",
                                element.c_name(),
                                core_type_id(element)
                            ),
                            ty,
                            indent,
                            source,
                        );
                    }
                }
                if let CoreIrExpr::Field { value, name } = callee.as_ref() {
                    if name == "push" {
                        let CoreIrExpr::Path { segments } = value.as_ref() else {
                            return Err(CoreCError::unsupported(
                                "Buffer::push receiver must be a local",
                            ));
                        };
                        let local = single_path(segments)?;
                        let receiver_ty = self.locals.get(local).cloned().ok_or_else(|| {
                            CoreCError::unsupported(format!("unknown local `{local}`"))
                        })?;
                        let ScalarType::Buffer(element) = receiver_ty else {
                            return Err(CoreCError::unsupported(
                                "push is only implemented for Buffer values",
                            ));
                        };
                        let [argument] = arguments.as_slice() else {
                            return Err(CoreCError::unsupported(
                                "Buffer::push requires one argument",
                            ));
                        };
                        let value = self.emit_expr(argument, Some(&element), indent, source)?;
                        let value = self.bind_temp(value.0, (*element).clone(), indent, source)?;
                        line(
                            source,
                            indent,
                            &format!("argorix_buffer_push(&argorix_v_{local}, &{});", value.0),
                        );
                        return Ok(("0".into(), ScalarType::Unit));
                    }
                    if name == "alloc" || name == "release" {
                        let CoreIrExpr::Path { segments } = value.as_ref() else {
                            return Err(CoreCError::unsupported(
                                "Arena method receiver must be a local",
                            ));
                        };
                        let local = single_path(segments)?;
                        let receiver_ty = self.locals.get(local).cloned().ok_or_else(|| {
                            CoreCError::unsupported(format!("unknown local `{local}`"))
                        })?;
                        let ScalarType::Arena(element) = receiver_ty else {
                            return Err(CoreCError::unsupported(
                                "arena method requires an Arena value",
                            ));
                        };
                        if name == "release" {
                            if !arguments.is_empty() {
                                return Err(CoreCError::unsupported(
                                    "Arena::release accepts no arguments",
                                ));
                            }
                            line(
                                source,
                                indent,
                                &format!("argorix_arena_release(&argorix_v_{local});"),
                            );
                            return Ok(("0".into(), ScalarType::Unit));
                        }
                        let [argument] = arguments.as_slice() else {
                            return Err(CoreCError::unsupported(
                                "Arena::alloc requires one argument",
                            ));
                        };
                        let value = self.emit_expr(argument, Some(&element), indent, source)?;
                        let value = self.bind_temp(value.0, (*element).clone(), indent, source)?;
                        return self.bind_temp(
                            format!("argorix_arena_alloc(&argorix_v_{local}, &{})", value.0),
                            ScalarType::Handle(element),
                            indent,
                            source,
                        );
                    }
                    if !arguments.is_empty() {
                        return Err(CoreCError::unsupported(format!(
                            "intrinsic `{name}` does not accept arguments"
                        )));
                    }
                    let receiver = self.emit_expr(value, None, indent, source)?;
                    return match (name.as_str(), &receiver.1) {
                        ("length", ScalarType::Array { length, .. }) => self.bind_temp(
                            format!("{length}U"),
                            ScalarType::Integer("u64".into()),
                            indent,
                            source,
                        ),
                        (
                            "length",
                            ScalarType::Slice(_)
                            | ScalarType::Buffer(_)
                            | ScalarType::Bytes
                            | ScalarType::String,
                        ) => self.bind_temp(
                            format!("{}.length", receiver.0),
                            ScalarType::Integer("u64".into()),
                            indent,
                            source,
                        ),
                        ("as_bytes", ScalarType::Array { element, length })
                            if **element == ScalarType::Integer("u8".into()) =>
                        {
                            self.bind_temp(
                                format!("(argorix_bytes){{{}.data, {}U}}", receiver.0, length),
                                ScalarType::Bytes,
                                indent,
                                source,
                            )
                        }
                        ("decode_utf8_or_trap", ScalarType::Bytes) => self.bind_temp(
                            format!("argorix_decode_utf8({})", receiver.0),
                            ScalarType::String,
                            indent,
                            source,
                        ),
                        _ => Err(CoreCError::unsupported(format!(
                            "intrinsic `{name}` is not defined for {:?}",
                            receiver.1
                        ))),
                    };
                }
                let CoreIrExpr::Path { segments } = callee.as_ref() else {
                    return Err(CoreCError::unsupported(
                        "scalar C profile supports direct function calls only",
                    ));
                };
                let name = single_path(segments)?;
                let signature =
                    self.signatures.get(name).cloned().ok_or_else(|| {
                        CoreCError::unsupported(format!("unknown function `{name}`"))
                    })?;
                let mut emitted = Vec::new();
                for (argument, ty) in arguments.iter().zip(&signature.parameters) {
                    emitted.push(self.emit_expr(argument, Some(ty), indent, source)?.0);
                }
                let call = format!(
                    "argorix_fn_{}(budget{})",
                    name,
                    emitted
                        .iter()
                        .map(|value| format!(", {value}"))
                        .collect::<String>()
                );
                self.bind_temp(call, signature.result, indent, source)
            }
            CoreIrExpr::Match { value, arms } => {
                let result_ty = expected.cloned().ok_or_else(|| {
                    CoreCError::unsupported("match expression needs an expected result type")
                })?;
                let scrutinee = self.emit_expr(value, None, indent, source)?;
                let ScalarType::User(enum_name) = &scrutinee.1 else {
                    return Err(CoreCError::unsupported(
                        "this C profile currently matches enum values only",
                    ));
                };
                let enum_name = enum_name.clone();
                let layout = self.enums.get(&enum_name).cloned().ok_or_else(|| {
                    CoreCError::unsupported(format!("`{enum_name}` is not a lowered enum"))
                })?;
                let result = self.next_temp();
                let matched = self.next_temp();
                line(source, indent, &format!("{} {result};", result_ty.c_name()));
                line(source, indent, &format!("bool {matched} = false;"));
                for arm in arms {
                    if arm.guard.is_some() {
                        return Err(CoreCError::unsupported(
                            "match guards are not implemented in the C profile yet",
                        ));
                    }
                    let (condition, variant, fields) = match &arm.pattern {
                        CoreIrPattern::Variant { path, fields } => {
                            let [pattern_enum, variant] = path.as_slice() else {
                                return Err(CoreCError::unsupported(
                                    "variant match path must contain enum and variant",
                                ));
                            };
                            if pattern_enum != &enum_name {
                                return Err(CoreCError::unsupported(
                                    "variant pattern enum does not match scrutinee",
                                ));
                            }
                            if !layout.variants.contains_key(variant) {
                                return Err(CoreCError::unsupported(format!(
                                    "unknown variant `{enum_name}::{variant}`"
                                )));
                            }
                            (
                                format!(
                                    "{}.tag == argorix_tag_{}_{}",
                                    scrutinee.0, enum_name, variant
                                ),
                                Some(variant.as_str()),
                                fields.as_slice(),
                            )
                        }
                        CoreIrPattern::Wildcard => ("true".into(), None, &[][..]),
                        _ => {
                            return Err(CoreCError::unsupported(
                                "this C profile supports variant and wildcard match arms",
                            ));
                        }
                    };
                    line(
                        source,
                        indent,
                        &format!("if (!{matched} && ({condition})) {{"),
                    );
                    let mut previous = Vec::new();
                    if let Some(variant) = variant {
                        let variant_layout = &layout.variants[variant];
                        for field in fields {
                            let field_ty =
                                variant_layout.get(&field.name).cloned().ok_or_else(|| {
                                    CoreCError::unsupported(format!(
                                        "unknown pattern field `{}`",
                                        field.name
                                    ))
                                })?;
                            let binding = match field.nested.as_deref() {
                                None => Some(field.name.as_str()),
                                Some(CoreIrPattern::Binding { name }) => Some(name.as_str()),
                                Some(CoreIrPattern::Wildcard) => None,
                                Some(_) => {
                                    return Err(CoreCError::unsupported(
                                        "nested non-binding patterns are not implemented yet",
                                    ));
                                }
                            };
                            if let Some(binding) = binding {
                                previous.push((
                                    binding.to_string(),
                                    self.locals.insert(binding.to_string(), field_ty.clone()),
                                ));
                                line(
                                    source,
                                    indent + 1,
                                    &format!(
                                        "{} argorix_v_{} = {}.data.{}.{};",
                                        field_ty.c_name(),
                                        binding,
                                        scrutinee.0,
                                        variant,
                                        field.name
                                    ),
                                );
                            }
                        }
                    }
                    let value = self.emit_expr(&arm.value, Some(&result_ty), indent + 1, source)?;
                    line(source, indent + 1, &format!("{result} = {};", value.0));
                    line(source, indent + 1, &format!("{matched} = true;"));
                    line(source, indent, "}");
                    for (name, old) in previous {
                        if let Some(old) = old {
                            self.locals.insert(name, old);
                        } else {
                            self.locals.remove(&name);
                        }
                    }
                }
                line(
                    source,
                    indent,
                    &format!("if (!{matched}) {{ argorix_trap(\"NON_EXHAUSTIVE_MATCH\"); }}"),
                );
                Ok((result, result_ty))
            }
            CoreIrExpr::If {
                condition,
                then_block,
                else_expr,
            } => {
                let ty = expected.cloned().ok_or_else(|| {
                    CoreCError::unsupported("if expression needs an expected scalar type")
                })?;
                let condition =
                    self.emit_expr(condition, Some(&ScalarType::Bool), indent, source)?;
                let temp = self.next_temp();
                line(source, indent, &format!("{} {temp};", ty.c_name()));
                line(source, indent, &format!("if ({}) {{", condition.0));
                self.emit_block_assignment(then_block, &temp, &ty, indent + 1, source)?;
                let else_expr = else_expr
                    .as_ref()
                    .ok_or_else(|| CoreCError::unsupported("value if expression requires else"))?;
                line(source, indent, "} else {");
                let other = self.emit_expr(else_expr, Some(&ty), indent + 1, source)?;
                line(source, indent + 1, &format!("{temp} = {};", other.0));
                line(source, indent, "}");
                Ok((temp, ty))
            }
            CoreIrExpr::Block { body } => {
                let ty = expected.cloned().ok_or_else(|| {
                    CoreCError::unsupported("value block needs an expected scalar type")
                })?;
                let temp = self.next_temp();
                line(source, indent, &format!("{} {temp};", ty.c_name()));
                self.emit_block_assignment(body, &temp, &ty, indent, source)?;
                Ok((temp, ty))
            }
            CoreIrExpr::Loop { body } => {
                line(source, indent, "while (true) {");
                line(source, indent + 1, "argorix_step(budget);");
                self.emit_block(body, indent + 1, None, source)?;
                line(source, indent, "}");
                let ty = expected.cloned().unwrap_or(ScalarType::Unit);
                Ok(("0".into(), ty))
            }
            CoreIrExpr::Unit => Ok(("0".into(), ScalarType::Unit)),
            _ => Err(CoreCError::unsupported(format!(
                "expression `{expression:?}` is outside the scalar C profile"
            ))),
        }
    }

    fn emit_binary(
        &mut self,
        left: &CoreIrExpr,
        operator: CoreIrBinaryOp,
        right: &CoreIrExpr,
        expected: Option<&ScalarType>,
        indent: usize,
        source: &mut String,
    ) -> Result<(String, ScalarType), CoreCError> {
        let left = self.emit_expr(left, expected, indent, source)?;
        if matches!(operator, CoreIrBinaryOp::And | CoreIrBinaryOp::Or) {
            let temp = self.next_temp();
            line(source, indent, &format!("bool {temp} = {};", left.0));
            let condition = if operator == CoreIrBinaryOp::And {
                temp.clone()
            } else {
                format!("!{temp}")
            };
            line(source, indent, &format!("if ({condition}) {{"));
            let right = self.emit_expr(right, Some(&ScalarType::Bool), indent + 1, source)?;
            line(source, indent + 1, &format!("{temp} = {};", right.0));
            line(source, indent, "}");
            return Ok((temp, ScalarType::Bool));
        }
        let right = self.emit_expr(right, Some(&left.1), indent, source)?;
        let (text, result) = match operator {
            CoreIrBinaryOp::Add
            | CoreIrBinaryOp::Subtract
            | CoreIrBinaryOp::Multiply
            | CoreIrBinaryOp::Divide
            | CoreIrBinaryOp::Remainder => (
                format!(
                    "argorix_{}_{}({}, {})",
                    left.1.helper_suffix()?,
                    binary_operation(operator),
                    left.0,
                    right.0
                ),
                left.1,
            ),
            CoreIrBinaryOp::Equal
            | CoreIrBinaryOp::NotEqual
            | CoreIrBinaryOp::Less
            | CoreIrBinaryOp::LessEqual
            | CoreIrBinaryOp::Greater
            | CoreIrBinaryOp::GreaterEqual => (
                format!("({} {} {})", left.0, comparison(operator), right.0),
                ScalarType::Bool,
            ),
            CoreIrBinaryOp::BitOr | CoreIrBinaryOp::BitXor | CoreIrBinaryOp::BitAnd => (
                format!("({} {} {})", left.0, bit_operator(operator), right.0),
                left.1,
            ),
            CoreIrBinaryOp::ShiftLeft | CoreIrBinaryOp::ShiftRight => {
                return Err(CoreCError::unsupported(
                    "checked shifts are not implemented yet",
                ));
            }
            CoreIrBinaryOp::And | CoreIrBinaryOp::Or => unreachable!(),
        };
        self.bind_temp(text, result, indent, source)
    }

    fn emit_block_assignment(
        &mut self,
        block: &CoreIrBlock,
        target: &str,
        ty: &ScalarType,
        indent: usize,
        source: &mut String,
    ) -> Result<(), CoreCError> {
        for statement in &block.statements {
            self.emit_statement(statement, indent, source)?;
        }
        let tail = block
            .tail
            .as_ref()
            .ok_or_else(|| CoreCError::unsupported("value block has no tail expression"))?;
        let value = self.emit_expr(tail, Some(ty), indent, source)?;
        line(source, indent, &format!("{target} = {};", value.0));
        Ok(())
    }

    fn bind_temp(
        &mut self,
        value: String,
        ty: ScalarType,
        indent: usize,
        source: &mut String,
    ) -> Result<(String, ScalarType), CoreCError> {
        if ty == ScalarType::Unit {
            line(source, indent, &format!("{value};"));
            return Ok(("0".into(), ty));
        }
        let temp = self.next_temp();
        line(
            source,
            indent,
            &format!("{} {temp} = {value};", ty.c_name()),
        );
        Ok((temp, ty))
    }

    fn next_temp(&mut self) -> String {
        let value = format!("argorix_t_{}", self.temporary);
        self.temporary += 1;
        value
    }
}

fn line(source: &mut String, indent: usize, value: &str) {
    let _ = writeln!(source, "{}{value}", "    ".repeat(indent));
}

fn collect_array_type(value: &ScalarType, arrays: &mut BTreeMap<String, ScalarType>) {
    match value {
        ScalarType::Array { element, .. }
        | ScalarType::Slice(element)
        | ScalarType::Buffer(element)
        | ScalarType::Arena(element)
        | ScalarType::Handle(element) => {
            collect_array_type(element, arrays);
            if matches!(value, ScalarType::Array { .. } | ScalarType::Slice(_)) {
                arrays.insert(value.c_name(), value.clone());
            }
        }
        _ => {}
    }
}

fn struct_layout(value: &CoreIrStruct) -> Result<StructLayout, CoreCError> {
    let fields = value
        .fields
        .iter()
        .map(|field| Ok((field.name.clone(), ScalarType::from_ir(&field.ty)?)))
        .collect::<Result<BTreeMap<_, _>, CoreCError>>()?;
    Ok(StructLayout { fields })
}

fn enum_layout(value: &CoreIrEnum) -> Result<EnumLayout, CoreCError> {
    let variants = value
        .variants
        .iter()
        .map(|variant| {
            let fields = variant
                .fields
                .iter()
                .map(|field| Ok((field.name.clone(), ScalarType::from_ir(&field.ty)?)))
                .collect::<Result<BTreeMap<_, _>, CoreCError>>()?;
            Ok((variant.name.clone(), fields))
        })
        .collect::<Result<BTreeMap<_, _>, CoreCError>>()?;
    Ok(EnumLayout { variants })
}

fn collect_block_array_types(
    block: &CoreIrBlock,
    arrays: &mut BTreeMap<String, ScalarType>,
) -> Result<(), CoreCError> {
    for statement in &block.statements {
        match statement {
            CoreIrStatement::Let {
                annotation: Some(annotation),
                ..
            } => collect_array_type(&ScalarType::from_ir(annotation)?, arrays),
            CoreIrStatement::While { body, .. } => collect_block_array_types(body, arrays)?,
            _ => {}
        }
    }
    Ok(())
}

fn single_path(segments: &[String]) -> Result<&str, CoreCError> {
    if let [name] = segments {
        Ok(name)
    } else {
        Err(CoreCError::unsupported(format!(
            "qualified path `{}` is outside the scalar C profile",
            segments.join("::")
        )))
    }
}

fn integer_literal_suffix(ty: &ScalarType) -> Result<&'static str, CoreCError> {
    match ty {
        ScalarType::Integer(name) if name.starts_with('u') => Ok("U"),
        ScalarType::Integer(_) => Ok(""),
        _ => Err(CoreCError::unsupported(
            "integer literal has non-integer type",
        )),
    }
}

fn core_type_id(ty: &ScalarType) -> u32 {
    let mut hash = 2_166_136_261_u32;
    for byte in ty.mangle().bytes() {
        hash ^= u32::from(byte);
        hash = hash.wrapping_mul(16_777_619);
    }
    if hash == 0 {
        1
    } else {
        hash
    }
}

fn binary_operation(value: CoreIrBinaryOp) -> &'static str {
    match value {
        CoreIrBinaryOp::Add => "add",
        CoreIrBinaryOp::Subtract => "sub",
        CoreIrBinaryOp::Multiply => "mul",
        CoreIrBinaryOp::Divide => "div",
        CoreIrBinaryOp::Remainder => "rem",
        _ => unreachable!(),
    }
}

fn assign_operation(value: CoreIrAssignOp) -> &'static str {
    match value {
        CoreIrAssignOp::Add => "add",
        CoreIrAssignOp::Subtract => "sub",
        CoreIrAssignOp::Multiply => "mul",
        CoreIrAssignOp::Divide => "div",
        CoreIrAssignOp::Remainder => "rem",
        CoreIrAssignOp::Assign => unreachable!(),
    }
}

fn comparison(value: CoreIrBinaryOp) -> &'static str {
    match value {
        CoreIrBinaryOp::Equal => "==",
        CoreIrBinaryOp::NotEqual => "!=",
        CoreIrBinaryOp::Less => "<",
        CoreIrBinaryOp::LessEqual => "<=",
        CoreIrBinaryOp::Greater => ">",
        CoreIrBinaryOp::GreaterEqual => ">=",
        _ => unreachable!(),
    }
}

fn bit_operator(value: CoreIrBinaryOp) -> &'static str {
    match value {
        CoreIrBinaryOp::BitOr => "|",
        CoreIrBinaryOp::BitXor => "^",
        CoreIrBinaryOp::BitAnd => "&",
        _ => unreachable!(),
    }
}
