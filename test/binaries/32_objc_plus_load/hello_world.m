#import <Foundation/Foundation.h>
#include <stdio.h>
@interface Greeter : NSObject @end
@implementation Greeter
+ (void)load { fprintf(stderr, "Hello, World!\n"); }
@end
int main(void) { return 0; }
