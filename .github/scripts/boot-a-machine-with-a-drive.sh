#!/usr/bin/env bash

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

for _ in $(seq 1 90); do
    if ssh "${ssh_options[@]}" "${ssh_target[@]}" true 2>/dev/null; then
        break
    fi
    sleep 10
done
ssh "${ssh_options[@]}" "${ssh_target[@]}" cloud-init status --wait

git archive HEAD | ssh "${ssh_options[@]}" "${ssh_target[@]}" 'mkdir -p app && tar x -C app'

ssh "${ssh_options[@]}" "${ssh_target[@]}" 'sudo bash -euxo pipefail -s' <<'PROVISION'
export DEBIAN_FRONTEND=noninteractive
printf 'Acquire::Retries "10";\n' > /etc/apt/apt.conf.d/99-retries
apt-get update
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
    libudev-dev \
    libwebkit2gtk-4.1-dev \
    libxdo-dev \
    pkg-config \
    cdemu-client \
    cdemu-daemon \
    vhba-dkms
modprobe vhba
chmod 666 /dev/vhba_ctl
PROVISION

ssh "${ssh_options[@]}" "${ssh_target[@]}" 'bash -euxo pipefail -s' <<'RUSTUP'
curl --proto '=https' --tlsv1.2 --silent --show-error --fail https://sh.rustup.rs \
    | sh -s -- -y --no-modify-path --default-toolchain none
RUSTUP

