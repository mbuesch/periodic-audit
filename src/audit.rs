// -*- coding: utf-8 -*-
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright (C) 2026 Michael Büsch <m@bues.ch>

use crate::{
    config::Config,
    report::{Report, ReportEntry},
};
use anyhow as ah;
use serde_json as json;
use std::{path::PathBuf, process::Stdio};
use tokio::{fs::read_dir, process::Command};

fn split_json_parts(input: &str) -> Vec<String> {
    let mut parts = Vec::with_capacity((input.len() / 64).max(1));
    let mut part = String::with_capacity(input.len());
    let mut indent = 0_i32;
    for c in input.chars() {
        match c {
            '{' => {
                indent += 1;
                part.push(c);
            }
            '}' => {
                indent -= 1;
                part.push(c);
                if indent <= 0 {
                    let ptrim = part.trim();
                    if !ptrim.is_empty() {
                        parts.push(ptrim.to_string());
                    }
                    part.clear();
                    indent = 0;
                }
            }
            '\n' => {
                let ptrim = part.trim();
                if !ptrim.is_empty() {
                    parts.push(ptrim.to_string());
                }
                part.clear();
                indent = 0;
            }
            _ => {
                if indent > 0 {
                    part.push(c);
                }
            }
        }
    }
    parts
}

pub async fn audit_binaries(config: &Config, paths: &[PathBuf]) -> ah::Result<Report, Report> {
    let mut report = Report::new();

    let mut bins = Vec::with_capacity(paths.len() * 2);
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
                        if !e
                            .metadata()
                            .await
                            .map_err(|e| {
                                report.fail(format!(
                                    "Error stating directory entry in '{}': {e}",
                                    p.display(),
                                ))
                            })?
                            .is_dir()
                        {
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
        let mut cmd = Command::new(&config.cargo_audit.exe);
        let mut cmd = cmd
            .arg("audit")
            .args(["--deny", "warnings"])
            .args(["--format", "json"]);
        if let Some(db_path) = &config.cargo_audit.db {
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
                config.cargo_audit.exe.display(),
                e
            ))
        })?;

        // Parse cargo-audit output
        let stdout = String::from_utf8(out.stdout)
            .map_err(|e| report.fail(format!("Parse cargo-audit stdout as UTF-8: {}", e)))?;
        if config.cargo_audit.debug() {
            if let Some(code) = out.status.code() {
                report.add_message(format!("cargo-audit exited with code {}", code));
            } else {
                report.add_message("cargo-audit exited due to signal".to_string());
            }
        }
        for (i, json_part) in split_json_parts(&stdout).into_iter().enumerate() {
            let path = bins[i].clone();

            let audit_result: json::Value = json::from_str(json_part.trim())
                .map_err(|e| report.fail(format!("Parse cargo-audit JSON output: {}", e)))?;

            if config.cargo_audit.debug() {
                println!("\n\naudit result for {}:", path.display());
                println!(
                    "{}",
                    json::to_string_pretty(&audit_result).map_err(|e| {
                        report.fail(format!("Format cargo-audit JSON output: {}", e))
                    })?
                );
            }

            let vulnerable = audit_result
                .pointer("/vulnerabilities/found")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);

            report.add(ReportEntry {
                path,
                vulnerable,
                json: json_part,
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

// vim: ts=4 sw=4 expandtab
