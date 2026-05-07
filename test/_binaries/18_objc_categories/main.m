
#import <Foundation/Foundation.h>

@interface NSObject (Hello)
- (void)greet;
@end

@implementation NSObject (Hello)
- (void)greet {
    printf("Hello, World!\n");
}
@end

int main(void) {
    NSObject *obj = [[NSObject alloc] init];
    [obj greet];
    [obj release];
    return 0;
}
