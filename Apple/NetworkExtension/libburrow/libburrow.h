#include <stdint.h>
int retrieve();

typedef struct {
    int64_t ipv4_addr;
    int64_t ipv4_netmask;
    int32_t mtu;
} NetWorkSettings;

NetWorkSettings getNetworkSettings(int);
void initialize_oslog();
void spawn_server();
