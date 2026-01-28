# Periodic cargo-audit with email reports

A tool to periodically audit Rust binaries for vulnerabilities using `cargo-audit`.

The report is sent via email to the configured recipients.

# systemd service

A systemd service and timer unit is provided to run the audit periodically.
It is recommended to use systemd, but you can also run the tool via cron or any other scheduler.

# Building

To build the project, ensure you have Rust and Cargo installed.
Then run:

```bash
./build.sh
```

# Installation

## Install periodic-audit

First create the unprivileged user and group that will run the service:

```bash
./create-user.sh
```

To install the `periodic-audit` binary and the systemd service, run:

```bash
./install.sh
```

## Install cargo-audit

If you don't have `cargo-audit` installed, you can install to `/opt/periodic-audit/bin` by running:

```bash
./install-cargo-audit.sh
```

Check and modify the path in the configuration file if you install `cargo-audit` to another custom location.

# License

Copyright (c) 2026 Michael Büsch <m@bues.ch>

Licensed under the Apache License version 2.0 or the MIT license, at your option.
