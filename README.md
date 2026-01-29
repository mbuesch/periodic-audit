# Periodic cargo-audit with email reports

A tool to periodically audit Rust binaries for vulnerabilities using
[cargo-audit](https://crates.io/crates/cargo-audit).

The report is sent via email to the configured recipients.

# systemd service

A systemd service and timer unit is provided to run the audit periodically.
It is recommended to use systemd, but you can also run the tool via cron or any other scheduler.

The systemd service and timer units will be installed by the `install.sh` script (see below).

# Building

To build the project, ensure you have Rust and Cargo installed.
Then run:

```bash
./build.sh
```

# Installation

## Install cargo-audit and cargo-auditable

If you don't have `cargo-audit` or `cargo-auditable` installed, you can install them to `/opt/periodic-audit/bin` by running:

```bash
./install-cargo-audit.sh
```

Check and modify the path in the `periodic-audit.conf` configuration file if you install `cargo-audit` to another custom location.

After installation make sure `/opt/periodic-audit/bin` is in your `$PATH`.

## Install periodic-audit

First create the unprivileged user and group that will run the service:

```bash
./create-user.sh
```

Creating the user and group only has to be done once.
The script will delete any existing user and group with the same name before creating them anew and therefore can result in different UIDs and GIDs on multiple runs.

To install the `periodic-audit` binary and the systemd service, run:

```bash
./install.sh
```

# Making your binaries auditable

It is **highly recommended** to build all your Rust binaries that you want to audit with the
[cargo-auditable](https://crates.io/crates/cargo-auditable)
tool.

This tool adds the necessary metadata to your binaries to allow `cargo-audit` to analyze them properly.

# License

Copyright (c) 2026 Michael Büsch <m@bues.ch>

Licensed under the Apache License version 2.0 or the MIT license, at your option.
