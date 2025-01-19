#!/usr/bin/env bash
#
# This script creates a network namespace (NS1), connects it to the host
# via a veth pair, assigns IP addresses, and then builds and runs a Rust TCP VPN server.


if [ "$1" != "server" ] && [ "$1" != "client" ]; then
    echo "Usage: $0 [server|client]"
    exit 1
fi

###############################################################################
# 1. Create and configure the network namespace (NS1)
###############################################################################

# Create the 'NS1' namespace
sudo ip netns add NS1

# Enable the loopback interface inside NS1
sudo ip netns exec NS1 ip link set lo up

# Create a veth pair: 'V1' in the host namespace and 'V2' in NS1
sudo ip link add name V1 type veth peer name V2 netns NS1

# Assign IP addresses to both sides of the veth pair
sudo ip address add 172.19.66.1/24 dev V1
sudo ip netns exec NS1 ip address add 172.19.66.2/24 dev V2

# Bring both veth interfaces up
sudo ip link set V1 up
sudo ip netns exec NS1 ip link set V2 up

###############################################################################
# 2. Build the Rust project
###############################################################################

cargo build
if [ $? -ne 0 ]; then
    echo "Cargo build failed. Exiting."
    exit 1
fi

# Uncomment the following line if you want a release build:
# cargo build --release

###############################################################################
# 3. Configure environment and capabilities
###############################################################################

# Enable backtraces for Rust (helpful for debugging)
export RUST_LIB_BACKTRACE=1

# The binary we just built (adjust if you built with --release)
BIN=target/debug/rust-tcp-vpn

# Allow the binary to configure network interfaces without needing sudo
sudo setcap cap_net_admin=eip "$BIN"

###############################################################################
# 4. Run the VPN server
###############################################################################

# Run the Rust TCP VPN server

# if argument passed is equual to "server"

if [ "$1" == "server" ]; then
    "$BIN" --ifname tun0 \
        --ifaddr 172.19.88.1 \
        --netmask 24 \
        --server --host 0.0.0.0 --port 1789
fi
if [ "$1" == "client" ]; then
    sudo ip netns exec NS1 $BIN --ifname tun1 \
        --ifaddr 172.19.88.2 \
        --netmask 24 \
        --host 172.19.66.1 \
        --port 1789
fi
exit 0
