#!/usr/bin/env bash
#
# This script undoes the changes made by the setup script:
#  1. Removes the 'NS1' network namespace.
#  2. Deletes the veth pair (V1 and V2).
#  3. Removes the 'cap_net_admin' capability from the Rust binary.
#  4. (Optional) Removes the TUN interface if you manually created it.

set -e

# Path to the Rust binary we used in the original script
BIN="target/debug/rust-tcp-vpn"

echo "==== Removing setcap from the Rust binary ===="
if [ -f "$BIN" ]; then
    sudo setcap -r "$BIN" || true
else
    echo "Binary $BIN not found. Skipping capability removal."
fi

echo "==== Deleting the veth pair (V1) if it exists ===="
if ip link show V1 &>/dev/null; then
    sudo ip link del V1
else
    echo "Veth interface V1 does not exist. Skipping."
fi

echo "==== Deleting the NS1 network namespace if it exists ===="
if ip netns list | grep -q "NS1"; then
    sudo ip netns del NS1
else
    echo "Network namespace NS1 does not exist. Skipping."
fi

# If you uncommented lines in the original script that manually created tun0, 
# you can also remove it by uncommenting the lines below:

# echo "==== Removing the tun0 interface if it exists ===="
# if ip link show tun0 &>/dev/null; then
#     sudo ip link del tun0
# else
#     echo "tun0 interface does not exist. Skipping."
# fi

echo "Cleanup complete."
exit 0

