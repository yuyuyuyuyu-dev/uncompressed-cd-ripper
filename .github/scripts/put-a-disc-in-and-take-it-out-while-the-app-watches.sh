#!/usr/bin/env bash

source .github/scripts/boot-a-machine-with-a-drive.sh

ssh "${ssh_options[@]}" "${ssh_target[@]}" 'bash -euxo pipefail -s' <<'WATCH_THE_DRIVE'
export PATH="$HOME/.cargo/bin:$PATH"

"$HOME/app/.github/scripts/make-a-test-disc.py" /tmp

cd "$HOME/app/src-tauri"
cargo build --example watch

dbus-run-session -- bash -euxo pipefail -c '
    cdemu-daemon --bus session &
    for _ in $(seq 1 60); do cdemu status > /dev/null 2>&1 && break; sleep 1; done

    sudo "$HOME/app/src-tauri/target/debug/examples/watch" \
        < /dev/null > /tmp/watched.txt 2> /tmp/watching-errors.txt &
    watcher=$!
    for _ in $(seq 1 60); do grep -qx watching /tmp/watched.txt && break; sleep 1; done
    sleep 1

    cdemu load 0 /tmp/disc.cue

    for _ in $(seq 1 60); do
        [ "$(grep -c "^holding " /tmp/watched.txt)" -ge 1 ] && break
        sleep 1
    done

    cdemu unload 0

    for _ in $(seq 1 60); do
        [ "$(grep -c "^holding " /tmp/watched.txt)" -ge 2 ] && break
        sleep 1
    done

    sudo kill "$watcher" || true
'
WATCH_THE_DRIVE

scp "${ssh_options[@]}" -P "$ssh_port" \
    "ci@127.0.0.1:/tmp/watched.txt" "ci@127.0.0.1:/tmp/watching-errors.txt" "$workspace/"
ssh "${ssh_options[@]}" "${ssh_target[@]}" sudo poweroff || true
