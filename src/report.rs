// -*- coding: utf-8 -*-
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright (C) 2026 Michael Büsch <m@bues.ch>

use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct ReportEntry {
    pub path: PathBuf,
    pub vulnerable: bool,
    pub json: String,
}

#[derive(Debug, Clone, Default)]
pub struct Report {
    entries: Vec<ReportEntry>,
    messages: Vec<String>,
    failed: bool,
}

impl Report {
    pub fn new() -> Self {
        Self {
            entries: vec![],
            messages: vec![],
            failed: false,
        }
    }

    pub fn add(&mut self, entry: ReportEntry) {
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
        self.entries.iter().any(|entry| entry.vulnerable)
    }
}

impl std::fmt::Display for Report {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.failed() {
            writeln!(f, "Audit FAILED.")?;
        } else {
            writeln!(f, "Audit results:")?;
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
        for msg in self.messages() {
            writeln!(f, "{msg}")?;
        }
        Ok(())
    }
}

// vim: ts=4 sw=4 expandtab
