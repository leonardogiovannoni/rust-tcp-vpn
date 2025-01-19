
sudo ip netns add NS1
sudo ip netns exec NS1 ip link set lo up
sudo ip link add name V1 type veth peer name V2 netns NS1
sudo ip a add 172.19.66.1/24 dev V1
sudo ip netns exec NS1 ip a add 172.19.66.2/24 dev V2
sudo ip link set V1 up
sudo ip netns exec NS1 ip link set V2 up

cargo build
cargo build --release
ext=$?
if [ $ext -ne 0 ]; then
    exit $ext
fi
export RUST_LIB_BACKTRACE=1
BIN=target/debug/rust-tcp-vpn
sudo setcap cap_net_admin=eip $BIN

sudo ip addr add 172.19.88.1/24 dev tun0
sudo ip link set up dev tun0

export RUST_LIB_BACKTRACE=1
$BIN --ifname tun0 \
    --ifaddr 172.19.88.1 \
    --netmask 24 \
    --server --host 0.0.0.0 --port 1789
exit
#server_pid=$!

#trap "kill $server_pid" INT TERM

#wait $client_pid
# sudo ip netns exec NS1 $BIN --ifname tun1 \
#     --ifaddr 172.19.88.2 \
#     --netmask 24 \
#     --host 172.19.66.1 \
#     --port 1789
#
# client_pid=$!
#
# trap "kill $client_pid" INT TERM
#
# wait $server_pid $client_pid
