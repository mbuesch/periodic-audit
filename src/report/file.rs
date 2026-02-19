// -*- coding: utf-8 -*-
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright (C) 2026 Michael Büsch <m@bues.ch>

use crate::{config::Config, report::Report};
use anyhow::{self as ah, Context as _};
use tokio::{fs::OpenOptions, io::AsyncWriteExt as _};

pub async fn write_report(config: &Config, report: &Report) -> ah::Result<()> {
    let Some(rf) = config.report_file() else {
        return Ok(());
    };
    if rf.disabled() {
        return Ok(());
    }

    let mut opts = OpenOptions::new();
    opts.create(true).write(true);
    if rf.append() {
        opts.append(true);
    } else {
        opts.truncate(true);
    }
    let mut file = opts
        .open(rf.path())
        .await
        .with_context(|| format!("Open report file '{}'", rf.path().display()))?;

    let s = format!("{report}\n\n\n==========================================================\n\n");

    file.write_all(s.as_bytes())
        .await
        .with_context(|| format!("Write report to '{}'", rf.path().display()))?;

    Ok(())
}
