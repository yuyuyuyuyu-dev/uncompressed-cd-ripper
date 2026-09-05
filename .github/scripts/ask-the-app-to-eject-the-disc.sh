#!/usr/bin/env bash

source .github/scripts/boot-a-machine-with-a-drive.sh

ssh "${ssh_options[@]}" "${ssh_target[@]}" 'bash -euxo pipefail -s' <<'EJECT_THE_DISC'
export PATH="$HOME/.cargo/bin:$PATH"

"$HOME/app/.github/scripts/make-a-test-disc.py" /tmp

cd "$HOME/app/src-tauri"
cargo build --example eject

dbus-run-session -- bash -euxo pipefail -c '
    cdemu-daemon --bus session &
    for _ in $(seq 1 60); do cdemu status > /dev/null 2>&1 && break; sleep 1; done

    cdemu load 0 /tmp/disc.cue

    for _ in $(seq 1 60); do cdemu status | grep -q /tmp/disc.cue && break; sleep 1; done
    cdemu status > /tmp/drive-with-the-disc-in.txt

    sudo "$HOME/app/src-tauri/target/debug/examples/eject" \
        < /dev/null > /tmp/ejected.txt 2> /tmp/ejecting-errors.txt

    for _ in $(seq 1 60); do cdemu status | grep -q /tmp/disc.cue || break; sleep 1; done
    cdemu status > /tmp/drive-once-it-was-ejected.txt
'
EJECT_THE_DISC

scp "${ssh_options[@]}" -P "$ssh_port" \
    "ci@127.0.0.1:/tmp/ejected.txt" \
    "ci@127.0.0.1:/tmp/ejecting-errors.txt" \
    "ci@127.0.0.1:/tmp/drive-with-the-disc-in.txt" \
    "ci@127.0.0.1:/tmp/drive-once-it-was-ejected.txt" \
    "$workspace/"
ssh "${ssh_options[@]}" "${ssh_target[@]}" sudo poweroff || true
