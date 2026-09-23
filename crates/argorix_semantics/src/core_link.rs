//! Linking of Core modules for the transitional stage0 frontend.
//!
//! `spec/core/modules.md`: every file declares one module, symbols are private
//! unless `pub`, imports form a deterministic graph, and duplicates, missing
//! imports, private symbols and cycles are errors. The checker, the IR and the
//! C backend all work on one program, so this module turns a module graph into
//! one: it resolves what each module names through its imports, gives every
//! item outside the root a module-qualified name, and concatenates the result
//! with dependencies first.
//!
//! What a module can name through an import, which is the only reading the
//! grammar allows:
//!
//! - `import a.b;` binds `b`, and `import a.b as x;` binds `x`.
//! - `b.f(..)` calls the public function `f` of `a.b`, and `b.C` reads its
//!   public constant `C`. A local named `b` shadows the binding.
//! - A public struct or enum of a directly imported module is named by its
//!   bare name, because a type in Core 0.1 is a single identifier. The name
//!   must not clash with the importer's own items or another import's.
//!
//! Only direct imports are visible; nothing is re-exported. Cycles are
//! rejected outright: the spec allows cycles of signatures through handles
//! after joint resolution, which stage0 does not implement.

use crate::core::{check_core_program, CoreCheckOptions};
use argorix_parser::core::*;
use std::collections::{BTreeMap, BTreeSet};

/// A linking failure, attributed to the module whose source contains it so
/// the driver can render it against the right file.
#[derive(Debug, Clone)]
pub struct CoreLinkError {
    pub module: String,
    pub diagnostics: Vec<CoreDiagnostic>,
}

impl CoreLinkError {
    fn single(module: &str, code: &str, message: String, span: argorix_parser::span::Span) -> Self {
        Self {
            module: module.to_string(),
            diagnostics: vec![CoreDiagnostic::new(
                CorePhase::Resolution,
                code,
                message,
                span,
            )],
        }
    }
}

/// The modules `root` reaches through its imports, dependencies first and
/// `root` last. `modules` is the locked compilation set; `duplicates` names
/// the modules declared by more than one file, which the spec forbids.
pub fn core_link_order(
    root: &CoreProgram,
    modules: &BTreeMap<String, CoreProgram>,
    duplicates: &BTreeSet<String>,
) -> Result<Vec<String>, CoreLinkError> {
    let mut state = BTreeMap::new();
    let mut order = Vec::new();
    visit(root, modules, duplicates, &mut state, &mut order)?;
    Ok(order)
}

/// Link `root` with every module it reaches into one program. Items of the
/// root keep their names; every other item is renamed `a__b__name` for module
/// `a.b`, so private helpers of two modules never collide.
pub fn link_core_program(
    root: &CoreProgram,
    modules: &BTreeMap<String, CoreProgram>,
    duplicates: &BTreeSet<String>,
) -> Result<CoreProgram, CoreLinkError> {
    let order = core_link_order(root, modules, duplicates)?;
    let root_name = root.module.value.clone();
    let program_of = |name: &str| -> &CoreProgram {
        if name == root_name {
            root
        } else {
            &modules[name]
        }
    };
    let names: BTreeMap<String, ModuleNames> = order
        .iter()
        .map(|name| {
            (
                name.clone(),
                ModuleNames::of(program_of(name), name == &root_name),
            )
        })
        .collect();

    let mut items = Vec::new();
    for name in &order {
        let program = program_of(name);
        let mut linker = Linker::new(program, &names)?;
        let mut rewritten = program.items.clone();
        for item in &mut rewritten {
            linker.item(item);
        }
        if !linker.errors.is_empty() {
            return Err(CoreLinkError {
                module: name.clone(),
                diagnostics: linker.errors,
            });
        }
        items.extend(rewritten);
    }
    Ok(CoreProgram {
        version: root.version.clone(),
        module: root.module.clone(),
        // Every import is resolved into `items`; none is left to check.
        imports: Vec::new(),
        items,
        span: root.span,
    })
}

/// Check a package as the driver does: every module `root` reaches is linked
/// and checked as a root of its own, dependencies first, and the root last.
/// A failure names the module whose source contains it, the first in link
/// order that fails. Returns the link order and the linked root.
pub fn check_core_package(
    root: &CoreProgram,
    modules: &BTreeMap<String, CoreProgram>,
    duplicates: &BTreeSet<String>,
    options: &CoreCheckOptions,
) -> Result<(Vec<String>, CoreProgram), CoreLinkError> {
    let order = core_link_order(root, modules, duplicates)?;
    let root_name = &root.module.value;
    for name in order.iter().filter(|name| *name != root_name) {
        let linked = link_core_program(&modules[name], modules, duplicates)?;
        check_core_program(&linked, options).map_err(|diagnostics| CoreLinkError {
            module: name.clone(),
            diagnostics,
        })?;
    }
    let linked = link_core_program(root, modules, duplicates)?;
    check_core_program(&linked, options).map_err(|diagnostics| CoreLinkError {
        module: root_name.clone(),
        diagnostics,
    })?;
    Ok((order, linked))
}

fn visit(
    program: &CoreProgram,
    modules: &BTreeMap<String, CoreProgram>,
    duplicates: &BTreeSet<String>,
    state: &mut BTreeMap<String, bool>,
    order: &mut Vec<String>,
) -> Result<(), CoreLinkError> {
    let name = program.module.value.clone();
    // `false` while the module is on the current path, `true` once done.
    state.insert(name.clone(), false);
    for import in &program.imports {
        let target = &import.path.value;
        if duplicates.contains(target) {
            return Err(CoreLinkError::single(
                &name,
                "DuplicateModule",
                format!("module `{target}` is declared by more than one file"),
                import.path.span,
            ));
        }
        // The path being walked comes first: the root itself is not in
        // `modules`, so a module importing it has to be seen as a cycle
        // before it could be reported as missing.
        match state.get(target) {
            Some(false) => {
                return Err(CoreLinkError::single(
                    &name,
                    "ImportCycle",
                    format!(
                        "importing `{target}` closes an import cycle, which stage0 does not accept"
                    ),
                    import.path.span,
                ));
            }
            Some(true) => continue,
            None => {}
        }
        let Some(target_program) = modules.get(target) else {
            return Err(CoreLinkError::single(
                &name,
                "ImportNotLocked",
                format!("module `{target}` is not present in the locked compilation set"),
                import.path.span,
            ));
        };
        visit(target_program, modules, duplicates, state, order)?;
    }
    state.insert(name.clone(), true);
    order.push(name);
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ItemKind {
    Function,
    Struct,
    Enum,
    Const,
}

/// What a module declares: each item's kind, linked name and visibility.
#[derive(Debug, Clone, Default)]
struct ModuleNames {
    linked: BTreeMap<String, String>,
    kinds: BTreeMap<String, ItemKind>,
    public: BTreeSet<String>,
}

impl ModuleNames {
    fn of(program: &CoreProgram, is_root: bool) -> Self {
        let prefix = program.module.value.replace('.', "__");
        let mut names = Self::default();
        for item in &program.items {
            let (name, kind) = match &item.kind {
                CoreItemKind::Function(value) => (&value.name.value, ItemKind::Function),
                CoreItemKind::Struct(value) => (&value.name.value, ItemKind::Struct),
                CoreItemKind::Enum(value) => (&value.name.value, ItemKind::Enum),
                CoreItemKind::Const(value) => (&value.name.value, ItemKind::Const),
            };
            let linked = if is_root {
                name.clone()
            } else {
                format!("{prefix}__{name}")
            };
            names.linked.insert(name.clone(), linked);
            names.kinds.insert(name.clone(), kind);
            if item.public {
                names.public.insert(name.clone());
            }
        }
        names
    }

    fn is_type(&self, name: &str) -> bool {
        matches!(
            self.kinds.get(name),
            Some(ItemKind::Struct | ItemKind::Enum)
        )
    }

    fn is_value(&self, name: &str) -> bool {
        matches!(
            self.kinds.get(name),
            Some(ItemKind::Function | ItemKind::Const)
        )
    }
}

struct Linker<'a> {
    module: String,
    own: &'a ModuleNames,
    all: &'a BTreeMap<String, ModuleNames>,
    /// Import binding -> imported module.
    bindings: BTreeMap<String, String>,
    /// Public type of a direct import, by bare name -> linked name.
    imported_types: BTreeMap<String, String>,
    locals: Vec<BTreeSet<String>>,
    errors: Vec<CoreDiagnostic>,
}

impl<'a> Linker<'a> {
    fn new(
        program: &CoreProgram,
        all: &'a BTreeMap<String, ModuleNames>,
    ) -> Result<Self, CoreLinkError> {
        let module = program.module.value.clone();
        let own = &all[&module];
        let mut bindings = BTreeMap::new();
        let mut imported_types: BTreeMap<String, String> = BTreeMap::new();
        for import in &program.imports {
            let target = import.path.value.clone();
            let binding = import
                .alias
                .as_ref()
                .map(|alias| alias.value.clone())
                .unwrap_or_else(|| target.rsplit('.').next().unwrap_or(&target).to_string());
            if own.kinds.contains_key(&binding) || bindings.contains_key(&binding) {
                return Err(CoreLinkError::single(
                    &module,
                    "DuplicateImportedName",
                    format!("import binding `{binding}` is already a name in this module"),
                    import.span,
                ));
            }
            let exports = &all[&target];
            for name in &exports.public {
                if !exports.is_type(name) {
                    continue;
                }
                if own.kinds.contains_key(name) || imported_types.contains_key(name) {
                    return Err(CoreLinkError::single(
                        &module,
                        "DuplicateImportedName",
                        format!(
                            "public type `{name}` of `{target}` clashes with a name already in scope"
                        ),
                        import.span,
                    ));
                }
                imported_types.insert(name.clone(), exports.linked[name].clone());
            }
            bindings.insert(binding, target);
        }
        Ok(Self {
            module,
            own,
            all,
            bindings,
            imported_types,
            locals: Vec::new(),
            errors: Vec::new(),
        })
    }

    fn is_local(&self, name: &str) -> bool {
        self.locals.iter().any(|scope| scope.contains(name))
    }

    fn declare(&mut self, name: &str) {
        if let Some(scope) = self.locals.last_mut() {
            scope.insert(name.to_string());
        }
    }

    fn resolve_type(&self, name: &str) -> Option<String> {
        if self.own.is_type(name) {
            return Some(self.own.linked[name].clone());
        }
        self.imported_types.get(name).cloned()
    }

    fn item(&mut self, item: &mut CoreItem) {
        match &mut item.kind {
            CoreItemKind::Function(function) => {
                function.name.value = self.own.linked[&function.name.value].clone();
                self.locals.push(BTreeSet::new());
                for parameter in &mut function.parameters {
                    self.ty(&mut parameter.ty);
                    let name = parameter.name.value.clone();
                    self.declare(&name);
                }
                self.ty(&mut function.return_type);
                self.block(&mut function.body);
                self.locals.pop();
            }
            CoreItemKind::Struct(value) => {
                value.name.value = self.own.linked[&value.name.value].clone();
                for field in &mut value.fields {
                    self.ty(&mut field.ty);
                }
            }
            CoreItemKind::Enum(value) => {
                value.name.value = self.own.linked[&value.name.value].clone();
                for variant in &mut value.variants {
                    for field in &mut variant.fields {
                        self.ty(&mut field.ty);
                    }
                }
            }
            CoreItemKind::Const(value) => {
                value.name.value = self.own.linked[&value.name.value].clone();
                self.ty(&mut value.ty);
                self.expr(&mut value.value);
            }
        }
    }

    fn ty(&mut self, ty: &mut CoreType) {
        match &mut ty.kind {
            CoreTypeKind::Named(name) => {
                if let Some(linked) = self.resolve_type(name) {
                    *name = linked;
                }
            }
            CoreTypeKind::Container { element, .. } => self.ty(element),
        }
    }

    fn block(&mut self, block: &mut CoreBlock) {
        self.locals.push(BTreeSet::new());
        for statement in &mut block.statements {
            self.statement(statement);
        }
        if let Some(tail) = &mut block.tail {
            self.expr(tail);
        }
        self.locals.pop();
    }

    fn statement(&mut self, statement: &mut CoreStatement) {
        match &mut statement.kind {
            CoreStatementKind::Let {
                name,
                annotation,
                value,
                ..
            } => {
                // The initializer is resolved before the new local exists, so
                // `let b = b.f(..)` still reaches the import.
                self.expr(value);
                if let Some(annotation) = annotation {
                    self.ty(annotation);
                }
                let name = name.value.clone();
                self.declare(&name);
            }
            CoreStatementKind::Assign { target, value, .. } => {
                self.expr(target);
                self.expr(value);
            }
            CoreStatementKind::While { condition, body } => {
                self.expr(condition);
                self.block(body);
            }
            CoreStatementKind::Break(value) | CoreStatementKind::Return(value) => {
                if let Some(value) = value {
                    self.expr(value);
                }
            }
            CoreStatementKind::Continue => {}
            CoreStatementKind::Expr(value) => self.expr(value),
        }
    }

    fn expr(&mut self, expression: &mut CoreExpr) {
        // A field read on an import binding names an item of that module.
        if let CoreExprKind::Field { value, name } = &expression.kind {
            if let CoreExprKind::Path(segments) = &value.kind {
                if let [binding] = segments.as_slice() {
                    if !self.is_local(binding) {
                        if let Some(target) = self.bindings.get(binding).cloned() {
                            let item = name.value.clone();
                            let span = name.span;
                            if let Some(linked) = self.imported_value(&target, &item, span) {
                                expression.kind = CoreExprKind::Path(vec![linked]);
                            }
                            return;
                        }
                    }
                }
            }
        }
        match &mut expression.kind {
            CoreExprKind::Integer { .. }
            | CoreExprKind::String(_)
            | CoreExprKind::Bool(_)
            | CoreExprKind::Unit => {}
            CoreExprKind::Path(segments) => match segments.as_mut_slice() {
                [name] => {
                    if !self.is_local(name) && self.own.is_value(name) {
                        *name = self.own.linked[name.as_str()].clone();
                    }
                }
                [type_name, _variant] => {
                    if let Some(linked) = self.resolve_type(type_name) {
                        *type_name = linked;
                    }
                }
                _ => {}
            },
            CoreExprKind::Aggregate { path, fields } => {
                if let Some(first) = path.first_mut() {
                    if let Some(linked) = self.resolve_type(first) {
                        *first = linked;
                    }
                }
                for (_, value) in fields {
                    self.expr(value);
                }
            }
            CoreExprKind::Array(values) => {
                for value in values {
                    self.expr(value);
                }
            }
            CoreExprKind::Block(block) | CoreExprKind::Loop(block) => self.block(block),
            CoreExprKind::If {
                condition,
                then_block,
                else_expr,
            } => {
                self.expr(condition);
                self.block(then_block);
                if let Some(other) = else_expr {
                    self.expr(other);
                }
            }
            CoreExprKind::Match { value, arms } => {
                self.expr(value);
                for arm in arms {
                    self.locals.push(BTreeSet::new());
                    self.pattern(&mut arm.pattern);
                    if let Some(guard) = &mut arm.guard {
                        self.expr(guard);
                    }
                    self.expr(&mut arm.value);
                    self.locals.pop();
                }
            }
            CoreExprKind::Call { callee, arguments } => {
                self.expr(callee);
                for argument in arguments {
                    self.expr(argument);
                }
            }
            CoreExprKind::Index { value, index } => {
                self.expr(value);
                self.expr(index);
            }
            CoreExprKind::Field { value, .. } => self.expr(value),
            CoreExprKind::Unary { value, .. } => self.expr(value),
            CoreExprKind::Binary { left, right, .. } => {
                self.expr(left);
                self.expr(right);
            }
        }
    }

    /// The linked name of public function or constant `item` of `target`.
    fn imported_value(
        &mut self,
        target: &str,
        item: &str,
        span: argorix_parser::span::Span,
    ) -> Option<String> {
        let exports = &self.all[target];
        if !exports.is_value(item) {
            self.errors.push(CoreDiagnostic::new(
                CorePhase::Resolution,
                "UnknownImportedSymbol",
                format!("module `{target}` has no function or constant `{item}`"),
                span,
            ));
            return None;
        }
        if !exports.public.contains(item) {
            self.errors.push(CoreDiagnostic::new(
                CorePhase::Resolution,
                "PrivateSymbol",
                format!(
                    "`{item}` is private to `{target}`; only `pub` items can be used from `{}`",
                    self.module
                ),
                span,
            ));
            return None;
        }
        Some(exports.linked[item].clone())
    }

    fn pattern(&mut self, pattern: &mut CorePattern) {
        match &mut pattern.kind {
            CorePatternKind::Wildcard | CorePatternKind::Bool(_) | CorePatternKind::Integer(_) => {}
            CorePatternKind::Binding(name) => {
                let name = name.clone();
                self.declare(&name);
            }
            CorePatternKind::Variant { path, fields } => {
                if path.len() == 2 {
                    if let Some(linked) = self.resolve_type(&path[0]) {
                        path[0] = linked;
                    }
                }
                for (field, nested) in fields {
                    match nested {
                        Some(nested) => self.pattern(nested),
                        // `E::A { p0 }` binds the field's own name.
                        None => {
                            let name = field.value.clone();
                            self.declare(&name);
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(source: &str) -> CoreProgram {
        parse_core_source(source).expect("test source parses")
    }

    fn package(sources: &[&str]) -> (CoreProgram, BTreeMap<String, CoreProgram>) {
        let mut programs: Vec<CoreProgram> = sources.iter().map(|source| parse(source)).collect();
        let root = programs.remove(0);
        let modules = programs
            .into_iter()
            .map(|program| (program.module.value.clone(), program))
            .collect();
        (root, modules)
    }

    fn link(sources: &[&str]) -> Result<CoreProgram, CoreLinkError> {
        let (root, modules) = package(sources);
        link_core_program(&root, &modules, &BTreeSet::new())
    }

    fn codes(error: CoreLinkError) -> Vec<String> {
        error
            .diagnostics
            .into_iter()
            .map(|diagnostic| diagnostic.code)
            .collect()
    }

    const HELPER: &str = "core 0.1;\nmodule demo.helper;\n\
pub const SCALE: u32 = 20u32;\n\
pub fn twice(value: u32) -> u32 { bump(value) + value - 2u32 }\n\
fn bump(value: u32) -> u32 { value + 2u32 }\n\
pub struct Pair { left: u32, right: u32, }\n";

    #[test]
    fn a_public_function_and_constant_are_reached_through_the_binding() {
        let linked = link(&[
            "core 0.1;\nmodule demo.main;\nimport demo.helper;\n\
fn bump(value: u32) -> u32 { value }\n\
pub fn argorix_main() -> u32 { helper.twice(helper.SCALE) + bump(2u32) }\n",
            HELPER,
        ])
        .expect("links");
        // Both `bump`s survive: the helper's private one is renamed.
        let names: Vec<String> = linked
            .items
            .iter()
            .filter_map(|item| match &item.kind {
                CoreItemKind::Function(function) => Some(function.name.value.clone()),
                _ => None,
            })
            .collect();
        assert!(names.contains(&"bump".to_string()));
        assert!(names.contains(&"demo__helper__bump".to_string()));
        assert!(names.contains(&"demo__helper__twice".to_string()));
        assert!(linked.imports.is_empty());
        check_core_program(&linked, &CoreCheckOptions::default()).expect("linked program checks");
    }

    #[test]
    fn a_public_type_is_named_by_its_bare_name() {
        let linked = link(&[
            "core 0.1;\nmodule demo.main;\nimport demo.helper as h;\n\
pub fn argorix_main() -> u32 { let pair: Pair = Pair { left: 40u32, right: 2u32 }; pair.left + pair.right }\n",
            HELPER,
        ])
        .expect("links");
        check_core_program(&linked, &CoreCheckOptions::default()).expect("linked program checks");
    }

    #[test]
    fn a_private_function_is_refused() {
        let error = link(&[
            "core 0.1;\nmodule demo.main;\nimport demo.helper;\n\
pub fn argorix_main() -> u32 { helper.bump(1u32) }\n",
            HELPER,
        ])
        .expect_err("private");
        assert_eq!(error.module, "demo.main");
        assert_eq!(codes(error), ["PrivateSymbol"]);
    }

    #[test]
    fn a_missing_symbol_is_refused() {
        let error = link(&[
            "core 0.1;\nmodule demo.main;\nimport demo.helper;\n\
pub fn argorix_main() -> u32 { helper.nothing(1u32) }\n",
            HELPER,
        ])
        .expect_err("missing");
        assert_eq!(codes(error), ["UnknownImportedSymbol"]);
    }

    #[test]
    fn a_local_shadows_the_import_binding() {
        // `helper` is a local struct value here, so `helper.left` is a field.
        let linked = link(&[
            "core 0.1;\nmodule demo.main;\nimport demo.helper;\n\
pub fn argorix_main() -> u32 { let helper: Pair = Pair { left: 42u32, right: 0u32 }; helper.left }\n",
            HELPER,
        ])
        .expect("links");
        check_core_program(&linked, &CoreCheckOptions::default()).expect("linked program checks");
    }

    #[test]
    fn an_import_cycle_is_refused() {
        let error = link(&[
            "core 0.1;\nmodule demo.a;\nimport demo.b;\npub fn one() -> u32 { 1u32 }\n",
            "core 0.1;\nmodule demo.b;\nimport demo.a;\npub fn two() -> u32 { 2u32 }\n",
        ])
        .expect_err("cycle");
        assert_eq!(error.module, "demo.b");
        assert_eq!(codes(error), ["ImportCycle"]);
    }

    #[test]
    fn a_missing_module_and_a_duplicate_module_are_refused() {
        let (root, modules) = package(&[
            "core 0.1;\nmodule demo.main;\nimport demo.helper;\npub fn argorix_main() -> u32 { 0u32 }\n",
        ]);
        let error = link_core_program(&root, &modules, &BTreeSet::new()).expect_err("missing");
        assert_eq!(codes(error), ["ImportNotLocked"]);
        let duplicates = BTreeSet::from(["demo.helper".to_string()]);
        let error = link_core_program(&root, &modules, &duplicates).expect_err("duplicate");
        assert_eq!(codes(error), ["DuplicateModule"]);
    }

    #[test]
    fn a_binding_or_type_that_clashes_is_refused() {
        let error = link(&[
            "core 0.1;\nmodule demo.main;\nimport demo.helper;\n\
struct Pair { left: u32, }\npub fn argorix_main() -> u32 { 0u32 }\n",
            HELPER,
        ])
        .expect_err("clash");
        assert_eq!(codes(error), ["DuplicateImportedName"]);
    }
}
