#!/usr/bin/env bash

set -euxo pipefail

boot_a_machine_with_a_drive() {
    source .github/scripts/boot-a-machine-with-a-drive.sh
}

use_the_drive_while_the_kernel_writes_down_what_it_is_asked() {
    ssh "${ssh_options[@]}" "${ssh_target[@]}" dbus-run-session -- \
        '$HOME/app/.github/scripts/rip-and-write-down-what-the-drive-was-asked/vm-main.sh'
}

bring_back_what_the_kernel_wrote() {
    scp "${ssh_options[@]}" -P "$ssh_port" \
        "ci@127.0.0.1:/tmp/drive.txt" "ci@127.0.0.1:/tmp/trace.txt" "$workspace/"
}

shut_the_machine_down() {
    ssh "${ssh_options[@]}" "${ssh_target[@]}" sudo poweroff || true
}

main() {
    boot_a_machine_with_a_drive
    use_the_drive_while_the_kernel_writes_down_what_it_is_asked
    bring_back_what_the_kernel_wrote
    shut_the_machine_down
}

main
