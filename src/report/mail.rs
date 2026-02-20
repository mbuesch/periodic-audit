// -*- coding: utf-8 -*-
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright (C) 2026 Michael Büsch <m@bues.ch>

use crate::{config::Config, report::Report};
use anyhow::{self as ah, Context as _};
use lettre::{
    AsyncSmtpTransport, AsyncTransport as _, Message, Tokio1Executor,
    message::{Mailbox, header::ContentType},
};
use std::sync::Arc;
use tokio::{sync::Semaphore, task::JoinSet};

pub async fn send_report(config: &Config, report: &Report) -> ah::Result<()> {
    let Some(rm) = config.report_mail() else {
        return Ok(());
    };
    if rm.disabled() {
        return Ok(());
    }
    if rm.to().is_empty() {
        println!("No report_mail.to addresses configured; not sending report e-mail.");
        return Ok(());
    }

    let subject = format!(
        "{}{}",
        if report.failed() {
            "[AUDIT FAILED] "
        } else if report.vulnerable() {
            "[VULNERABILITIES FOUND] "
        } else {
            ""
        },
        rm.subject(),
    );
    let from: Mailbox = rm
        .from()
        .parse()
        .context("Parse report_mail.from address")?;
    let report_string = format!("{report}");

    let mut messages = Vec::with_capacity(rm.to().len());

    for to in rm.to() {
        let message = Message::builder()
            .from(from.clone())
            .to(to.parse().context("Parse report_mail.to address")?)
            .subject(&subject)
            .user_agent("periodic-audit".to_string())
            .header(ContentType::TEXT_PLAIN)
            .body(report_string.clone())?;
        messages.push(message);
    }

    let transport = if let Some(relay) = &rm.relay() {
        Arc::new(AsyncSmtpTransport::<Tokio1Executor>::from_url(relay)?.build())
    } else {
        Arc::new(AsyncSmtpTransport::<Tokio1Executor>::unencrypted_localhost())
    };

    let sema = Arc::new(Semaphore::new(rm.max_concurrency()));
    let mut set = JoinSet::new();
    for message in messages {
        let transport = Arc::clone(&transport);
        let sema = Arc::clone(&sema);
        set.spawn(async move {
            let _permit = sema.acquire_owned().await;
            transport.send(message).await.context("Send e-mail")
        });
    }
    while let Some(res) = set.join_next().await {
        res.context("Join task")??;
    }

    Ok(())
}

// vim: ts=4 sw=4 expandtab
