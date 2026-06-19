use crate::ModuleError;

/// The minimal `argorix.toml` package manifest.
///
/// v0.16 only supports local packages: a name, a version, and an entry file.
/// There are no external dependencies, registries, or absolute paths.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Manifest {
    pub name: String,
    pub version: String,
    pub entry_main: String,
}

/// Parse the supported subset of TOML used by `argorix.toml`.
///
/// The parser is intentionally tiny and deterministic: it understands `[section]`
/// headers, `key = "value"` pairs, `#` comments, and blank lines. Anything else is
/// rejected so manifests stay auditable.
pub fn parse_manifest(source: &str) -> Result<Manifest, ModuleError> {
    let mut section = String::new();
    let mut name = None;
    let mut version = None;
    let mut entry_main = None;

    for (number, raw) in source.lines().enumerate() {
        let line = strip_comment(raw).trim();
        if line.is_empty() {
            continue;
        }
        if let Some(inner) = line
            .strip_prefix('[')
            .and_then(|rest| rest.strip_suffix(']'))
        {
            section = inner.trim().to_owned();
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            return Err(ModuleError::ManifestParse(format!(
                "line {}: expected `key = \"value\"`",
                number + 1
            )));
        };
        let key = key.trim();
        let value = parse_string(value.trim()).ok_or_else(|| {
            ModuleError::ManifestParse(format!(
                "line {}: value for `{key}` must be a quoted string",
                number + 1
            ))
        })?;
        match (section.as_str(), key) {
            ("package", "name") => name = Some(value),
            ("package", "version") => version = Some(value),
            ("entry", "main") => entry_main = Some(value),
            _ => {
                return Err(ModuleError::ManifestParse(format!(
                    "line {}: unsupported manifest key `{key}` in section `[{section}]`",
                    number + 1
                )))
            }
        }
    }

    let entry_main =
        entry_main.ok_or_else(|| ModuleError::ManifestParse("missing `entry.main`".to_owned()))?;
    if entry_main.is_empty() {
        return Err(ModuleError::ManifestParse(
            "`entry.main` must not be empty".to_owned(),
        ));
    }
    Ok(Manifest {
        name: name
            .ok_or_else(|| ModuleError::ManifestParse("missing `package.name`".to_owned()))?,
        version: version
            .ok_or_else(|| ModuleError::ManifestParse("missing `package.version`".to_owned()))?,
        entry_main,
    })
}

fn strip_comment(line: &str) -> &str {
    match line.find('#') {
        Some(index) => &line[..index],
        None => line,
    }
}

fn parse_string(value: &str) -> Option<String> {
    let bytes = value.as_bytes();
    if bytes.len() >= 2 && bytes[0] == b'"' && bytes[bytes.len() - 1] == b'"' {
        Some(value[1..value.len() - 1].to_owned())
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::parse_manifest;

    #[test]
    fn parses_minimal_manifest() {
        let manifest = parse_manifest(
            r#"
            [package]
            name = "argorix-example"
            version = "0.16.0"

            [entry]
            main = "src/main.argx"
            "#,
        )
        .unwrap();
        assert_eq!(manifest.name, "argorix-example");
        assert_eq!(manifest.version, "0.16.0");
        assert_eq!(manifest.entry_main, "src/main.argx");
    }

    #[test]
    fn rejects_missing_entry() {
        let error = parse_manifest("[package]\nname = \"x\"\nversion = \"0.16.0\"\n").unwrap_err();
        assert!(error.to_string().contains("entry.main"));
    }
}
