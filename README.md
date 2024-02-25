# Description
A simple toy point-to-point VPN based on Linux TUN interfaces and written in Rust. The VPN works on TCP and not on UDP because TCP connections can be easily handled with ssh redirections and other tools that require more work to be adapted when UDP is involved.

# How to test
Open multiple terminals as root, then apply following commands opportunely:

```bash
# add auxiliary network namespace and bring (other) localhost up
ip netns add NS1 && ip netns exec NS1 ip link set lo up

# add two veth interfaces to connect main and secondary network namespaces, then enable them
ip link add name V1 type veth peer name V2 netns NS1 && ip a add 172.19.66.1/24 dev V1 && ip netns exec NS1 ip a add 172.19.66.2/24 dev V2 && ip link set V1 up && ip netns exec NS1 ip link set V2 up
# eventually, to remove the interfaces and the network namespace:
#ip link del V1 && ip netns delete NS1

# run instance as server in the main namespace
RUST_BACKTRACE=1 cargo run -- --ifname tun0 --ifaddr 172.19.88.1 --netmask 24 --server --host 0.0.0.0 --port 1789
# run instances as clients inside secondary network namespace
RUST_BACKTRACE=1 ip netns exec NS1 cargo run -- --ifname tun1 --ifaddr 172.19.88.2 --netmask 24 --host 172.19.66.1 --port 1789
```
