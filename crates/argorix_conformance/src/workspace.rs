use serde::Serialize;
use std::{
    fs,
    path::{Path, PathBuf},
};

use crate::ConformanceError;

pub struct CaseWorkspace {
    pub dir: PathBuf,
    pub ir: PathBuf,
    pub bytecode: PathBuf,
    pub trace: PathBuf,
    pub report: PathBuf,
    pub bundle: PathBuf,
}

impl CaseWorkspace {
    pub fn create(workdir: &Path, case_id: &str) -> Result<Self, ConformanceError> {
        let root = absolute(workdir)?;
        let dir = root.join(case_id);
        if !dir.starts_with(&root) {
            return Err(ConformanceError::Workspace(
                "case directory escapes workdir".into(),
            ));
        }
        if dir.exists() {
            fs::remove_dir_all(&dir).map_err(|error| {
                ConformanceError::Workspace(format!(
                    "failed to recreate case workdir `{case_id}`: {error}"
                ))
            })?;
        }
        fs::create_dir_all(&dir).map_err(|error| {
            ConformanceError::Workspace(format!(
                "failed to create case workdir `{case_id}`: {error}"
            ))
        })?;
        Ok(Self {
            ir: dir.join("ir.json"),
            bytecode: dir.join("program.argbc.json"),
            trace: dir.join("run.trace.json"),
            report: dir.join("run.security.json"),
            bundle: dir.join("run.bundle.json"),
            dir,
        })
    }

    pub fn write_json<T: Serialize>(&self, path: &Path, value: &T) -> Result<(), ConformanceError> {
        let json = serde_json::to_string_pretty(value)?;
        fs::write(path, format!("{json}\n")).map_err(|error| {
            ConformanceError::Workspace(format!(
                "failed to write artifact `{}`: {error}",
                path.file_name()
                    .and_then(|name| name.to_str())
                    .unwrap_or("artifact")
            ))
        })
    }
}

fn absolute(path: &Path) -> Result<PathBuf, ConformanceError> {
    if path.is_absolute() {
        Ok(path.to_path_buf())
    } else {
        std::env::current_dir()
            .map(|cwd| cwd.join(path))
            .map_err(|error| ConformanceError::Workspace(error.to_string()))
    }
}
