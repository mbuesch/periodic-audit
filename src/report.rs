// -*- coding: utf-8 -*-
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright (C) 2026 Michael Büsch <m@bues.ch>

use chrono::{DateTime, Utc};
use std::path::PathBuf;

pub mod command;
pub mod file;
pub mod mail;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct ReportEntry {
    pub path: PathBuf,
    pub vulnerable: bool,
    pub json: String,
    pub json_pretty: String,
}

#[derive(Debug, Clone, Default)]
pub struct Report {
    stamp: DateTime<Utc>,
    entries: Vec<ReportEntry>,
    messages: Vec<String>,
    failed: bool,
    vulnerable: bool,
}

impl Report {
    pub fn new() -> Self {
        Self {
            stamp: Utc::now(),
            entries: Vec::with_capacity(32),
            messages: Vec::with_capacity(8),
            failed: false,
            vulnerable: false,
        }
    }

    pub fn add(&mut self, entry: ReportEntry) {
        self.vulnerable |= entry.vulnerable;
        self.entries.push(entry);
    }

    pub fn add_message(&mut self, msg: String) {
        self.messages.push(msg);
    }

    pub fn entries(&self) -> &[ReportEntry] {
        &self.entries
    }

    pub fn messages(&self) -> &[String] {
        &self.messages
    }

    pub fn failed(&self) -> bool {
        self.failed
    }

    pub fn fail(&self, message: String) -> Self {
        let mut this = self.clone();
        this.failed = true;
        this.add_message(message);
        this
    }

    pub fn vulnerable(&self) -> bool {
        self.vulnerable
    }
}

impl std::fmt::Display for Report {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let date = self.stamp.format("%+");

        // Summary
        if self.failed() {
            writeln!(f, "[{date}] Audit FAILED.")?;
        } else {
            writeln!(f, "[{date}] Audit results:")?;
            for entry in self.entries() {
                writeln!(
                    f,
                    "  {}: {}",
                    entry.path.display(),
                    if entry.vulnerable { "VULNERABLE" } else { "Ok" }
                )?;
            }
            writeln!(f)?;
        }

        // Log messages
        for msg in self.messages() {
            writeln!(f, "{msg}")?;
        }

        // Vulnerability details
        if !self.failed() {
            for entry in self.entries().iter().filter(|e| e.vulnerable) {
                writeln!(f, "\n\n{}:\n{}", entry.path.display(), entry.json_pretty)?;
            }
        }
        Ok(())
    }
}

// vim: ts=4 sw=4 expandtab
