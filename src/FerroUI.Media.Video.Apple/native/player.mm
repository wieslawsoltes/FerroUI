#import <AVFoundation/AVFoundation.h>
#import <CoreVideo/CoreVideo.h>
#include <cmath>
#include <cstdint>
#include <cstdio>

@interface FerroPortablePlayer : NSObject
@property(nonatomic,strong) AVPlayer *player;
@property(nonatomic,strong) AVPlayerItem *item;
@property(nonatomic,strong) AVPlayerItemVideoOutput *output;
@property(nonatomic,strong) id endedObserver;
@property(nonatomic) BOOL ended;
- (void)close;
@end
@implementation FerroPortablePlayer
- (void)close {
    if (_endedObserver) [[NSNotificationCenter defaultCenter] removeObserver:_endedObserver];
    _endedObserver=nil;
    [_player pause];
    [_item cancelPendingSeeks];
    if (_output) [_item removeOutput:_output];
    _output=nil;_item=nil;_player=nil;
}
- (void)dealloc { [self close]; }
@end
@interface FerroPortableSeek : NSObject
@property(nonatomic) int result;
@property(nonatomic) double position;
@end
@implementation FerroPortableSeek
@end
extern "C" {
struct FerroPortableState {double position,duration;uint32_t width,height;int32_t state,seekable;};
void *ferroui_portable_player_create(const char *source) {
    @autoreleasepool {
        if (![NSThread isMainThread]) return nullptr;
        NSString *text=[NSString stringWithUTF8String:source];
        if (!text.length) return nullptr;
        NSURL *url=([text hasPrefix:@"https://"] || [text hasPrefix:@"http://"] || [text hasPrefix:@"file://"])
            ? [NSURL URLWithString:text] : [NSURL fileURLWithPath:text];
        if (!url) return nullptr;
        FerroPortablePlayer *owner=[FerroPortablePlayer new];
        owner.item=[AVPlayerItem playerItemWithURL:url];
        owner.output=[[AVPlayerItemVideoOutput alloc] initWithPixelBufferAttributes:@{
            (id)kCVPixelBufferPixelFormatTypeKey:@(kCVPixelFormatType_32BGRA),
            (id)kCVPixelBufferIOSurfacePropertiesKey:@{},(id)kCVPixelBufferMetalCompatibilityKey:@YES}];
        [owner.item addOutput:owner.output];
        owner.player=[AVPlayer playerWithPlayerItem:owner.item];
        __weak FerroPortablePlayer *weak=owner;
        owner.endedObserver=[[NSNotificationCenter defaultCenter] addObserverForName:AVPlayerItemDidPlayToEndTimeNotification object:owner.item queue:[NSOperationQueue mainQueue] usingBlock:^(NSNotification *note) {
            (void)note; weak.ended=YES;
        }];
        return (__bridge_retained void *)owner;
    }
}
void ferroui_portable_player_release(void *p) { @autoreleasepool {FerroPortablePlayer *owner=(__bridge_transfer FerroPortablePlayer *)p;[owner close];} }
void ferroui_portable_player_state(void *p,FerroPortableState *out) {
    FerroPortablePlayer *owner=(__bridge FerroPortablePlayer *)p;
    out->position=CMTimeGetSeconds(owner.item.currentTime);
    out->duration=CMTimeGetSeconds(owner.item.duration);
    CGSize size=owner.item.presentationSize;
    out->width=std::isfinite(size.width)&&size.width>0?(uint32_t)size.width:0;
    out->height=std::isfinite(size.height)&&size.height>0?(uint32_t)size.height:0;
    out->seekable=0;
    for (NSValue *value in owner.item.seekableTimeRanges) {
        double duration=CMTimeGetSeconds(value.CMTimeRangeValue.duration);
        if (std::isfinite(duration) && duration>0) {out->seekable=1;break;}
    }
    if (owner.item.status==AVPlayerItemStatusFailed || owner.player.status==AVPlayerStatusFailed) out->state=5;
    else if (owner.item.status!=AVPlayerItemStatusReadyToPlay) out->state=0;
    else if (owner.ended) out->state=4;
    else if (owner.player.timeControlStatus==AVPlayerTimeControlStatusWaitingToPlayAtSpecifiedRate) out->state=3;
    else if (owner.player.rate!=0) out->state=2;
    else out->state=1;
}
void ferroui_portable_player_error(void *p,char *buffer,size_t length) {
    FerroPortablePlayer *owner=(__bridge FerroPortablePlayer *)p;
    const char *text=(owner.item.error?:owner.player.error).localizedDescription.UTF8String;
    if (length) std::snprintf(buffer,length,"%s",text?:"Native playback failed");
}
void ferroui_portable_player_play(void *p) {
    FerroPortablePlayer *owner=(__bridge FerroPortablePlayer *)p;
    if (owner.ended) {owner.ended=NO;[owner.player seekToTime:kCMTimeZero];}
    [owner.player play];
}
void ferroui_portable_player_pause(void *p) { [((__bridge FerroPortablePlayer *)p).player pause]; }
void ferroui_portable_player_audio(void *p,double volume,int muted) {
    FerroPortablePlayer *owner=(__bridge FerroPortablePlayer *)p;
    owner.player.volume=(float)volume;owner.player.muted=muted!=0;
}
void *ferroui_portable_player_seek(void *p,double seconds) {
    FerroPortablePlayer *owner=(__bridge FerroPortablePlayer *)p;
    [owner.item cancelPendingSeeks];owner.ended=NO;
    FerroPortableSeek *request=[FerroPortableSeek new];
    __weak AVPlayer *weakPlayer=owner.player;
    [owner.player seekToTime:CMTimeMakeWithSeconds(seconds,1000000) toleranceBefore:kCMTimeZero toleranceAfter:kCMTimeZero completionHandler:^(BOOL finished) {
        // AVFoundation may deliver from its private queue. Publish on the main
        // queue, the same owner thread that polls/releases this request.
        dispatch_async(dispatch_get_main_queue(),^{
            AVPlayer *player=weakPlayer;
            request.position=player?CMTimeGetSeconds(player.currentTime):NAN;
            request.result=finished&&player?1:-1;
        });
    }];
    return (__bridge_retained void *)request;
}
int ferroui_portable_seek_state(void *p,double *position) {FerroPortableSeek *request=(__bridge FerroPortableSeek *)p;*position=request.position;return request.result;}
void ferroui_portable_seek_release(void *p) {id request=(__bridge_transfer id)p;(void)request;}
void *ferroui_portable_player_frame(void *p,double *timestamp) {
    FerroPortablePlayer *owner=(__bridge FerroPortablePlayer *)p;
    CMTime time=owner.item.currentTime;
    if (![owner.output hasNewPixelBufferForItemTime:time]) return nullptr;
    CMTime display=kCMTimeInvalid;
    CVPixelBufferRef buffer=[owner.output copyPixelBufferForItemTime:time itemTimeForDisplay:&display];
    *timestamp=CMTimeGetSeconds(display);
    return buffer;
}
}
