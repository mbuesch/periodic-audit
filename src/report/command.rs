// -*- coding: utf-8 -*-
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright (C) 2026 Michael Büsch <m@bues.ch>

use crate::{config::Config, report::Report};
use anyhow::{self as ah, Context as _};
use std::process::Stdio;
use tokio::{io::AsyncWriteExt as _, process::Command};

pub async fn run(config: &Config, report: &Report) -> ah::Result<()> {
    let Some(rc) = config.report_command() else {
        return Ok(());
    };
    if rc.disabled() {
        return Ok(());
    }

    let mut child = Command::new(rc.exe())
        .stdin(Stdio::piped())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()
        .with_context(|| format!("Spawn report command '{}'", rc.exe().display()))?;

    let mut stdin = child.stdin.take().context("Open report command stdin")?;
    stdin
        .write_all(format!("{report}").as_bytes())
        .await
        .context("Write report to report-command stdin")?;

    let status = child
        .wait()
        .await
        .context("Wait for report-command to exit")?;

    if !status.success() {
        return Err(ah::format_err!(
            "Report command '{}' exited with status {:?}",
            rc.exe().display(),
            status.code()
        ));
    }
    Ok(())
}
