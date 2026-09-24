pub mod core;
pub mod core_c;
pub mod ir;

pub use core::{
    core_package_ir_dump, lower_core_program, verify_core_ir, CoreIrBackend, CoreIrDiagnostic,
    CoreIrEffect, CoreIrProgram, VerifiedCoreIr,
};
pub use core_c::{core_package_c_dump, CoreCBackend, CoreCError, CoreCOutput};
pub use ir::{IrModule, IrModuleImport, IrProgram, IrProviderContract};
