use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use chrono::Utc;

use crate::diagnostics::types::{
    BuildDiagnosticsFile, DiagnosticsIndex, DiagnosticsIndexEntry, StoredDiagnostic,
};

const AI_DIAGNOSTICS_DIR: &str = "ai-diagnostics";
const INDEX_FILE: &str = "index.json";
const MAX_BUILDS_TO_KEEP: usize = 5;

pub fn persist_build(spago_dir: &Path, diagnostics: &[StoredDiagnostic]) -> Result<Option<PathBuf>> {
    if diagnostics.is_empty() {
        return Ok(None);
    }

    let dir = diagnostics_dir(spago_dir);
    fs::create_dir_all(&dir).context("Failed to create AI diagnostics directory")?;

    let created_at = Utc::now();
    let file_name = format!(
        "build-{}-{}-{}.json",
        created_at.format("%Y%m%dT%H%M%S"),
        created_at.timestamp_subsec_millis(),
        created_at.timestamp_subsec_nanos()
    );
    let build_path = dir.join(&file_name);

    let build_payload = BuildDiagnosticsFile {
        created_at: created_at.to_rfc3339(),
        diagnostics: diagnostics.to_vec(),
    };

    fs::write(
        &build_path,
        serde_json::to_vec(&build_payload).context("Failed to serialize diagnostics sidecar")?,
    )
    .with_context(|| format!("Failed to write sidecar file {}", build_path.display()))?;

    let mut index = load_index(&dir)?;
    index.builds.retain(|f| f != &file_name);
    index.builds.insert(0, file_name.clone());

    for (offset, item) in diagnostics.iter().enumerate() {
        index.entries.insert(
            item.summary.id.clone(),
            DiagnosticsIndexEntry {
                build_file: file_name.clone(),
                offset,
            },
        );
    }

    if index.builds.len() > MAX_BUILDS_TO_KEEP {
        let stale_builds = index.builds.split_off(MAX_BUILDS_TO_KEEP);

        for stale in stale_builds {
            let stale_path = dir.join(&stale);
            let _ = fs::remove_file(stale_path);

            index.entries.retain(|_, entry| entry.build_file != stale);
        }
    }

    write_index(&dir, &index)?;
    Ok(Some(build_path))
}

pub fn load_diagnostic_by_id(spago_dir: &Path, id: &str) -> Result<StoredDiagnostic> {
    let dir = diagnostics_dir(spago_dir);
    let index = load_index(&dir)?;

    let entry = index
        .entries
        .get(id)
        .with_context(|| format!("No diagnostic with id `{}` found", id))?;

    let build_path = dir.join(&entry.build_file);
    let build_payload: BuildDiagnosticsFile = serde_json::from_slice(
        &fs::read(&build_path).with_context(|| {
            format!(
                "Failed to read diagnostics sidecar file `{}`",
                build_path.display()
            )
        })?,
    )
    .with_context(|| format!("Failed to parse diagnostics sidecar `{}`", build_path.display()))?;

    match build_payload.diagnostics.get(entry.offset) {
        Some(found) => Ok(found.clone()),
        None => bail!(
            "Diagnostic id `{}` points to stale offset in `{}`",
            id,
            build_path.display()
        ),
    }
}

fn diagnostics_dir(spago_dir: &Path) -> PathBuf {
    spago_dir.join(AI_DIAGNOSTICS_DIR)
}

fn index_path(dir: &Path) -> PathBuf {
    dir.join(INDEX_FILE)
}

fn load_index(dir: &Path) -> Result<DiagnosticsIndex> {
    let path = index_path(dir);
    if !path.exists() {
        return Ok(DiagnosticsIndex::default());
    }

    let data = fs::read(&path).with_context(|| format!("Failed to read {}", path.display()))?;
    let index =
        serde_json::from_slice(&data).with_context(|| format!("Failed to parse {}", path.display()))?;
    Ok(index)
}

fn write_index(dir: &Path, index: &DiagnosticsIndex) -> Result<()> {
    let path = index_path(dir);
    fs::write(
        &path,
        serde_json::to_vec(index).context("Failed to serialize diagnostics index")?,
    )
    .with_context(|| format!("Failed to write diagnostics index {}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diagnostics::types::CompactDiagnostic;
    use serde_json::json;

    fn sample_diag(id: &str) -> StoredDiagnostic {
        StoredDiagnostic {
            summary: CompactDiagnostic {
                id: id.to_string(),
                severity: "error".to_string(),
                kind: "UnknownName".to_string(),
                file: "src/Main.purs".to_string(),
                line: Some(12),
                column: Some(4),
                hint: Some("Unknown value".to_string()),
            },
            full: json!({
                "filename": "src/Main.purs",
                "type": "error",
                "errorCode": "UnknownName",
                "position": { "startLine": 12, "startColumn": 4 }
            }),
        }
    }

    #[test]
    fn persist_and_lookup_round_trip() {
        let tmp = tempfile::tempdir().unwrap();
        let spago_dir = tmp.path().join(".spago");
        fs::create_dir_all(&spago_dir).unwrap();

        let persisted = persist_build(&spago_dir, &[sample_diag("abc123")]).unwrap();
        assert!(persisted.is_some());

        let found = load_diagnostic_by_id(&spago_dir, "abc123").unwrap();
        assert_eq!(found.summary.id, "abc123");
        assert_eq!(found.summary.file, "src/Main.purs");
    }

    #[test]
    fn retention_keeps_latest_five_builds() {
        let tmp = tempfile::tempdir().unwrap();
        let spago_dir = tmp.path().join(".spago");
        fs::create_dir_all(&spago_dir).unwrap();

        for i in 0..7 {
            let id = format!("diag-{}", i);
            persist_build(&spago_dir, &[sample_diag(&id)]).unwrap();
        }

        let dir = diagnostics_dir(&spago_dir);
        let index = load_index(&dir).unwrap();
        assert_eq!(index.builds.len(), 5);

        // Latest should still exist.
        assert!(load_diagnostic_by_id(&spago_dir, "diag-6").is_ok());
    }
}
