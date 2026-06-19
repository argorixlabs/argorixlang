use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConformanceSuite {
    pub suite_version: String,
    pub cases: Vec<ConformanceCase>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConformanceCase {
    pub id: String,
    pub name: String,
    pub category: String,
    pub source_path: Option<String>,
    pub bytecode_path: Option<String>,
    pub stages: Vec<String>,
    pub injection: Option<String>,
    pub mutation: Option<ConformanceMutation>,
    pub expected_failure_stage: Option<String>,
    pub expected_failure_contains: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConformanceMutation {
    pub before_stage: String,
    pub artifact: String,
    pub json_pointer: String,
    pub value: serde_json::Value,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConformanceResult {
    pub suite_version: String,
    pub passed: bool,
    pub cases_total: usize,
    pub cases_passed: usize,
    pub cases_failed: usize,
    pub case_results: Vec<ConformanceCaseResult>,
    pub failures: Vec<ConformanceFailure>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConformanceCaseResult {
    pub id: String,
    pub name: String,
    pub category: String,
    pub passed: bool,
    pub stages: Vec<ConformanceStageResult>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConformanceStageResult {
    pub stage: String,
    pub status: String,
    pub message: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConformanceFailure {
    pub case_id: String,
    pub stage: String,
    pub reason: String,
}
