pub mod bytecode;
pub mod lower;

pub use bytecode::{
    verify_bytecode, BytecodeA2ABridgeContract, BytecodeATrustBoundary,
    BytecodeATrustCredentialContract, BytecodeATrustHandshake, BytecodeATrustIdentity,
    BytecodeAdapter, BytecodeAdapterProfile, BytecodeAgent, BytecodeAssertion, BytecodeCapability,
    BytecodeCrypto, BytecodeCryptoBoundary, BytecodeDidMethod, BytecodeError, BytecodeFailure,
    BytecodeFeature, BytecodeMcpBridgeContract, BytecodeModel, BytecodeModule,
    BytecodeModuleImport, BytecodePassport, BytecodePassportAsn, BytecodePolicy,
    BytecodePolicyRule, BytecodePolicyViolation, BytecodeProgram, BytecodeProviderContract,
    BytecodeProviderHarness, BytecodeSecret, BytecodeTool, BytecodeTrustLedger,
    BytecodeTrustLedgerEntry, BytecodeType, BytecodeTypeField, Instruction,
};
pub use lower::lower_ir;
