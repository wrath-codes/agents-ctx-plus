use std::fs;
use std::path::Path;

use serde::Serialize;
use zen_schema::SchemaRegistry;

use crate::error::HookError;

#[derive(Debug, Clone, Serialize)]
pub struct TrailValidationError {
    pub file: String,
    pub line: usize,
    pub message: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct TrailValidationReport {
    pub files_checked: usize,
    pub operations_checked: usize,
    pub errors: Vec<TrailValidationError>,
}

impl TrailValidationReport {
    #[must_use]
    pub fn is_valid(&self) -> bool {
        self.errors.is_empty()
    }
}

pub fn validate_staged_trail_files(
    project_root: &Path,
) -> Result<TrailValidationReport, HookError> {
    let repo = gix::discover(project_root)
        .map_err(|_| HookError::NotGitRepo(project_root.to_path_buf()))?;
    let index = repo
        .open_index()
        .map_err(|error| HookError::Git(error.to_string()))?;

    let staged = index
        .entries()
        .iter()
        .filter_map(|entry| {
            let path = entry.path(&index).to_string();
            if path.starts_with(".zenith/trail/") && path.ends_with(".jsonl") {
                Some(path)
            } else {
                None
            }
        })
        .collect::<Vec<_>>();

    let schema = SchemaRegistry::new();
    let mut errors = Vec::new();
    let mut operations_checked = 0usize;

    for rel in &staged {
        let full_path = project_root.join(rel);
        let content = fs::read_to_string(&full_path)?;

        for (line_idx, line) in content.lines().enumerate() {
            let line_no = line_idx + 1;
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            operations_checked += 1;

            if trimmed.starts_with('\u{feff}') {
                errors.push(TrailValidationError {
                    file: rel.clone(),
                    line: line_no,
                    message: "BOM detected".to_string(),
                });
                continue;
            }

            if trimmed.starts_with("<<<<<<<")
                || trimmed.starts_with("=======")
                || trimmed.starts_with(">>>>>>>")
            {
                errors.push(TrailValidationError {
                    file: rel.clone(),
                    line: line_no,
                    message: "git conflict marker detected".to_string(),
                });
                continue;
            }

            let value: serde_json::Value = match serde_json::from_str(trimmed) {
                Ok(value) => value,
                Err(error) => {
                    errors.push(TrailValidationError {
                        file: rel.clone(),
                        line: line_no,
                        message: format!("invalid JSON: {error}"),
                    });
                    continue;
                }
            };

            if let Err(error) = schema.validate("trail_operation", &value) {
                errors.push(TrailValidationError {
                    file: rel.clone(),
                    line: line_no,
                    message: format!("schema validation failed: {error}"),
                });
            }
        }
    }

    Ok(TrailValidationReport {
        files_checked: staged.len(),
        operations_checked,
        errors,
    })
}
