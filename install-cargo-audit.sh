#!/bin/sh
set -e
cargo install --force --root /opt/periodic-audit/ cargo-audit
cargo install --force --root /opt/periodic-audit/ cargo-auditable
