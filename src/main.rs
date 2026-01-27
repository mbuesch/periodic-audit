// -*- coding: utf-8 -*-
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright (C) 2026 Michael Büsch <m@bues.ch>

use crate::{audit::audit_binaries, config::Config, mail::send_report};
use anyhow::{self as ah, Context as _};
use clap::Parser;
use std::{path::PathBuf, sync::Arc, time::Duration};
use tokio::{runtime, time::sleep};

mod audit;
mod config;
mod mail;
mod report;

#[derive(Parser, Debug, Clone)]
struct Opts {
    /// Override the default path to the configuration file.
    #[arg(short, long)]
    config: Option<PathBuf>,

    /// Show version information and exit.
    #[arg(long, short = 'v')]
    version: bool,
}

impl Opts {
    /// Get the configuration path from command line or default.
    pub fn get_config(&self) -> PathBuf {
        if let Some(config) = &self.config {
            config.clone()
        } else {
            Config::get_default_path()
        }
    }
}

async fn async_main(opts: Arc<Opts>) -> ah::Result<()> {
    // Load the configuration file.
    let conf = Config::load(&opts.get_config()).await.context(format!(
        "Load configuration file '{}'",
        opts.get_config().display()
    ))?;

    // Run cargo-audit on the specified paths, retrying on failure.
    let mut tries = 0_u32;
    let report = loop {
        let report = match audit_binaries(&conf, &conf.watch.paths).await {
            Ok(report) => {
                println!("{report}");
                if !report.failed() {
                    break report;
                }
                report
            }
            Err(report) => {
                eprintln!("Error during audit:\n{report}");
                report
            }
        };
        tries += 1;
        if tries >= conf.cargo_audit.tries().min(30) {
            break report;
        }
        eprintln!("One or more audits failed. Retrying...");
        sleep(Duration::from_secs((1 << (tries - 1)).min(60))).await;
    };

    // Send the report e-mail.
    send_report(&conf, &report).context("Send report e-mail")?;

    Ok(())
}

fn main() -> ah::Result<()> {
    let opts = Arc::new(Opts::parse());

    if opts.version {
        println!("periodic-audit version {}", env!("CARGO_PKG_VERSION"));
        return Ok(());
    }

    runtime::Builder::new_current_thread()
        .thread_keep_alive(Duration::from_millis(500))
        .max_blocking_threads(16)
        .enable_all()
        .build()
        .context("Tokio runtime builder")?
        .block_on(async_main(opts))
}

// vim: ts=4 sw=4 expandtab
