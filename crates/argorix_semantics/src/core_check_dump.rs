//! The diagnostics dump of the Core checker, which the Argorix checker
//! (`compiler/check.argx`) must reproduce (ESP-012).
//!
//! One line per diagnostic, `line:column: phase[Code]`, in the order the
//! checker reports them. A program that does not parse, or that checks cleanly,
//! gives the one line `parse failed` or `ok`. The module is checked on its own:
//! its locked set holds only itself, so an import is `ImportNotLocked`.

use crate::core::{check_core_program, CoreCheckOptions};
use argorix_parser::core::parse_core_source;
use std::collections::BTreeSet;

pub fn core_check_dump(source: &[u8]) -> String {
    let Ok(text) = std::str::from_utf8(source) else {
        return "parse failed\n".into();
    };
    let Ok(program) = parse_core_source(text) else {
        return "parse failed\n".into();
    };
    let options = CoreCheckOptions {
        available_modules: BTreeSet::from([program.module.value.clone()]),
    };
    match check_core_program(&program, &options) {
        Ok(()) => "ok\n".into(),
        Err(diagnostics) => diagnostics
            .iter()
            .map(|diagnostic| {
                format!(
                    "{}:{}: {}[{}]\n",
                    diagnostic.span.line, diagnostic.span.column, diagnostic.phase, diagnostic.code
                )
            })
            .collect(),
    }
}
