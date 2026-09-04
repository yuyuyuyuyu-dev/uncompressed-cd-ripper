#!/usr/bin/env bash

source .github/scripts/boot-a-machine-with-a-drive.sh

ssh "${ssh_options[@]}" "${ssh_target[@]}" 'bash -euxo pipefail -s' <<'USE_THE_DRIVE'
export PATH="$HOME/.cargo/bin:$PATH"

"$HOME/app/.github/scripts/make-a-test-disc.py" /tmp
chmod 444 /tmp/disc.bin /tmp/disc.cue

sudo sh -c 'echo 1 > /sys/kernel/debug/tracing/events/scsi/scsi_dispatch_cmd_start/enable'
sudo sh -c ': > /sys/kernel/debug/tracing/trace'

dbus-run-session -- bash -euxo pipefail -c '
    cdemu-daemon --bus session &
    for _ in $(seq 1 60); do cdemu status > /dev/null 2>&1 && break; sleep 1; done

    cdemu load 0 /tmp/disc.cue
    for _ in $(seq 1 60); do cdemu status | grep -q /tmp/disc.cue && break; sleep 1; done

    for _ in $(seq 1 60); do
        drive=$(cdemu device-mapping | sed -n "s|^0[[:space:]][[:space:]]*\([^[:space:]][^[:space:]]*\).*|\1|p") || true
        [ -n "$drive" ] && break
        sleep 1
    done
    [ -n "$drive" ]
    sudo chmod a+rw "$drive"
    cdemu status

    readlink -f "/sys/block/$(basename "$drive")/device" \
        | grep -oE "host[0-9]+" | head -1 | tr -dc "0-9" > /tmp/drive-host

    cd "$HOME/app/src-tauri"
    cargo test --all-features
    cargo run --example rip -- --disc "$drive" -o "$HOME/ripped"

    test -s "$HOME/ripped/01.flac"
'

sudo cat /sys/kernel/debug/tracing/trace > /tmp/trace.txt

grep "host_no=$(cat /tmp/drive-host) " /tmp/trace.txt > /tmp/drive.txt || true
USE_THE_DRIVE

scp "${ssh_options[@]}" -P "$ssh_port" \
    "ci@127.0.0.1:/tmp/drive.txt" "ci@127.0.0.1:/tmp/trace.txt" "$workspace/"
ssh "${ssh_options[@]}" "${ssh_target[@]}" sudo poweroff || true
