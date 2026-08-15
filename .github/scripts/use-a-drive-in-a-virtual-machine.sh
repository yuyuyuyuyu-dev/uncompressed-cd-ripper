#!/usr/bin/env bash

# Runs everything the app does with a drive, against a drive that is not real,
# with the kernel writing down every command that reaches it. Leaves those
# commands in drive.txt for the job to assert on.
#
# A virtual machine rather than a container. The drive is a kernel module, and
# a container shares the kernel of the machine it runs on, so a container would
# not have got us a drive of our own: the module would still have to be built
# for the runner's kernel and loaded into it. Inside a machine of our own, the
# kernel the module is built for and the kernel it is loaded into are the same
# one by construction.
#
# Debian rather than the runner's Ubuntu because Debian packages that module.
# Nobody packages it for Ubuntu, and the alternative was fetching a tarball and
# building it, which is a version nothing updates and a download nothing
# promises to keep serving.
#
# Everything is built inside the machine rather than carried in from outside.
# It costs a full build every run, and it buys the only thing worth having when
# a job cannot be tried by hand: nothing crosses a boundary where two
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
# The image arrives sized for itself. A Rust build with Tauri in it needs more.
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

# Booting and settling takes minutes rather than seconds, and cloud-init has to
# finish before the account being waited for exists.
for _ in $(seq 1 90); do
    if ssh "${ssh_options[@]}" "${ssh_target[@]}" true 2>/dev/null; then
        break
    fi
    sleep 10
done
ssh "${ssh_options[@]}" "${ssh_target[@]}" cloud-init status --wait

git archive HEAD | ssh "${ssh_options[@]}" "${ssh_target[@]}" 'mkdir -p app && tar x -C app'

# The drive, the daemon that answers for it, and what the app needs to build.
# The Tauri libraries are here because the crate links them whether or not a
# window is ever opened.
ssh "${ssh_options[@]}" "${ssh_target[@]}" 'sudo bash -euxo pipefail -s' <<'PROVISION'
export DEBIAN_FRONTEND=noninteractive
apt-get update
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
    vhba-dkms
# Built by dkms against this machine's kernel when it was installed, which is
# the kernel it is going in to.
modprobe vhba
PROVISION

# No toolchain named here: rust-toolchain.toml is, and rustup reads it on the
# first cargo call.
ssh "${ssh_options[@]}" "${ssh_target[@]}" 'bash -euxo pipefail -s' <<'RUSTUP'
curl --proto '=https' --tlsv1.2 --silent --show-error --fail https://sh.rustup.rs \
    | sh -s -- -y --no-modify-path --default-toolchain none
RUSTUP

ssh "${ssh_options[@]}" "${ssh_target[@]}" 'bash -euxo pipefail -s' <<'USE_THE_DRIVE'
export PATH="$HOME/.cargo/bin:$PATH"

# The disc cannot be written to. That is a second line and not the assertion:
# what follows records what the app sent to the drive, and a command is written
# down when it is dispatched, before anything decides whether to honour it.
head -c $((2352 * 750)) /dev/urandom > /tmp/disc.bin
printf 'FILE "disc.bin" BINARY\n  TRACK 01 AUDIO\n    INDEX 01 00:00:00\n' > /tmp/disc.cue
chmod 444 /tmp/disc.bin /tmp/disc.cue

sudo sh -c 'echo 1 > /sys/kernel/debug/tracing/events/scsi/scsi_dispatch_cmd_start/enable'
sudo sh -c ': > /sys/kernel/debug/tracing/trace'

# Every test the Rust side has, and not only the ripping. The rule is about the
# whole app, and one that covers only the code somebody remembered to point it
# at is not much of a rule.
#
# The device is left writable on purpose. Nothing was written to a drive that
# could not be written to would be worth nothing to say: the app has to be free
# to write and decline.
dbus-run-session -- bash -euxo pipefail -c '
    cdemu-daemon --bus session &
    sleep 5
    cdemu load 0 /tmp/disc.cue
    sleep 5
    sudo chmod a+rw /dev/sr0

    cd "$HOME/app/src-tauri"
    cargo test --all-features
    cargo run --example rip -- /dev/sr0 "$HOME/ripped"
'

sudo cat /sys/kernel/debug/tracing/trace > /tmp/trace.txt

# Other drives are somebody else's business. This is the one the app was given.
#
# Finding nothing is allowed to happen here rather than stopping the script,
# because an empty recording is something the job has an assertion for, and a
# recording that stopped early is worth carrying home to look at.
host=$(readlink -f /sys/block/sr0/device | grep -oE 'host[0-9]+' | head -1 | tr -dc '0-9')
grep "host_no=$host " /tmp/trace.txt > /tmp/drive.txt || true
USE_THE_DRIVE

scp "${ssh_options[@]}" -P "$ssh_port" \
    "ci@127.0.0.1:/tmp/drive.txt" "ci@127.0.0.1:/tmp/trace.txt" "$workspace/"
ssh "${ssh_options[@]}" "${ssh_target[@]}" sudo poweroff || true
