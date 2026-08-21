#!/usr/bin/env bash

# Leaves the commands the drive was sent in drive.txt for the job to assert on.
#
# A machine rather than a container, because the drive is a kernel module and a
# container shares the kernel of its host. Debian rather than Ubuntu, because
# Debian packages that module and the alternative was a tarball nothing updates.
# Everything is built inside, so nothing crosses a boundary where two
# distributions have to agree about libraries.

set -euxo pipefail

workspace="$(pwd)"
image=debian-13-generic-amd64.qcow2
ssh_port=2222
ssh_options=(
    -o StrictHostKeyChecking=no
    -o UserKnownHostsFile=/dev/null
    -o LogLevel=ERROR
    -o ConnectTimeout=5
    -i "$workspace/vm-key"
)
ssh_target=(-p "$ssh_port" "ci@127.0.0.1")

curl -fsSLo "$image" "https://cloud.debian.org/images/cloud/trixie/latest/$image"
# A Rust build with Tauri in it needs more room than the image arrives with.
qemu-img resize "$image" +20G

ssh-keygen -t ed25519 -N '' -f "$workspace/vm-key" -C ci

cat > user-data <<EOF
#cloud-config
users:
  - name: ci
    sudo: 'ALL=(ALL) NOPASSWD:ALL'
    shell: /bin/bash
    ssh_authorized_keys:
      - $(cat "$workspace/vm-key.pub")
growpart:
  mode: auto
  devices: ['/']
EOF
printf 'instance-id: disc\nlocal-hostname: disc\n' > meta-data
cloud-localds seed.iso user-data meta-data

qemu-system-x86_64 \
    -enable-kvm \
    -cpu host \
    -m 8192 \
    -smp "$(nproc)" \
    -drive "file=$image,if=virtio" \
    -drive "file=seed.iso,if=virtio,format=raw" \
    -netdev "user,id=net0,hostfwd=tcp::$ssh_port-:22" \
    -device virtio-net-pci,netdev=net0 \
    -display none \
    -daemonize \
    -pidfile "$workspace/qemu.pid" \
    -serial "file:$workspace/vm-console.log"

# cloud-init has to finish before the account being waited for exists.
for _ in $(seq 1 90); do
    if ssh "${ssh_options[@]}" "${ssh_target[@]}" true 2>/dev/null; then
        break
    fi
    sleep 10
done
ssh "${ssh_options[@]}" "${ssh_target[@]}" cloud-init status --wait

git archive HEAD | ssh "${ssh_options[@]}" "${ssh_target[@]}" 'mkdir -p app && tar x -C app'

# The Tauri libraries are here because the crate links them whether or not a
# window is ever opened.
ssh "${ssh_options[@]}" "${ssh_target[@]}" 'sudo bash -euxo pipefail -s' <<'PROVISION'
export DEBIAN_FRONTEND=noninteractive
apt-get update
# Before the module, and for the running kernel rather than the newest, or
# dkms builds it for a kernel this machine is not on.
apt-get install --yes --no-install-recommends "linux-headers-$(uname -r)"
apt-get install --yes --no-install-recommends \
    build-essential \
    curl \
    file \
    libayatana-appindicator3-dev \
    libcdio-cdda-dev \
    libcdio-dev \
    libcdio-paranoia-dev \
    libclang-dev \
    librsvg2-dev \
    libssl-dev \
    libwebkit2gtk-4.1-dev \
    libxdo-dev \
    pkg-config \
    cdemu-client \
    cdemu-daemon \
    libsamplerate0 \
    vhba-dkms
modprobe vhba
# The daemon runs as the account this is driven from, and the module's own rule
# hands the control device to a group that account is not in.
chmod 666 /dev/vhba_ctl
PROVISION

# No toolchain named: rust-toolchain.toml is, and rustup reads it.
ssh "${ssh_options[@]}" "${ssh_target[@]}" 'bash -euxo pipefail -s' <<'RUSTUP'
curl --proto '=https' --tlsv1.2 --silent --show-error --fail https://sh.rustup.rs \
    | sh -s -- -y --no-modify-path --default-toolchain none
RUSTUP

ssh "${ssh_options[@]}" "${ssh_target[@]}" 'bash -euxo pipefail -s' <<'USE_THE_DRIVE'
export PATH="$HOME/.cargo/bin:$PATH"

# Read-only is a second line, not the assertion: a command is written down when
# it is dispatched, before anything decides whether to honour it.
head -c $((2352 * 750)) /dev/urandom > /tmp/disc.bin
printf 'FILE "disc.bin" BINARY\n  TRACK 01 AUDIO\n    INDEX 01 00:00:00\n' > /tmp/disc.cue
chmod 444 /tmp/disc.bin /tmp/disc.cue

sudo sh -c 'echo 1 > /sys/kernel/debug/tracing/events/scsi/scsi_dispatch_cmd_start/enable'
sudo sh -c ': > /sys/kernel/debug/tracing/trace'

# Every Rust test and not only the ripping, because the rule is about the whole
# app. The device is left writable on purpose: nothing was written to a drive
# that could not be written to would be worth nothing to say.
dbus-run-session -- bash -euxo pipefail -c '
    cdemu-daemon --bus session &
    sleep 5
    cdemu load 0 /tmp/disc.cue
    sleep 5
    sudo chmod a+rw /dev/sr0

    cdemu status

    cd "$HOME/app/src-tauri"
    cargo test --all-features
    cargo run --example rip -- /dev/sr0 "$HOME/ripped"

    # A rip that found no tracks writes nothing and says it went fine, and
    # everything after it then reports on a drive that was never read.
    test -s "$HOME/ripped/01.flac"
'

sudo cat /sys/kernel/debug/tracing/trace > /tmp/trace.txt

# Other drives are somebody else's business. Finding nothing is allowed here
# because the job asserts on that, and a short recording is worth looking at.
host=$(readlink -f /sys/block/sr0/device | grep -oE 'host[0-9]+' | head -1 | tr -dc '0-9')
grep "host_no=$host " /tmp/trace.txt > /tmp/drive.txt || true
USE_THE_DRIVE

scp "${ssh_options[@]}" -P "$ssh_port" \
    "ci@127.0.0.1:/tmp/drive.txt" "ci@127.0.0.1:/tmp/trace.txt" "$workspace/"
ssh "${ssh_options[@]}" "${ssh_target[@]}" sudo poweroff || true
