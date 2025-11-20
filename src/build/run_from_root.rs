use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use regex::Regex;

/// Map sources to be relative to the output directory instead of the current working directory.
///
/// Given sources like `./src/blah` and `./src/hmmm` relative to CWD, and an output directory
/// like `../../` (which resolves to `/lib/my-lib`), this function returns paths relative to
/// the output directory like `../src/blah` and `../src/hmmm` (or absolute paths if needed).
///
/// The function handles both file paths and glob patterns. For glob patterns, it preserves
/// the glob syntax while adjusting the base path. Since the compiler runs with
/// `current_dir(output_dir)`, these paths should be relative to the output directory.
pub fn map_sources_to_output_dir(sources: &[String], output_dir: &PathBuf) -> Result<Vec<String>> {
    let cwd = std::env::current_dir().context("Failed to get current working directory")?;
    map_sources_to_output_dir_impl(sources, output_dir, &cwd)
}

/// Implementation of map_sources_to_output_dir that accepts cwd as a parameter for testability.
fn map_sources_to_output_dir_impl(
    sources: &[String],
    output_dir: &PathBuf,
    cwd: &Path,
) -> Result<Vec<String>> {
    let output_dir = resolve_to_absolute(output_dir, cwd, false)?;

    sources
        .iter()
        .map(|source| {
            // Check if this is a glob pattern (contains * or **)
            let is_glob = source.contains('*');

            if is_glob {
                // For glob patterns, extract the base directory and glob part
                let glob_start = source.find('*').unwrap_or(source.len());
                let base_part = &source[..glob_start];
                let glob_part = &source[glob_start..];

                // Get the directory part (everything up to the last / before *)
                let base_dir = if let Some(last_slash) = base_part.rfind('/') {
                    &base_part[..=last_slash]
                } else if base_part.is_empty() || base_part == "./" {
                    "./"
                } else {
                    base_part
                };

                // Resolve the base directory and make it relative to output directory
                let base_dir_path = Path::new(base_dir);
                let base_dir = resolve_to_absolute(base_dir_path, &cwd, false)?;
                let base_dir_rel = make_relative(&base_dir, &output_dir);

                // Reconstruct the glob pattern with the new base
                let new_base = if base_dir_rel == "." {
                    String::new()
                } else {
                    format!(
                        "{}{}",
                        base_dir_rel,
                        if base_dir_rel.ends_with('/') { "" } else { "/" }
                    )
                };

                Ok(format!("{}{}", new_base, glob_part))
            } else {
                // For file paths, resolve and make relative to output directory
                let source_path = Path::new(source);
                let source = resolve_to_absolute(source_path, &cwd, true)?;
                Ok(make_relative(&source, &output_dir))
            }
        })
        .collect()
}

/// Map diagnostic paths from output-relative to CWD-relative.
///
/// This function finds file paths in compiler diagnostic messages (in various formats:
/// JSON format like `"filename":"path"`, standard format like `path:line:col:`, etc.)
/// and converts them from being relative to the output directory to being relative to
/// the current working directory.
pub fn map_diagnostic_paths_from_output_to_cwd(line: &str, output_dir: &PathBuf) -> Result<String> {
    let cwd = std::env::current_dir().context("Failed to get current working directory")?;
    map_diagnostic_paths_from_output_to_cwd_impl(line, output_dir, &cwd)
}

/// Implementation of map_diagnostic_paths_from_output_to_cwd that accepts cwd as a parameter for testability.
fn map_diagnostic_paths_from_output_to_cwd_impl(
    line: &str,
    output_dir: &PathBuf,
    cwd: &Path,
) -> Result<String> {
    let output_dir = resolve_to_absolute(output_dir, cwd, false)?;

    // Helper function to map a single path
    let map_path = |path_str: &str| -> String {
        let path = Path::new(path_str);

        // Skip if it's already an absolute path (starts with /)
        if path.is_absolute() {
            return path_str.to_string();
        }

        // Join the path with the output directory and normalize
        let joined = output_dir.join(path);
        let normalized = normalize_path(&joined);

        // Make it relative to CWD
        make_relative(&normalized, &cwd)
    };

    // Pattern 1: Match paths in JSON format like "filename":"../path/to/file.purs" or "name":"../path/to/file.purs"
    let json_path_pattern =
        Regex::new(r#""(filename|name)":\s*"((?:\.\.?/)?[^"]+\.purs)""#).unwrap();
    let result = json_path_pattern.replace_all(line, |caps: &regex::Captures| {
        format!("\"{}\":\"{}\"", &caps[1], map_path(&caps[2]))
    });

    // Pattern 2: Match standard format like path/to/file.purs:line:col: or path/to/file.purs
    let standard_path_pattern =
        Regex::new(r"((?:\.\.?/)?[^:\s]+\.purs)(?::\d+:\d+)?(?::|$)").unwrap();
    let result = standard_path_pattern.replace_all(&result, |caps: &regex::Captures| {
        caps[0].replacen(&caps[1], &map_path(&caps[1]), 1)
    });

    Ok(result.to_string())
}

/// Normalize a path by resolving `.` and `..` components.
/// Works even if the path doesn't exist on disk.
fn normalize_path(path: &Path) -> PathBuf {
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            std::path::Component::Prefix(_) | std::path::Component::RootDir => {
                normalized.push(component);
            }
            std::path::Component::ParentDir => {
                if normalized.parent().is_some() {
                    normalized.pop();
                }
            }
            std::path::Component::CurDir => {
                // Skip .
            }
            std::path::Component::Normal(name) => {
                normalized.push(name);
            }
        }
    }
    normalized
}

/// Resolve a path to an absolute path, using canonicalize if possible,
/// otherwise using normalization.
fn resolve_to_absolute(path: &Path, base: &Path, use_canonicalize: bool) -> Result<PathBuf> {
    if path.is_absolute() {
        return Ok(path.to_path_buf());
    }

    let joined = base.join(path);
    if use_canonicalize {
        joined
            .canonicalize()
            .with_context(|| format!("Failed to canonicalize path: {}", path.display()))
    } else {
        Ok(normalize_path(&joined))
    }
}

/// Make a path relative to another path.
/// Returns the relative path as a string, or an absolute path if relativization fails.
fn make_relative(from: &Path, to: &Path) -> String {
    match from.strip_prefix(to) {
        Ok(rel) => {
            if rel.as_os_str().is_empty() {
                ".".to_string()
            } else {
                rel.to_string_lossy().to_string()
            }
        }
        Err(_) => {
            // Path is outside the base, compute relative path
            match pathdiff::diff_paths(from, to) {
                Some(rel) => rel.to_string_lossy().to_string(),
                None => from.to_string_lossy().to_string(),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_path_simple() {
        let path = Path::new("/foo/bar/./baz");
        let normalized = normalize_path(path);
        assert_eq!(normalized, PathBuf::from("/foo/bar/baz"));
    }

    #[test]
    fn test_normalize_path_with_parent_dir() {
        let path = Path::new("/foo/bar/../baz");
        let normalized = normalize_path(path);
        assert_eq!(normalized, PathBuf::from("/foo/baz"));
    }

    #[test]
    fn test_normalize_path_multiple_parent_dirs() {
        let path = Path::new("/foo/bar/baz/../../qux");
        let normalized = normalize_path(path);
        assert_eq!(normalized, PathBuf::from("/foo/qux"));
    }

    #[test]
    fn test_normalize_path_current_dir() {
        let path = Path::new("/foo/./bar/./baz");
        let normalized = normalize_path(path);
        assert_eq!(normalized, PathBuf::from("/foo/bar/baz"));
    }

    #[test]
    fn test_make_relative_same_path() {
        let path = Path::new("/foo/bar");
        let base = Path::new("/foo/bar");
        assert_eq!(make_relative(path, base), ".");
    }

    #[test]
    fn test_make_relative_subdirectory() {
        let path = Path::new("/foo/bar/baz");
        let base = Path::new("/foo/bar");
        assert_eq!(make_relative(path, base), "baz");
    }

    #[test]
    fn test_make_relative_nested_subdirectory() {
        let path = Path::new("/foo/bar/baz/qux");
        let base = Path::new("/foo/bar");
        assert_eq!(make_relative(path, base), "baz/qux");
    }

    #[test]
    fn test_make_relative_parent_directory() {
        let path = Path::new("/foo/bar");
        let base = Path::new("/foo/bar/baz");
        assert_eq!(make_relative(path, base), "..");
    }

    #[test]
    fn test_make_relative_sibling_directory() {
        let path = Path::new("/foo/baz");
        let base = Path::new("/foo/bar");
        assert_eq!(make_relative(path, base), "../baz");
    }

    #[test]
    fn test_resolve_to_absolute_already_absolute() {
        let path = Path::new("/foo/bar");
        let base = Path::new("/some/base");
        let result = resolve_to_absolute(path, base, false).unwrap();
        assert_eq!(result, PathBuf::from("/foo/bar"));
    }

    #[test]
    fn test_resolve_to_absolute_relative_path() {
        let path = Path::new("bar/baz");
        let base = Path::new("/foo");
        let result = resolve_to_absolute(path, base, false).unwrap();
        assert_eq!(result, PathBuf::from("/foo/bar/baz"));
    }

    #[test]
    fn test_resolve_to_absolute_with_parent_dirs() {
        let path = Path::new("../baz");
        let base = Path::new("/foo/bar");
        let result = resolve_to_absolute(path, base, false).unwrap();
        assert_eq!(result, PathBuf::from("/foo/baz"));
    }

    #[test]
    fn test_map_sources_to_output_dir_simple_file() {
        let sources = vec!["./src/Main.purs".to_string()];
        let output_dir = PathBuf::from("/project/output");
        let cwd = Path::new("/project");

        let result = map_sources_to_output_dir_impl(&sources, &output_dir, cwd).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0], "../src/Main.purs");
    }

    #[test]
    fn test_map_sources_to_output_dir_glob_pattern() {
        let sources = vec!["./src/**/*.purs".to_string()];
        let output_dir = PathBuf::from("/project/output");
        let cwd = Path::new("/project");

        let result = map_sources_to_output_dir_impl(&sources, &output_dir, cwd).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0], "../src/**/*.purs");
    }

    #[test]
    fn test_map_sources_to_output_dir_glob_with_wildcard() {
        let sources = vec!["./src/*.purs".to_string()];
        let output_dir = PathBuf::from("/project/output");
        let cwd = Path::new("/project");

        let result = map_sources_to_output_dir_impl(&sources, &output_dir, cwd).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0], "../src/*.purs");
    }

    #[test]
    fn test_map_sources_to_output_dir_multiple_sources() {
        let sources = vec![
            "./src/Main.purs".to_string(),
            "./lib/Utils.purs".to_string(),
        ];
        let output_dir = PathBuf::from("/project/output");
        let cwd = Path::new("/project");

        let result = map_sources_to_output_dir_impl(&sources, &output_dir, cwd).unwrap();
        assert_eq!(result.len(), 2);
        assert_eq!(result[0], "../src/Main.purs");
        assert_eq!(result[1], "../lib/Utils.purs");
    }

    #[test]
    fn test_map_diagnostic_paths_json_format() {
        let line = r#"{"filename":"../src/Main.purs","position":{"startLine":10}}"#;
        let output_dir = PathBuf::from("/project/output");
        let cwd = Path::new("/project");

        let result = map_diagnostic_paths_from_output_to_cwd_impl(line, &output_dir, cwd).unwrap();
        assert!(result.contains(r#""filename":"src/Main.purs""#));
    }

    #[test]
    fn test_map_diagnostic_paths_standard_format() {
        let line = "../src/Main.purs:10:5: error message";
        let output_dir = PathBuf::from("/project/output");
        let cwd = Path::new("/project");

        let result = map_diagnostic_paths_from_output_to_cwd_impl(line, &output_dir, cwd).unwrap();
        assert!(result.starts_with("src/Main.purs:10:5:"));
    }

    #[test]
    fn test_map_diagnostic_paths_name_field() {
        let line = r#"{"name":"../lib/Utils.purs","type":"warning"}"#;
        let output_dir = PathBuf::from("/project/output");
        let cwd = Path::new("/project");

        let result = map_diagnostic_paths_from_output_to_cwd_impl(line, &output_dir, cwd).unwrap();
        assert!(result.contains(r#""name":"lib/Utils.purs""#));
    }

    #[test]
    fn test_map_diagnostic_paths_absolute_path() {
        let line = r#"{"filename":"/absolute/path/Main.purs","position":{"startLine":10}}"#;
        let output_dir = PathBuf::from("/project/output");
        let cwd = Path::new("/project");

        let result = map_diagnostic_paths_from_output_to_cwd_impl(line, &output_dir, cwd).unwrap();
        // Absolute paths should remain unchanged
        assert!(result.contains(r#""filename":"/absolute/path/Main.purs""#));
    }

    #[test]
    fn test_map_diagnostic_paths_multiple_paths() {
        let line = r#"{"filename":"../src/Main.purs"} and also {"name":"../lib/Utils.purs"}"#;
        let output_dir = PathBuf::from("/project/output");
        let cwd = Path::new("/project");

        let result = map_diagnostic_paths_from_output_to_cwd_impl(line, &output_dir, cwd).unwrap();
        assert!(result.contains(r#""filename":"src/Main.purs""#));
        assert!(result.contains(r#""name":"lib/Utils.purs""#));
    }

    #[test]
    fn test_map_diagnostic_paths_with_current_dir() {
        let line = "./Main.purs:5:10: error";
        let output_dir = PathBuf::from("/project/output");
        let cwd = Path::new("/project");

        let result = map_diagnostic_paths_from_output_to_cwd_impl(line, &output_dir, cwd).unwrap();
        assert!(result.starts_with("output/Main.purs:5:10:"));
    }
}
