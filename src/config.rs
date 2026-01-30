// -*- coding: utf-8 -*-
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright (C) 2026 Michael Büsch <m@bues.ch>

use anyhow::{self as ah};
use serde::{Deserialize, Serialize};
use std::{
    num::NonZeroUsize,
    path::{Path, PathBuf},
};
use tokio::fs;

#[cfg(not(target_os = "windows"))]
const CONF_PATH: &str = "etc/periodic-audit/periodic-audit.conf";
#[cfg(target_os = "windows")]
const CONF_PATH: &str = "periodic-audit.conf";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigWatch {
    paths: Vec<PathBuf>,
}

impl ConfigWatch {
    pub fn paths(&self) -> &[PathBuf] {
        &self.paths
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigMail {
    disabled: Option<bool>,
    relay: Option<String>,
    subject: String,
    from: String,
    to: Vec<String>,
    max_concurrency: Option<NonZeroUsize>,
}

impl ConfigMail {
    pub fn disabled(&self) -> bool {
        self.disabled.unwrap_or(false)
    }

    pub fn relay(&self) -> Option<&str> {
        self.relay.as_deref()
    }

    pub fn subject(&self) -> &str {
        &self.subject
    }

    pub fn from(&self) -> &str {
        &self.from
    }

    pub fn to(&self) -> &[String] {
        &self.to
    }

    pub fn max_concurrency(&self) -> usize {
        self.max_concurrency.unwrap_or(1.try_into().unwrap()).into()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigCargoAudit {
    exe: PathBuf,
    debug: Option<bool>,
    tries: Option<NonZeroUsize>,
    db: Option<PathBuf>,
}

impl ConfigCargoAudit {
    pub fn exe(&self) -> &Path {
        &self.exe
    }

    pub fn debug(&self) -> bool {
        self.debug.unwrap_or(false)
    }

    pub fn tries(&self) -> usize {
        self.tries.unwrap_or(5.try_into().unwrap()).into()
    }

    pub fn db(&self) -> Option<&Path> {
        self.db.as_deref()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    watch: ConfigWatch,
    mail: ConfigMail,
    cargo_audit: ConfigCargoAudit,
}

impl Config {
    pub fn watch(&self) -> &ConfigWatch {
        &self.watch
    }

    pub fn mail(&self) -> &ConfigMail {
        &self.mail
    }

    pub fn cargo_audit(&self) -> &ConfigCargoAudit {
        &self.cargo_audit
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_minimal_config_and_defaults() {
        let toml = r#"
[watch]
paths = ["/foo"]

[mail]
subject = "subj"
from = "from@example.com"
to = ["to@example.com"]

[cargo_audit]
exe = "/usr/bin/cargo-audit"
        "#;
        let conf: Config = toml::from_str(toml).unwrap();
        assert_eq!(conf.watch.paths.len(), 1);
        assert_eq!(conf.watch.paths[0], Path::new("/foo"));
        assert_eq!(conf.mail.subject, "subj");
        assert_eq!(conf.mail.from, "from@example.com");
        assert_eq!(conf.mail.to, vec!["to@example.com".to_string()]);
        assert!(!conf.mail.disabled());
        assert_eq!(conf.mail.max_concurrency(), 1);
        assert_eq!(conf.cargo_audit.exe, Path::new("/usr/bin/cargo-audit"));
        assert!(!conf.cargo_audit.debug());
        assert_eq!(conf.cargo_audit.tries(), 5);
    }

    #[test]
    fn parse_full_config_and_non_default() {
        let toml = r#"
[watch]
paths = ["/foo", "/bar/biz"]

[mail]
disabled = true
relay = "smtp://smtp.example.com:587"
subject = "full subj"
from = "noreply@example.com"
to = ["one@example.com", "two@example.com"]
max_concurrency = 4

[cargo_audit]
exe = "/usr/local/bin/cargo-audit"
debug = true
tries = 10
db = "/var/lib/cargo-audit/db"
        "#;
        let conf: Config = toml::from_str(toml).unwrap();
        assert_eq!(conf.watch.paths.len(), 2);
        assert_eq!(conf.watch.paths[0], Path::new("/foo"));
        assert_eq!(conf.watch.paths[1], Path::new("/bar/biz"));

        assert_eq!(conf.mail.subject, "full subj");
        assert_eq!(conf.mail.from, "noreply@example.com");
        assert_eq!(
            conf.mail.to,
            vec!["one@example.com".to_string(), "two@example.com".to_string()]
        );
        assert!(conf.mail.disabled());
        assert_eq!(conf.mail.max_concurrency(), 4);
        assert_eq!(
            conf.mail.relay.as_deref(),
            Some("smtp://smtp.example.com:587")
        );

        assert_eq!(
            conf.cargo_audit.exe,
            Path::new("/usr/local/bin/cargo-audit")
        );
        assert!(conf.cargo_audit.debug());
        assert_eq!(conf.cargo_audit.tries(), 10);
        assert_eq!(
            conf.cargo_audit.db.as_deref(),
            Some(Path::new("/var/lib/cargo-audit/db"))
        );
    }
}

// vim: ts=4 sw=4 expandtab
