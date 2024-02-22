
//#define _BSD_SOURCE

#include <arpa/inet.h>
#include <fcntl.h>
#include <linux/if_tun.h>
//#include <linux/in.h>
#include <net/if.h>
//#include <socket.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/ioctl.h>
#include <sys/socket.h>
#include <sys/stat.h>
#include <sys/types.h>
#include <unistd.h>
#include <sys/socket.h>
#include <netinet/in.h>

// https://www.kernel.org/doc/Documentation/networking/tuntap.txt
void set_interface_name(int if_fd, const char *ifname)
{
    struct ifreq ifr = {};

    memset(&ifr, 0, sizeof(ifr));
    strncpy(ifr.ifr_name, ifname, IFNAMSIZ);
    ifr.ifr_flags = IFF_TUN | IFF_NO_PI;

    int err = ioctl(if_fd, TUNSETIFF, (void *) &ifr);
    if (err < 0)
    {
        perror("ERROR TUNSETIFF");
        exit(EXIT_FAILURE);
    }
}


void set_interface_address(int if_fd, const char *ifname, const char *addr, int netmask)
{
    (void)if_fd;
    struct ifreq ifr;

    struct in_addr ipv4;
    const struct in_addr nmask = {
        htonl((-1) ^ ((1<<(32-netmask))-1))
    };
    struct sockaddr_in ipv4_addr;

    if (netmask <= 0 || netmask >= 32)
    {
        fprintf(stderr, "Invalid netmask %d\n", netmask);
        exit(EXIT_SUCCESS);
    }

    memset(&ipv4, 0, sizeof(ipv4));
    if (inet_pton(AF_INET, addr, (void *)&ipv4.s_addr) != 1)
    {
        perror("Failed to parse address with inet_pton");
        exit(EXIT_FAILURE);
    }

    int udp_socket = socket(AF_INET, SOCK_DGRAM, 0);
    if (udp_socket < 0)
    {
        perror("Failed to create udp_socket");
        exit(EXIT_FAILURE);
    }

    memset(&ifr, 0, sizeof(ifr));
    strncpy(ifr.ifr_name, ifname, IFNAMSIZ);
    // set interface address
    memset(&ipv4_addr, 0, sizeof(ipv4_addr));
    ipv4_addr.sin_family = AF_INET;
    ipv4_addr.sin_addr = ipv4;
    memcpy(&ifr.ifr_addr, &ipv4_addr, sizeof(ipv4_addr));
    if (ioctl(udp_socket, SIOCSIFADDR, &ifr) < 0)
    {
        perror("Failed to set netaddr");
        if (close(udp_socket))
        {
            perror("DOUBLE FAULT close(udp_socket)");
        }
        exit(EXIT_FAILURE);
    }
    // set interface netmask
    memset(&ifr, 0, sizeof(ifr));
    strncpy(ifr.ifr_name, ifname, IFNAMSIZ);
    memset(&ipv4_addr, 0, sizeof(ipv4_addr));
    ipv4_addr.sin_family = AF_INET;
    ipv4_addr.sin_addr = nmask;
    memcpy(&ifr.ifr_addr, &ipv4_addr, sizeof(ipv4_addr));
    if (ioctl(udp_socket, SIOCSIFNETMASK, &ifr) < 0)
    {
        perror("Failed to set netmask");
        if (close(udp_socket))
        {
            perror("DOUBLE FAULT close(udp_socket)");
        }
        exit(EXIT_FAILURE);
    }

    if (close(udp_socket) < 0)
    {
        perror("Failed to close(udp_socket)");
        exit(EXIT_FAILURE);
    }
}

void set_interface_up(int if_fd, const char *ifname)
{
    (void)if_fd;
    // https://stackoverflow.com/questions/11679514/what-is-the-difference-between-iff-up-and-iff-running
    // difference IFF_UP vs IFF_RUNNING
    //  IFF_UP      =>  bring the interface up.
    //  IFF_RUNNING =>  check the interface status.
    struct ifreq ifr;
    memset(&ifr, 0, sizeof(ifr));
    strncpy(ifr.ifr_name, ifname, IFNAMSIZ);

    int udp_socket = socket(AF_INET, SOCK_DGRAM, 0);
    if (udp_socket < 0)
    {
        perror("Failed to create udp_socket");
        exit(EXIT_FAILURE);
    }

    // read interface flags
    if (ioctl(udp_socket, SIOCGIFFLAGS, &ifr) < 0)
    {
        perror("Failed to SIOCGIFFLAGS");
        if (close(udp_socket))
        {
            perror("DOUBLE FAULT close(udp_socket)");
        }
        exit(EXIT_FAILURE);
    }
    // set flags
    ifr.ifr_flags |= IFF_UP;
    if (ioctl(udp_socket, SIOCSIFFLAGS, &ifr) < 0)
    {
        perror("Failed to SIOCSIFFLAGS");
        if (close(udp_socket))
        {
            perror("DOUBLE FAULT close(udp_socket)");
        }
        exit(EXIT_FAILURE);
    }

    if (close(udp_socket) < 0)
    {
        perror("Failed to close(udp_socket)");
        exit(EXIT_FAILURE);
    }
}

void set_interface_down(int if_fd, const char *ifname)
{
    (void)if_fd;
    // https://stackoverflow.com/questions/11679514/what-is-the-difference-between-iff-up-and-iff-running
    // difference IFF_UP vs IFF_RUNNING
    //  IFF_UP      =>  bring the interface up.
    //  IFF_RUNNING =>  check the interface status.
    struct ifreq ifr;
    memset(&ifr, 0, sizeof(ifr));
    strncpy(ifr.ifr_name, ifname, IFNAMSIZ);

    int udp_socket = socket(AF_INET, SOCK_DGRAM, 0);
    if (udp_socket < 0)
    {
        perror("Failed to create udp_socket");
        exit(EXIT_FAILURE);
    }

    // read interface flags
    if (ioctl(udp_socket, SIOCGIFFLAGS, &ifr) < 0)
    {
        perror("Failed to SIOCGIFFLAGS");
        if (close(udp_socket))
        {
            perror("DOUBLE FAULT close(udp_socket)");
        }
        exit(EXIT_FAILURE);
    }
    // set flags
    ifr.ifr_flags &= ~IFF_UP;
    if (ioctl(udp_socket, SIOCSIFFLAGS, &ifr) < 0)
    {
        perror("Failed to SIOCSIFFLAGS");
        if (close(udp_socket))
        {
            perror("DOUBLE FAULT close(udp_socket)");
        }
        exit(EXIT_FAILURE);
    }

    if (close(udp_socket) < 0)
    {
        perror("Failed to close(udp_socket)");
        exit(EXIT_FAILURE);
    }
}

