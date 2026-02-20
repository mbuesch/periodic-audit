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
pub struct ConfigReportMail {
    disabled: Option<bool>,
    relay: Option<String>,
    subject: String,
    from: String,
    to: Vec<String>,
    max_concurrency: Option<NonZeroUsize>,
}

impl ConfigReportMail {
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
pub struct ConfigReportFile {
    disabled: Option<bool>,
    append: Option<bool>,
    path: PathBuf,
}

impl ConfigReportFile {
    pub fn disabled(&self) -> bool {
        self.disabled.unwrap_or(false)
    }

    pub fn append(&self) -> bool {
        self.append.unwrap_or(false)
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigReportCommand {
    disabled: Option<bool>,
    exe: PathBuf,
}

impl ConfigReportCommand {
    pub fn disabled(&self) -> bool {
        self.disabled.unwrap_or(false)
    }

    pub fn exe(&self) -> &Path {
        &self.exe
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    watch: ConfigWatch,
    cargo_audit: ConfigCargoAudit,
    #[serde(alias = "mail")] // backwards compatibility
    report_mail: Option<ConfigReportMail>,
    report_file: Option<ConfigReportFile>,
    report_command: Option<ConfigReportCommand>,
}

impl Config {
    pub fn watch(&self) -> &ConfigWatch {
        &self.watch
    }

    pub fn cargo_audit(&self) -> &ConfigCargoAudit {
        &self.cargo_audit
    }

    pub fn report_mail(&self) -> Option<&ConfigReportMail> {
        self.report_mail.as_ref()
    }

    pub fn report_file(&self) -> Option<&ConfigReportFile> {
        self.report_file.as_ref()
    }

    pub fn report_command(&self) -> Option<&ConfigReportCommand> {
        self.report_command.as_ref()
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
    fn parse_minimal_config() {
        let toml = r#"
[watch]
paths = ["/foo"]

[cargo_audit]
exe = "/usr/bin/cargo-audit"
        "#;
        let conf: Config = toml::from_str(toml).unwrap();
        assert_eq!(conf.watch.paths.len(), 1);
        assert_eq!(conf.watch.paths[0], Path::new("/foo"));
        assert_eq!(conf.cargo_audit.exe, Path::new("/usr/bin/cargo-audit"));
        assert!(!conf.cargo_audit.debug());
        assert_eq!(conf.cargo_audit.tries(), 5);
        assert!(conf.report_file().is_none());
        assert!(conf.report_command().is_none());
    }

    #[test]
    fn test_minimal_sections() {
        let toml = r#"
[watch]
paths = ["/foo"]

[report_mail]
subject = "full subj"
from = "noreply@example.com"
to = ["one@example.com"]

[cargo_audit]
exe = "/usr/local/bin/cargo-audit"

[report_file]
path = "/var/log/periodic-audit.log"

[report_command]
exe = "/usr/local/bin/report-handler"
        "#;
        let conf: Config = toml::from_str(toml).unwrap();
        assert_eq!(conf.watch.paths.len(), 1);
        assert_eq!(conf.watch.paths[0], Path::new("/foo"));

        let rm = conf.report_mail.as_ref().unwrap();
        assert_eq!(rm.subject(), "full subj");
        assert_eq!(rm.from(), "noreply@example.com");
        assert_eq!(rm.to(), ["one@example.com".to_string()]);
        assert!(!rm.disabled());
        assert_eq!(rm.max_concurrency(), 1);
        assert!(rm.relay().is_none());

        assert_eq!(
            conf.cargo_audit.exe,
            Path::new("/usr/local/bin/cargo-audit")
        );
        assert!(!conf.cargo_audit.debug());
        assert_eq!(conf.cargo_audit.tries(), 5);
        assert!(conf.cargo_audit.db.is_none());

        let rf = conf.report_file.as_ref().unwrap();
        assert!(!rf.disabled());
        assert!(!rf.append());
        assert_eq!(rf.path(), Path::new("/var/log/periodic-audit.log"));

        let rc = conf.report_command.as_ref().unwrap();
        assert!(!rc.disabled());
        assert_eq!(rc.exe(), Path::new("/usr/local/bin/report-handler"));
    }

    #[test]
    fn parse_full_config_and_non_default() {
        let toml = r#"
[watch]
paths = ["/foo", "/bar/biz"]

[mail] # using the old section name to test the alias
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

[report_file]
disabled = true
append = true
path = "/var/log/periodic-audit.log"

[report_command]
disabled = true
exe = "/usr/local/bin/report-handler"
        "#;
        let conf: Config = toml::from_str(toml).unwrap();
        assert_eq!(conf.watch.paths.len(), 2);
        assert_eq!(conf.watch.paths[0], Path::new("/foo"));
        assert_eq!(conf.watch.paths[1], Path::new("/bar/biz"));

        let rm = conf.report_mail.as_ref().unwrap();
        assert_eq!(rm.subject(), "full subj");
        assert_eq!(rm.from(), "noreply@example.com");
        assert_eq!(
            rm.to(),
            ["one@example.com".to_string(), "two@example.com".to_string()]
        );
        assert!(rm.disabled());
        assert_eq!(rm.max_concurrency(), 4);
        assert_eq!(rm.relay(), Some("smtp://smtp.example.com:587"));

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

        let rf = conf.report_file.as_ref().unwrap();
        assert!(rf.disabled());
        assert!(rf.append());
        assert_eq!(rf.path(), Path::new("/var/log/periodic-audit.log"));

        let rc = conf.report_command.as_ref().unwrap();
        assert!(rc.disabled());
        assert_eq!(rc.exe(), Path::new("/usr/local/bin/report-handler"));
    }
}

// vim: ts=4 sw=4 expandtab
