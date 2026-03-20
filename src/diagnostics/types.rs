use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompactDiagnostic {
    pub id: String,
    pub severity: String,
    pub kind: String,
    pub file: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub column: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hint: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredDiagnostic {
    pub summary: CompactDiagnostic,
    pub full: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildDiagnosticsFile {
    pub created_at: String,
    pub diagnostics: Vec<StoredDiagnostic>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiagnosticsIndexEntry {
    pub build_file: String,
    pub offset: usize,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DiagnosticsIndex {
    pub builds: Vec<String>,
    pub entries: std::collections::HashMap<String, DiagnosticsIndexEntry>,
}
