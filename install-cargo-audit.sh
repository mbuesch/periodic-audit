#!/bin/sh
set -e
cargo install "$@" --root /opt/periodic-audit/ cargo-audit cargo-auditable
