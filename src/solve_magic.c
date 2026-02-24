#include <stdio.h>  // perror
#include <stdlib.h> // system 执行 shell 命令
#include <fcntl.h>  // open
#include <unistd.h> // close
#include <sys/ioctl.h>  // ioctl

#define MAGIC_DEV "/dev/magic"
#define MAGIC_CMD 0x1337

int main() {
    int fd = open(MAGIC_DEV, O_RDWR);
    if (fd < 0) {
        perror("Failed to open /dev/magic");
        return 1;
    }

    printf("Sending ioctl 0x%x \n", MAGIC_CMD);
    if (ioctl(fd, MAGIC_CMD, 0) < 0) {
        perror("ioctl failed");
        close(fd);
        return 1;
    }

    printf("CHECK DMESG NOW.\n");
    close(fd);
    
    system("dmesg | tail -n 10");
    return 0;
}