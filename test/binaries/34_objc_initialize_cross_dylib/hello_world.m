#import <Foundation/Foundation.h>
#include <stdio.h>

#if LIB_A
@interface A : NSObject @end
@implementation A
+ (void)initialize { fprintf(stderr, "Hello, World!\n"); }
+ (void)touch {}
@end
#elif LIB_B
@interface A : NSObject + (void)touch; @end
@interface B : NSObject @end
@implementation B
+ (void)load { [A touch]; }
@end
void b_anchor(void) {}
#else
void b_anchor(void);
int main(void) { b_anchor(); }
#endif
