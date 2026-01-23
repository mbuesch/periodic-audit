#!/bin/sh
# -*- coding: utf-8 -*-

basedir="$(realpath "$0" | xargs dirname)"

. "$basedir/scripts/lib.sh"

entry_checks()
{
    [ -d "$target" ] || die "periodic-audit is not built! Run ./build.sh"

    [ "$(id -u)" = "0" ] || die "Must be root to install periodic-audit."

    if ! grep -qe '^periodic-audit:' /etc/passwd; then
        die "The system user 'periodic-audit' does not exist in /etc/passwd. Please run ./create-user.sh"
    fi
    if ! grep -qe '^periodic-audit:' /etc/group; then
        die "The system group 'periodic-audit' does not exist in /etc/group. Please run ./create-user.sh"
    fi
}

install_dirs()
{
    do_install \
        -o root -g root -m 0755 \
        -d /opt/periodic-audit/bin

    do_install \
        -o root -g root -m 0755 \
        -d /opt/periodic-audit/etc/periodic-audit
}

install_conf()
{
    if [ -e /opt/periodic-audit/etc/periodic-audit/periodic-audit.conf ]; then
        do_chown root:periodic-audit /opt/periodic-audit/etc/periodic-audit/periodic-audit.conf
        do_chmod 0640 /opt/periodic-audit/etc/periodic-audit/periodic-audit.conf
    else
        do_install \
            -o root -g periodic-audit -m 0640 \
            "$basedir/periodic-audit.conf" \
            /opt/periodic-audit/etc/periodic-audit/periodic-audit.conf
    fi
}

install_periodic_audit()
{
    do_install \
        -o root -g root -m 0755 \
        "$target/periodic-audit" \
        /opt/periodic-audit/bin/

    do_install \
        -o root -g root -m 0644 \
        "$basedir/periodic-audit.service" \
        /etc/systemd/system/

    do_install \
        -o root -g root -m 0644 \
        "$basedir/periodic-audit.timer" \
        /etc/systemd/system/

    do_systemctl enable periodic-audit.service
    do_systemctl enable periodic-audit.timer
}

release="release"
while [ $# -ge 1 ]; do
    case "$1" in
        --debug|-d)
            release="debug"
            ;;
        --release|-r)
            release="release"
            ;;
        *)
            die "Invalid option: $1"
            ;;
    esac
    shift
done
target="$basedir/target/$release"

entry_checks
stop_services
disable_services
install_dirs
install_conf
install_periodic_audit
start_services

# vim: ts=4 sw=4 expandtab
