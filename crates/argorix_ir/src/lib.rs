pub mod core;
pub mod core_c;
pub mod ir;

pub use core::{
    lower_core_program, verify_core_ir, CoreIrBackend, CoreIrDiagnostic, CoreIrEffect,
    CoreIrProgram, VerifiedCoreIr,
};
pub use core_c::{CoreCBackend, CoreCError, CoreCOutput};
pub use ir::{IrModule, IrModuleImport, IrProgram, IrProviderContract};
