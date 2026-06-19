use crate::{types::ConformanceMutation, workspace::CaseWorkspace};
use std::{fs, path::Path};

pub fn apply_mutation(
    mutation: &ConformanceMutation,
    workspace: &CaseWorkspace,
) -> Result<(), String> {
    let path = artifact_path(&mutation.artifact, workspace)?;
    if !path.exists() {
        return Err(format!(
            "mutation artifact `{}` does not exist",
            mutation.artifact
        ));
    }
    let source = fs::read_to_string(path)
        .map_err(|error| format!("failed to read mutation artifact: {error}"))?;
    let mut value: serde_json::Value = serde_json::from_str(&source)
        .map_err(|error| format!("mutation artifact is invalid JSON: {error}"))?;
    let target = value.pointer_mut(&mutation.json_pointer).ok_or_else(|| {
        format!(
            "mutation JSON Pointer `{}` does not exist",
            mutation.json_pointer
        )
    })?;
    *target = mutation.value.clone();
    workspace
        .write_json(path, &value)
        .map_err(|error| error.to_string())
}

fn artifact_path<'a>(artifact: &str, workspace: &'a CaseWorkspace) -> Result<&'a Path, String> {
    match artifact {
        "bytecode" => Ok(&workspace.bytecode),
        "security_report" => Ok(&workspace.report),
        "bundle" => Ok(&workspace.bundle),
        other => Err(format!("unsupported mutation artifact `{other}`")),
    }
}
