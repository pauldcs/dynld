#include <dispatch/dispatch.h>
#include <stdio.h>
int main(void) {
    dispatch_queue_t q = dispatch_queue_create("test", DISPATCH_QUEUE_SERIAL);
    dispatch_semaphore_t sem = dispatch_semaphore_create(0);
    dispatch_async(q, ^{
        fprintf(stderr, "Hello, World!\n");
        dispatch_semaphore_signal(sem);
    });
    dispatch_semaphore_wait(sem, DISPATCH_TIME_FOREVER);
    return 0;
}
