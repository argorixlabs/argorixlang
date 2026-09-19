pub mod core;
pub mod ir;

pub use core::{
    lower_core_program, verify_core_ir, CoreIrBackend, CoreIrDiagnostic, CoreIrEffect,
    CoreIrProgram, VerifiedCoreIr,
};
pub use ir::{IrModule, IrModuleImport, IrProgram, IrProviderContract};
