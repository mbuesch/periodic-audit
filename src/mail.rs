// -*- coding: utf-8 -*-
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright (C) 2026 Michael Büsch <m@bues.ch>

use crate::{config::Config, report::Report};
use anyhow::{self as ah, Context as _};
use lettre::{
    AsyncSmtpTransport, AsyncTransport as _, Message, Tokio1Executor, message::header::ContentType,
};

pub async fn send_report(config: &Config, report: &Report) -> ah::Result<()> {
    if config.mail.disabled() {
        println!("Mail sending is disabled; not sending report e-mail.");
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
        config.mail.subject,
    );
    let report_string = format!("{report}");

    let mut messages = Vec::with_capacity(config.mail.to.len());

    for to in &config.mail.to {
        let message = Message::builder()
            .from(
                config
                    .mail
                    .from
                    .parse()
                    .context("Parse mail.from address")?,
            )
            .to(to.parse().context("Parse mail.to address")?)
            .subject(&subject)
            .user_agent("periodic-audit".to_string())
            .header(ContentType::TEXT_PLAIN)
            .body(report_string.clone())?;
        messages.push(message);
    }

    let transport = if let Some(relay) = &config.mail.relay {
        AsyncSmtpTransport::<Tokio1Executor>::from_url(relay)?.build()
    } else {
        AsyncSmtpTransport::<Tokio1Executor>::unencrypted_localhost()
    };

    for message in messages {
        transport.send(message).await.context("Send e-mail")?;
    }

    Ok(())
}

// vim: ts=4 sw=4 expandtab
