#!/usr/bin/env bash

set -euxo pipefail
shopt -s inherit_errexit

export PATH="$HOME/.cargo/bin:$PATH"

have_the_kernel_write_down_every_command_the_drive_is_sent() {
    sudo sh -c 'echo 1 > /sys/kernel/debug/tracing/events/scsi/scsi_dispatch_cmd_start/enable'
    sudo sh -c ': > /sys/kernel/debug/tracing/trace'
}

put_a_read_only_test_disc_in_the_drive() {
    "$HOME/app/.github/scripts/make-a-test-disc.py" /tmp
    chmod 444 /tmp/disc.bin /tmp/disc.cue

    cdemu-daemon --bus session &
    for _ in $(seq 1 60); do cdemu status > /dev/null 2>&1 && break; sleep 1; done

    cdemu load 0 /tmp/disc.cue
    for _ in $(seq 1 60); do cdemu status | grep -q /tmp/disc.cue && break; sleep 1; done

    cdemu status
}

find_out_which_drive_the_disc_went_into() {
    local drive=''

    for _ in $(seq 1 60); do
        drive=$(cdemu device-mapping | sed -n 's|^0[[:space:]][[:space:]]*\([^[:space:]][^[:space:]]*\).*|\1|p') || true
        [ -n "$drive" ] && break
        sleep 1
    done
    [ -n "$drive" ]

    echo "$drive"
}

use_the_drive_every_way_the_app_can() {
    local drive="$1"

    sudo chmod a+rw "$drive"

    cd "$HOME/app/src-tauri"
    # Checks as a black box that the functions exposed as Tauri commands never send the drive a write command.
    cargo test --all-features
    cargo run --example rip -- --disc "$drive" -o "$HOME/ripped"

    test -s "$HOME/ripped/01.flac"
}

keep_the_commands_this_drive_was_sent() {
    local drive="$1"
    local host

    host=$(readlink -f "/sys/block/$(basename "$drive")/device" \
        | grep -oE 'host[0-9]+' | head -1 | tr -dc '0-9')

    sudo cat /sys/kernel/debug/tracing/trace > /tmp/trace.txt
    grep "host_no=$host " /tmp/trace.txt > /tmp/drive.txt || true
}

main() {
    local drive

    have_the_kernel_write_down_every_command_the_drive_is_sent
    put_a_read_only_test_disc_in_the_drive
    drive="$(find_out_which_drive_the_disc_went_into)"
    use_the_drive_every_way_the_app_can "$drive"
    keep_the_commands_this_drive_was_sent "$drive"
}

main
