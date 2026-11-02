// Development-only diagnostics. Capture is restricted to this process's window.
// No desktop/display capture and no Screen Recording consent request.
#import <AppKit/AppKit.h>
#import <CoreVideo/CoreVideo.h>
#import <Metal/Metal.h>
#import <ScreenCaptureKit/ScreenCaptureKit.h>
#include <mach/mach.h>
#include <mach/task_info.h>
#include <stdint.h>
#include <malloc/malloc.h>
#include <unistd.h>
#import <objc/message.h>

@interface FerroMediaShot : NSObject
@property(nonatomic) int status;
@property(nonatomic,strong) NSData *pixels;
@property(nonatomic,strong) NSString *error;
@end
@implementation FerroMediaShot
@end

void *ferro_media_window_capture(void *pointer, uint32_t width, uint32_t height) {
    @autoreleasepool {
        FerroMediaShot *owner=[FerroMediaShot new];
        NSWindow *window=(__bridge NSWindow *)pointer;
        if (![NSThread isMainThread] || ![[NSApp windows] containsObject:window] ||
            width<1 || height<1 || (uint64_t)width*height*4>64*1024*1024) {
            owner.error=@"Capture requires an owned, bounded window on the UI thread"; owner.status=-1;
        } else if (@available(macOS 14.4,*)) {
            NSInteger number=window.windowNumber;
            [SCShareableContent getCurrentProcessShareableContentWithCompletionHandler:^(SCShareableContent *content,NSError *error) {
                @autoreleasepool {
                    SCWindow *selected=nil;
                    for (SCWindow *candidate in content.windows) {
                        if (candidate.windowID==number && candidate.owningApplication.processID==getpid()) {selected=candidate;break;}
                    }
                    if (!selected || error) {
                        @synchronized(owner) {owner.error=error.localizedDescription?:@"Own window not available to ScreenCaptureKit";owner.status=-1;} return;
                    }
                    SCContentFilter *filter=[[SCContentFilter alloc] initWithDesktopIndependentWindow:selected];
                    SCStreamConfiguration *config=[SCStreamConfiguration new];
                    config.width=width;config.height=height;config.showsCursor=NO;
                    config.ignoreShadowsSingleWindow=YES;config.colorSpaceName=kCGColorSpaceSRGB;
                    [SCScreenshotManager captureImageWithFilter:filter configuration:config completionHandler:^(CGImageRef image,NSError *failure) {
                        @autoreleasepool {
                            NSMutableData *pixels=image?[NSMutableData dataWithLength:(NSUInteger)width*height*4]:nil;
                            CGColorSpaceRef space=CGColorSpaceCreateWithName(kCGColorSpaceSRGB);
                            CGContextRef context=pixels?CGBitmapContextCreate(pixels.mutableBytes,width,height,8,width*4,space,kCGImageAlphaPremultipliedLast|kCGBitmapByteOrder32Big):NULL;
                            if(context) {
                                // CGImage raster rows already match the image
                                // buffer's top-first order. AppKit's flipped
                                // view coordinates must not flip this bitmap.
                                CGContextDrawImage(context,CGRectMake(0,0,width,height),image);CGContextRelease(context);
                            }
                            CGColorSpaceRelease(space);
                            @synchronized(owner) {
                                if(!context || failure) {owner.error=failure.localizedDescription?:@"Window capture conversion failed";owner.status=-1;}
                                else {owner.pixels=pixels;owner.status=1;}
                            }
                        }
                    }];
                }
            }];
        } else {owner.error=@"Window-only diagnostics require macOS 14.4";owner.status=-1;}
        return (__bridge_retained void *)owner;
    }
}
int ferro_media_capture_poll(void *pointer,unsigned char *pixels,size_t capacity,char *error,size_t error_capacity) {
    FerroMediaShot *owner=(__bridge FerroMediaShot *)pointer;
    @synchronized(owner) {
        if(error_capacity) snprintf(error,error_capacity,"%s",owner.error.UTF8String?:"");
        if(owner.status==1) {
            if(capacity!=owner.pixels.length) return -1;
            memcpy(pixels,owner.pixels.bytes,capacity);
        }
        return owner.status;
    }
}
void ferro_media_capture_release(void *pointer) {id owner=(__bridge_transfer id)pointer;(void)owner;}

int ferro_media_read_bgra(void *pointer,unsigned char *output,size_t capacity) {
    CVPixelBufferRef buffer=pointer;
    size_t width=CVPixelBufferGetWidth(buffer),height=CVPixelBufferGetHeight(buffer);
    if(CVPixelBufferGetPixelFormatType(buffer)!=kCVPixelFormatType_32BGRA || width*height*4!=capacity ||
       CVPixelBufferLockBaseAddress(buffer,kCVPixelBufferLock_ReadOnly)!=kCVReturnSuccess) return 0;
    const unsigned char *source=CVPixelBufferGetBaseAddress(buffer);size_t stride=CVPixelBufferGetBytesPerRow(buffer);
    for(size_t y=0;y<height;y++)for(size_t x=0;x<width;x++) {
        const unsigned char *p=source+y*stride+x*4;unsigned char *q=output+(y*width+x)*4;
        q[0]=p[2];q[1]=p[1];q[2]=p[0];q[3]=p[3];
    }
    CVPixelBufferUnlockBaseAddress(buffer,kCVPixelBufferLock_ReadOnly);return 1;
}

// Footprint includes native/driver allocations outside frame-budget accounting.
int ferro_media_memory(uint64_t *values) {
    @autoreleasepool {
        task_vm_info_data_t vm={0};mach_msg_type_number_t count=TASK_VM_INFO_COUNT;
        if(task_info(mach_task_self(),TASK_VM_INFO,(task_info_t)&vm,&count)!=KERN_SUCCESS)return 0;
        values[0]=vm.phys_footprint;values[1]=vm.resident_size;
        id<MTLDevice> device=MTLCreateSystemDefaultDevice();
        if(!device)return 0;
        values[2]=device.currentAllocatedSize;values[3]=device.registryID;
        mach_port_name_array_t names=NULL;mach_port_type_array_t types=NULL;
        mach_msg_type_number_t name_count=0,type_count=0;
        if(mach_port_names(mach_task_self(),&names,&name_count,&types,&type_count)!=KERN_SUCCESS)return 0;
        values[4]=name_count;
        malloc_statistics_t heap={0};malloc_zone_statistics(NULL,&heap);
        values[5]=heap.size_in_use;values[6]=heap.blocks_in_use;
        vm_deallocate(mach_task_self(),(vm_address_t)names,name_count*sizeof(*names));
        vm_deallocate(mach_task_self(),(vm_address_t)types,type_count*sizeof(*types));
        return 1;
    }
}

static id FerroAXValue(id node,NSString *name) {
    SEL selector=NSSelectorFromString(name);
    return [node respondsToSelector:selector]?((id(*)(id,SEL))objc_msgSend)(node,selector):nil;
}
static id FerroAXFind(id node,NSString *name,NSString *role,NSUInteger depth,NSUInteger *remaining) {
    if(!node || depth>48 || *remaining==0)return nil;
    --*remaining;
    NSString *label=FerroAXValue(node,@"accessibilityLabel")?:FerroAXValue(node,@"accessibilityTitle");
    if([label isEqualToString:name] && [FerroAXValue(node,@"accessibilityRole") isEqualToString:role])return node;
    for(id child in FerroAXValue(node,@"accessibilityChildren")){
        id found=FerroAXFind(child,name,role,depth+1,remaining);if(found)return found;
    }
    return nil;
}
// Only queries/acts on this process's native accessibility tree. Keyboard events
// go directly to this window, not through a global event tap or another app.
int ferro_media_transport_action(void *pointer,const char *label,const char *role,int action) {
    @autoreleasepool {
        NSWindow *window=(__bridge NSWindow *)pointer;
        if(![NSThread isMainThread] || ![[NSApp windows] containsObject:window])return 0;
        NSUInteger remaining=4096;
        id node=FerroAXFind(window.contentView,[NSString stringWithUTF8String:label],[NSString stringWithUTF8String:role],0,&remaining);
        if(!node)return 0;
        if(action==0)return 1;
        if(action==1){
            if(![node respondsToSelector:@selector(accessibilityPerformPress)])return 0;
            return [node accessibilityPerformPress]?1:0;
        }
        if(action==2){
            [NSApp activateIgnoringOtherApps:YES];
            [window makeKeyAndOrderFront:nil];
            if(![node respondsToSelector:@selector(setAccessibilityFocused:)])return 0;
            [node setAccessibilityFocused:YES];return 1;
        }
        if(action==3)return window.isKeyWindow?1:0;
        return 0;
    }
}
int ferro_media_transport_key(void *pointer,unsigned short key,const char *text) {
    @autoreleasepool {
        NSWindow *window=(__bridge NSWindow *)pointer;
        if(![NSThread isMainThread])return -1;
        if(![[NSApp windows] containsObject:window])return -2;
        if(!window.isKeyWindow)return -3;
        NSString *characters=[NSString stringWithUTF8String:text];
        for(NSNumber *kind in @[@(NSEventTypeKeyDown),@(NSEventTypeKeyUp)]){
            NSEvent *event=[NSEvent keyEventWithType:kind.unsignedIntegerValue location:NSZeroPoint modifierFlags:0 timestamp:NSProcessInfo.processInfo.systemUptime windowNumber:window.windowNumber context:nil characters:characters charactersIgnoringModifiers:characters isARepeat:NO keyCode:key];
            [window sendEvent:event];
        }
        return 1;
    }
}
