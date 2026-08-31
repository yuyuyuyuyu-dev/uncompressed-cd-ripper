#!/usr/bin/env bash

# Runs everything the app does with a drive, against a drive in a machine of its
# own, and brings back every command that drive was sent. The job that calls
# this is where they are judged; this only fetches them.

source .github/scripts/boot-a-machine-with-a-drive.sh

ssh "${ssh_options[@]}" "${ssh_target[@]}" 'bash -euxo pipefail -s' <<'USE_THE_DRIVE'
export PATH="$HOME/.cargo/bin:$PATH"

# Read-only is a second line, not the assertion: a command is written down when
# it is dispatched, before anything decides whether to honour it.
"$HOME/app/.github/scripts/make-a-test-disc.py" /tmp
chmod 444 /tmp/disc.bin /tmp/disc.cue

sudo sh -c 'echo 1 > /sys/kernel/debug/tracing/events/scsi/scsi_dispatch_cmd_start/enable'
sudo sh -c ': > /sys/kernel/debug/tracing/trace'

# Every Rust test and not only the ripping, because the rule is about the whole
# app. The device is left writable on purpose: nothing was written to a drive
# that could not be written to would be worth nothing to say.
dbus-run-session -- bash -euxo pipefail -c '
    cdemu-daemon --bus session &
    for _ in $(seq 1 60); do cdemu status > /dev/null 2>&1 && break; sleep 1; done

    cdemu load 0 /tmp/disc.cue
    for _ in $(seq 1 60); do cdemu status | grep -q /tmp/disc.cue && break; sleep 1; done

    # Asked for rather than assumed: QEMU gives the machine an empty CD-ROM at
    # /dev/sr0, and reading that one is what let this job pass while the app
    # never touched a disc. The name arrives a moment after the disc does.
    for _ in $(seq 1 60); do
        drive=$(cdemu device-mapping | sed -n "s|^0[[:space:]][[:space:]]*\([^[:space:]][^[:space:]]*\).*|\1|p") || true
        [ -n "$drive" ] && break
        sleep 1
    done
    [ -n "$drive" ]
    sudo chmod a+rw "$drive"
    cdemu status

    # Noted while the drive still exists. Stopping the daemon takes it away,
    # and the recording is filtered down to it after that has happened.
    readlink -f "/sys/block/$(basename "$drive")/device" \
        | grep -oE "host[0-9]+" | head -1 | tr -dc "0-9" > /tmp/drive-host

    cd "$HOME/app/src-tauri"
    cargo test --all-features
    cargo run --example rip -- --disc "$drive" -o "$HOME/ripped"

    # A rip that found no tracks writes nothing and says it went fine, and
    # everything after it then reports on a drive that was never read.
    test -s "$HOME/ripped/01.flac"
'

sudo cat /sys/kernel/debug/tracing/trace > /tmp/trace.txt

# Other drives are somebody else's business. Finding nothing is allowed here
# because the job asserts on that, and a short recording is worth looking at.
grep "host_no=$(cat /tmp/drive-host) " /tmp/trace.txt > /tmp/drive.txt || true
USE_THE_DRIVE

scp "${ssh_options[@]}" -P "$ssh_port" \
    "ci@127.0.0.1:/tmp/drive.txt" "ci@127.0.0.1:/tmp/trace.txt" "$workspace/"
ssh "${ssh_options[@]}" "${ssh_target[@]}" sudo poweroff || true
