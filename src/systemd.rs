// -*- coding: utf-8 -*-
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright (C) 2026 Michael Büsch <m@bues.ch>

use anyhow as ah;

/// Notify ready-status to systemd.
pub fn systemd_notify_ready() -> ah::Result<()> {
    sd_notify::notify(false, &[sd_notify::NotifyState::Ready])?;
    Ok(())
}

// vim: ts=4 sw=4 expandtab
