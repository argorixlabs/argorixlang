use argorix_conformance::types::{
    ConformanceCaseResult, ConformanceFailure, ConformanceResult, ConformanceStageResult,
    ConformanceSuite,
};
use serde_json::json;

#[test]
fn suite_deserializes_approved_case_schema() {
    let suite: ConformanceSuite = serde_json::from_value(json!({
        "suite_version": "0.16",
        "cases": [{
            "id": "evidence-report-tamper",
            "name": "Evidence verification fails when report changes",
            "category": "offline_verification",
            "source_path": "sources/provider_allowlists_v015.argx",
            "bytecode_path": null,
            "stages": ["parse", "semantic_check", "emit_ir", "emit_bytecode", "verify_bytecode", "run_vm", "security_report", "trace_out", "evidence_bundle", "verify_evidence"],
            "injection": "User:ResearchAgent:tell:UserPrompt",
            "mutation": {
                "before_stage": "verify_evidence",
                "artifact": "security_report",
                "json_pointer": "/module",
                "value": "Tampered"
            },
            "expected_failure_stage": "verify_evidence",
            "expected_failure_contains": "report_digest mismatch"
        }]
    }))
    .unwrap();

    assert_eq!(suite.suite_version, "0.16");
    assert_eq!(
        suite.cases[0].mutation.as_ref().unwrap().artifact,
        "security_report"
    );
    assert_eq!(
        suite.cases[0].expected_failure_stage.as_deref(),
        Some("verify_evidence")
    );
}

#[test]
fn result_serializes_stably_with_case_and_stage_results() {
    let result = ConformanceResult {
        suite_version: "0.16".into(),
        passed: false,
        cases_total: 1,
        cases_passed: 0,
        cases_failed: 1,
        case_results: vec![ConformanceCaseResult {
            id: "broken".into(),
            name: "Broken case".into(),
            category: "semantics".into(),
            passed: false,
            stages: vec![
                ConformanceStageResult {
                    stage: "parse".into(),
                    status: "passed".into(),
                    message: None,
                },
                ConformanceStageResult {
                    stage: "semantic_check".into(),
                    status: "failed".into(),
                    message: Some("unknown capability".into()),
                },
            ],
        }],
        failures: vec![ConformanceFailure {
            case_id: "broken".into(),
            stage: "semantic_check".into(),
            reason: "unknown capability".into(),
        }],
    };

    let value = serde_json::to_value(&result).unwrap();
    assert_eq!(value["suite_version"], "0.16");
    assert_eq!(value["case_results"][0]["stages"][1]["status"], "failed");
    assert_eq!(value["failures"][0]["case_id"], "broken");
}
