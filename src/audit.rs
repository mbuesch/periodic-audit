// -*- coding: utf-8 -*-
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright (C) 2026 Michael Büsch <m@bues.ch>

use crate::{
    config::Config,
    report::{Report, ReportEntry},
};
use anyhow::{self as ah, format_err as err};
use serde_json as json;
use std::{path::PathBuf, process::Stdio};
use tokio::{fs::read_dir, process::Command};

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt as _;

fn split_json_parts(input: &str, expected_nr_parts: usize) -> ah::Result<Vec<String>> {
    let mut parts = Vec::with_capacity(expected_nr_parts);
    let mut part = String::with_capacity(input.len());
    let mut indent = 0_i32;
    let mut in_string = false;
    let mut escape = false;

    for c in input.chars() {
        if escape {
            part.push(c);
            escape = false;
            continue;
        }

        match c {
            '\\' => {
                if in_string {
                    escape = true;
                    part.push(c);
                } else {
                    part.push(c);
                }
            }
            '"' => {
                part.push(c);
                in_string = !in_string;
            }
            '{' => {
                if !in_string {
                    indent += 1;
                }
                part.push(c);
            }
            '}' => {
                part.push(c);
                if !in_string {
                    indent -= 1;
                    if indent <= 0 {
                        let ptrim = part.trim();
                        if !ptrim.is_empty() {
                            parts.push(ptrim.to_string());
                        }
                        part.clear();
                        indent = 0;
                    }
                }
            }
            _ => {
                part.push(c);
            }
        }
    }
    if escape {
        return Err(err!("Trailing backslash in JSON data."));
    }
    if in_string {
        return Err(err!("Unterminated string in JSON data."));
    }
    if indent != 0 {
        return Err(err!("Mismatched braces in JSON data (indent = {indent})."));
    }
    if !part.trim().is_empty() {
        return Err(err!("Trailing garbage at end of JSON data."));
    }

    Ok(parts)
}

pub async fn audit_binaries(config: &Config, paths: &[PathBuf]) -> ah::Result<Report, Report> {
    let mut report = Report::new();

    let mut bins = Vec::with_capacity(paths.len() * 8);
    for p in paths {
        match tokio::fs::metadata(p).await {
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                report.add_message(format!(
                    "WARNING: '{}' does not exist; skipped.",
                    p.display()
                ));
                continue;
            }
            Err(e) => {
                report.add_message(format!(
                    "WARNING: Failed to stat path '{}': {}; skipped.",
                    p.display(),
                    e
                ));
            }
            Ok(m) => {
                if m.is_dir() {
                    let mut dir = read_dir(p).await.map_err(|e| {
                        report.fail(format!("Error reading directory '{}': {}", p.display(), e))
                    })?;
                    while let Some(e) = dir.next_entry().await.map_err(|e| {
                        report.fail(format!(
                            "Error reading directory entry in '{}': {e}",
                            p.display(),
                        ))
                    })? {
                        let meta = e.metadata().await.map_err(|e| {
                            report.fail(format!(
                                "Error stating directory entry in '{}': {e}",
                                p.display(),
                            ))
                        })?;

                        #[cfg(unix)]
                        let add = {
                            const EXECUTABLE: u32 = 0o111;
                            !meta.is_dir() && (meta.permissions().mode() & EXECUTABLE) != 0
                        };

                        #[cfg(not(unix))]
                        let add = !meta.is_dir();

                        if add {
                            bins.push(e.path());
                        }
                    }
                } else {
                    bins.push(p.clone());
                }
            }
        }
    }

    if bins.is_empty() {
        report.add_message("WARNING: No existing paths to audit; cargo-audit skipped.".to_string());
    } else {
        // Execute cargo-audit
        let mut cmd = Command::new(config.cargo_audit().exe());
        let mut cmd = cmd
            .arg("audit")
            .args(["--deny", "warnings"])
            .args(["--format", "json"]);
        if let Some(db_path) = &config.cargo_audit().db() {
            cmd = cmd.arg("--db").arg(db_path)
        }
        cmd = cmd
            .arg("bin")
            .args(&bins)
            .env_remove("TERM")
            .env_remove("COLORTERM")
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        let out = cmd.output().await.map_err(|e| {
            report.fail(format!(
                "Error executing cargo-audit ({}): {}",
                config.cargo_audit().exe().display(),
                e
            ))
        })?;

        // Parse cargo-audit output
        let stdout = String::from_utf8(out.stdout)
            .map_err(|e| report.fail(format!("Parse cargo-audit stdout as UTF-8: {}", e)))?;
        if config.cargo_audit().debug() {
            if let Some(code) = out.status.code() {
                report.add_message(format!("cargo-audit exited with code {}", code));
            } else {
                report.add_message("cargo-audit exited due to signal".to_string());
            }
        }
        let parts = split_json_parts(&stdout, bins.len())
            .map_err(|e| report.fail(format!("Split cargo-audit JSON output: {}", e)))?;
        if parts.len() != bins.len() {
            return Err(report.fail(format!(
                "cargo-audit returned {} JSON object(s) but {} binary(ies) were audited",
                parts.len(),
                bins.len()
            )));
        }
        for (path, json_part) in bins.iter().cloned().zip(parts.into_iter()) {
            let audit_result: json::Value = json::from_str(json_part.trim())
                .map_err(|e| report.fail(format!("Parse cargo-audit JSON output: {}", e)))?;

            let json_pretty = json::to_string_pretty(&audit_result)
                .map_err(|e| report.fail(format!("Format cargo-audit JSON output: {}", e)))?;

            if config.cargo_audit().debug() {
                println!("\n\naudit result for {}:", path.display());
                println!("{json_pretty}");
            }

            let vulnerable = audit_result
                .pointer("/vulnerabilities/found")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);

            report.add(ReportEntry {
                path,
                vulnerable,
                json: json_part,
                json_pretty,
            })
        }

        // Get cargo-audit error text, if any
        let stderr = String::from_utf8(out.stderr)
            .map_err(|e| report.fail(format!("Parse cargo-audit stderr as UTF-8: {}", e)))?;
        if !stderr.trim().is_empty() {
            report.add_message(format!("cargo-audit stderr:\n{}", stderr));
        }
    }

    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn split_single_object() {
        let input = r#"  {"a":1}  "#;
        let parts = split_json_parts(input, 0).expect("should split single object");
        assert_eq!(parts.len(), 1);
        assert_eq!(parts[0], r#"{"a":1}"#);
    }

    #[test]
    fn split_multiple_objects() {
        let input = r#"{"a":1}

{"b":2}"#;
        let parts = split_json_parts(input, 1).expect("should split two objects");
        assert_eq!(parts.len(), 2);
        assert_eq!(parts[0], r#"{"a":1}"#);
        assert_eq!(parts[1], r#"{"b":2}"#);
    }

    #[test]
    fn braces_inside_string_dont_affect_split() {
        let input = r#"{"s":"}{"}{}"#;
        let parts = split_json_parts(input, 10).expect("should split into two objects");
        assert_eq!(parts.len(), 2);
        assert_eq!(parts[0], r#"{"s":"}{"}"#);
        assert_eq!(parts[1], r#"{}"#);
    }

    #[test]
    fn unterminated_string_error() {
        let input = r#"{"a":"b}"#;
        let err = split_json_parts(input, 1).unwrap_err();
        assert!(err.to_string().contains("Unterminated string"));
    }

    #[test]
    fn trailing_backslash_error() {
        let input = r#"{"a":"b\"#;
        let err = split_json_parts(input, 1).unwrap_err();
        assert!(err.to_string().contains("Trailing backslash"));
    }

    #[test]
    fn mismatched_braces_error() {
        let input = r#"{"#;
        let err = split_json_parts(input, 1).unwrap_err();
        assert!(err.to_string().contains("Mismatched braces"));
    }

    #[test]
    fn trailing_garbage_error() {
        let input = r#"{} garbage"#;
        let err = split_json_parts(input, 1).unwrap_err();
        assert!(err.to_string().contains("Trailing garbage"));
    }

    #[test]
    fn nested_objects() {
        let input = r#"  {"a":{"b":{"c":3},"arr":[{"x":1}]}}  "#;
        let parts = split_json_parts(input, 1).expect("should handle nested objects");
        assert_eq!(parts.len(), 1);
        assert_eq!(parts[0], r#"{"a":{"b":{"c":3},"arr":[{"x":1}]}}"#);
    }
}

// vim: ts=4 sw=4 expandtab
