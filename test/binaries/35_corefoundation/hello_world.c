#include <CoreFoundation/CoreFoundation.h>
#include <stdio.h>
int main(void) {
    CFStringRef s = CFStringCreateWithCString(NULL, "Hello, World!", kCFStringEncodingUTF8);
    char buf[64];
    CFStringGetCString(s, buf, sizeof(buf), kCFStringEncodingUTF8);
    CFRelease(s);
    fprintf(stderr, "%s\n", buf);
    return 0;
}
