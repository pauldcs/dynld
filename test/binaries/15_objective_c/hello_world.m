/*
 * gcc -lobjc -framework Foundation hello_world.m
 */

#import <Foundation/Foundation.h>

@interface HelloWorld : NSObject
- (void)hello_world;
@end

@implementation HelloWorld
- (void)hello_world {
    NSString *message = @"Hello, World!";
    fputs(message.UTF8String, stdout);
    fputc('\n', stdout);
}
@end

int main(int argc, char *argv[])
{
    @autoreleasepool {
        HelloWorld *hw = [HelloWorld new];
        [hw hello_world];
    }
    return 0;
}