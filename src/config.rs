// -*- coding: utf-8 -*-
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright (C) 2026 Michael Büsch <m@bues.ch>

use anyhow::{self as ah};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tokio::fs;

#[cfg(not(target_os = "windows"))]
const CONF_PATH: &str = "etc/periodic-audit.conf";
#[cfg(target_os = "windows")]
const CONF_PATH: &str = "periodic-audit.conf";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigWatch {
    pub paths: Vec<PathBuf>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigMail {
    pub disabled: Option<bool>,
    pub relay: Option<String>,
    pub subject: String,
    pub from: String,
    pub to: Vec<String>,
}

impl ConfigMail {
    pub fn disabled(&self) -> bool {
        self.disabled.unwrap_or(false)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigCargoAudit {
    pub exe: PathBuf,
    pub debug: Option<bool>,
    pub tries: Option<u32>,
    pub db: Option<PathBuf>,
}

impl ConfigCargoAudit {
    pub fn debug(&self) -> bool {
        self.debug.unwrap_or(false)
    }

    pub fn tries(&self) -> u32 {
        self.tries.unwrap_or(5)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub watch: ConfigWatch,
    pub mail: ConfigMail,
    pub cargo_audit: ConfigCargoAudit,
}

impl Config {
    pub fn get_default_path() -> PathBuf {
        // The build-time environment variable PERIODICAUDIT_CONF_PREFIX can be
        // used to give an additional prefix.
        let prefix = match option_env!("PERIODICAUDIT_CONF_PREFIX") {
            Some(env_prefix) => env_prefix,
            None => {
                #[cfg(not(target_os = "windows"))]
                let prefix = "/";
                #[cfg(target_os = "windows")]
                let prefix = "";
                prefix
            }
        };

        let mut path = PathBuf::new();
        path.push(prefix);
        path.push(CONF_PATH);

        path
    }

    pub async fn load(path: &Path) -> ah::Result<Self> {
        let content = fs::read_to_string(path).await?;
        let conf = toml::from_str(&content)?;
        Ok(conf)
    }
}

// vim: ts=4 sw=4 expandtab
