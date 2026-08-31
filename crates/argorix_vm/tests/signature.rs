use argorix_bytecode::BytecodeProgram;
use argorix_vm::{
    evidence::{verify_evidence, verify_evidence_with_anchor, EvidenceBundle},
    signature::{
        encode_hex, signature_path, EvidenceSignature, SIGNATURE_ALGORITHM, SIGNATURE_VERSION,
    },
    InjectedMessage, SecurityReport, Vm,
};
use ed25519_dalek::{Signer, SigningKey};
use std::{fs, path::Path, path::PathBuf};

fn fixture(name: &str) -> BytecodeProgram {
    let raw = match name {
        "a" => include_str!("../../../examples/provider_allowlists_v013.argbc.json"),
        _ => include_str!("../../../examples/policy_assertions_v09.argbc.json"),
    };
    serde_json::from_str(raw).unwrap()
}

fn injection() -> InjectedMessage {
    InjectedMessage {
        from: "User".into(),
        to: "ResearchAgent".into(),
        act: "tell".into(),
        message_type: "UserPrompt".into(),
    }
}

fn temp_root(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!("argorix-signature-{}-{name}", std::process::id()))
}

fn write_json<T: serde::Serialize>(path: &Path, value: &T) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(
        path,
        format!("{}\n", serde_json::to_string_pretty(value).unwrap()),
    )
    .unwrap();
}

/// Produce a complete, verifiable evidence set in `root`.
fn generate(root: &Path, which: &str) -> PathBuf {
    let bundle_path = root.join("run.bundle.json");
    let bytecode_path = root.join("program.argbc.json");
    let trace_path = root.join("run.trace.json");
    let report_path = root.join("run.security.json");
    let bytecode = fixture(which);
    let outcome = Vm::new().run_reactive_outcome(&bytecode, injection());
    let trace = outcome.result.as_ref().unwrap();
    let report = SecurityReport::from_outcome(&bytecode, &outcome);
    let bundle = EvidenceBundle::from_outcome(
        &bytecode,
        &outcome,
        &report,
        &bundle_path,
        Some(&bytecode_path),
        Some(&trace_path),
        Some(&report_path),
        None,
    )
    .unwrap();
    write_json(&bytecode_path, &bytecode);
    write_json(&trace_path, trace);
    write_json(&report_path, &report);
    write_json(&bundle_path, &bundle);
    bundle_path
}

fn key(seed: u8) -> SigningKey {
    SigningKey::from_bytes(&[seed; 32])
}

fn sign_bundle(bundle_path: &Path, signing: &SigningKey) {
    let bundle: EvidenceBundle = serde_json::from_slice(&fs::read(bundle_path).unwrap()).unwrap();
    let canonical = serde_json::to_vec(&bundle).unwrap();
    let document = EvidenceSignature {
        signature_version: SIGNATURE_VERSION.into(),
        algorithm: SIGNATURE_ALGORITHM.into(),
        public_key: encode_hex(signing.verifying_key().as_bytes()),
        bundle_digest: argorix_vm::evidence::canonical_digest(&bundle).unwrap(),
        signature: encode_hex(&signing.sign(&canonical).to_bytes()),
    };
    write_json(&signature_path(bundle_path), &document);
}

#[test]
fn signed_bundle_verifies_against_its_trust_anchor() {
    let root = temp_root("signed-pass");
    let bundle_path = generate(&root, "a");
    let signing = key(7);
    sign_bundle(&bundle_path, &signing);

    let result =
        verify_evidence_with_anchor(&bundle_path, Some(signing.verifying_key().as_bytes()))
            .unwrap();

    assert!(result.passed, "{:?}", result.failures);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn coordinated_unsigned_replacement_passes_without_an_anchor_and_fails_with_one() {
    let root = temp_root("replacement");
    let bundle_path = generate(&root, "a");
    let signing = key(7);
    sign_bundle(&bundle_path, &signing);

    // Replace the bundle and every artifact by a different, self-consistent
    // set. Digest verification cannot tell the difference.
    let replacement = root.join("replacement");
    let replaced = generate(&replacement, "b");
    for name in [
        "run.bundle.json",
        "program.argbc.json",
        "run.trace.json",
        "run.security.json",
    ] {
        fs::copy(replacement.join(name), root.join(name)).unwrap();
    }
    let _ = replaced;

    let without = verify_evidence(&bundle_path).unwrap();
    assert!(
        without.passed,
        "integrity alone cannot detect a self-consistent replacement"
    );

    let with = verify_evidence_with_anchor(&bundle_path, Some(signing.verifying_key().as_bytes()))
        .unwrap();
    assert!(!with.passed);
    assert!(with
        .failures
        .iter()
        .any(|failure| failure.starts_with("signature:")));
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn anchor_without_a_signature_fails_closed() {
    let root = temp_root("no-signature");
    let bundle_path = generate(&root, "a");

    let result =
        verify_evidence_with_anchor(&bundle_path, Some(key(7).verifying_key().as_bytes())).unwrap();

    assert!(!result.passed);
    assert!(result.failures.iter().any(
        |failure| failure == "a trust anchor was supplied but the bundle carries no signature"
    ));
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn signature_from_a_foreign_key_is_rejected() {
    let root = temp_root("foreign-key");
    let bundle_path = generate(&root, "a");
    sign_bundle(&bundle_path, &key(9));

    let result =
        verify_evidence_with_anchor(&bundle_path, Some(key(7).verifying_key().as_bytes())).unwrap();

    assert!(!result.passed);
    assert!(result
        .failures
        .iter()
        .any(|failure| failure.contains("not the trust anchor")));
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn an_unsigned_bundle_is_unaffected_when_no_anchor_is_supplied() {
    let root = temp_root("no-anchor");
    let bundle_path = generate(&root, "a");

    let result = verify_evidence(&bundle_path).unwrap();

    assert!(result.passed, "{:?}", result.failures);
    fs::remove_dir_all(root).unwrap();
}
