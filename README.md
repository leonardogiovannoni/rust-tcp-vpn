# Description
A simple toy point-to-point VPN based on Linux TUN interfaces and written in Rust. The VPN works on TCP and not on UDP because TCP connections can be easily handled with ssh redirections and other tools that require more work to be adapted when UDP is involved.

