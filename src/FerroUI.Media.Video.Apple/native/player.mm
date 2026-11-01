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
@property(nonatomic,strong) AVAssetTrack *geometryTrack;
@property(nonatomic) BOOL geometryReady;
@property(nonatomic) CGAffineTransform trackTransform;
@property(nonatomic,strong) NSError *geometryError;
- (void)close;
@end
@implementation FerroPortablePlayer
- (void)close {
    if (_endedObserver) [[NSNotificationCenter defaultCenter] removeObserver:_endedObserver];
    _endedObserver=nil;
    [_player pause];
    [_item cancelPendingSeeks];
    if (_output) [_item removeOutput:_output];
    _output=nil;_item=nil;_player=nil;_geometryTrack=nil;
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
        owner.trackTransform=CGAffineTransformIdentity;
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
    if(owner.item.status==AVPlayerItemStatusReadyToPlay){
        AVAssetTrack *selected=nil;
        for(AVPlayerItemTrack *track in owner.item.tracks){
            if(track.enabled && [track.assetTrack.mediaType isEqualToString:AVMediaTypeVideo]){selected=track.assetTrack;break;}
        }
        if(selected && selected!=owner.geometryTrack){
            owner.geometryTrack=selected;owner.geometryReady=NO;owner.geometryError=nil;
            __weak FerroPortablePlayer *weak=owner;
            // Never synchronously load asset metadata on the UI thread. This
            // older availability-compatible API preserves the framework floor.
            [selected loadValuesAsynchronouslyForKeys:@[@"preferredTransform"] completionHandler:^{
                dispatch_async(dispatch_get_main_queue(),^{
                    FerroPortablePlayer *live=weak;
                    if(!live.item || live.geometryTrack!=selected)return;
                    NSError *error=nil;
                    if([selected statusOfValueForKey:@"preferredTransform" error:&error]==AVKeyValueStatusLoaded){
                        live.trackTransform=selected.preferredTransform;live.geometryReady=YES;
                    }else live.geometryError=error?:[NSError errorWithDomain:@"FerroUI.Media.Video" code:1 userInfo:@{NSLocalizedDescriptionKey:@"Video track geometry is unavailable"}];
                });
            }];
        }else if(!selected && !out->width && !out->height){
            owner.geometryReady=YES; // Audio-only media has no video geometry.
        }else if(!selected){
            owner.geometryError=[NSError errorWithDomain:@"FerroUI.Media.Video" code:2 userInfo:@{NSLocalizedDescriptionKey:@"Video track geometry is unavailable"}];
        }
    }
    out->seekable=0;
    for (NSValue *value in owner.item.seekableTimeRanges) {
        double duration=CMTimeGetSeconds(value.CMTimeRangeValue.duration);
        if (std::isfinite(duration) && duration>0) {out->seekable=1;break;}
    }
    if (owner.geometryError || owner.item.status==AVPlayerItemStatusFailed || owner.player.status==AVPlayerStatusFailed) out->state=5;
    else if (owner.item.status!=AVPlayerItemStatusReadyToPlay || !owner.geometryReady) out->state=0;
    else if (owner.ended) out->state=4;
    else if (owner.player.timeControlStatus==AVPlayerTimeControlStatusWaitingToPlayAtSpecifiedRate) out->state=3;
    else if (owner.player.rate!=0) out->state=2;
    else out->state=1;
}
void ferroui_portable_player_error(void *p,char *buffer,size_t length) {
    FerroPortablePlayer *owner=(__bridge FerroPortablePlayer *)p;
    const char *text=(owner.geometryError?:owner.item.error?:owner.player.error).localizedDescription.UTF8String;
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
    if(!owner.geometryReady || owner.geometryError)return nullptr;
    CMTime time=owner.item.currentTime;
    if (![owner.output hasNewPixelBufferForItemTime:time]) return nullptr;
    CMTime display=kCMTimeInvalid;
    CVPixelBufferRef buffer=[owner.output copyPixelBufferForItemTime:time itemTimeForDisplay:&display];
    *timestamp=CMTimeGetSeconds(display);
    return buffer;
}
struct FerroPortableGeometry {double transform[4],clean[4],pixel_aspect;};
int ferroui_portable_player_geometry(void *p,void *pixels,FerroPortableGeometry *out){
    FerroPortablePlayer *owner=(__bridge FerroPortablePlayer *)p;
    if(!owner.geometryReady || !pixels || !out)return 0;
    CGAffineTransform t=owner.trackTransform;
    if(!std::isfinite(t.tx) || !std::isfinite(t.ty))return 0;
    out->transform[0]=t.a;out->transform[1]=t.b;out->transform[2]=t.c;out->transform[3]=t.d;
    CVPixelBufferRef buffer=(CVPixelBufferRef)pixels;
    CGRect clean=CVImageBufferGetCleanRect(buffer);
    // CoreVideo's clean rectangle is lower-left; shared frame crop is top-left.
    out->clean[0]=clean.origin.x;
    out->clean[1]=(double)CVPixelBufferGetHeight(buffer)-CGRectGetMaxY(clean);
    out->clean[2]=clean.size.width;out->clean[3]=clean.size.height;
    NSDictionary *aspect=(__bridge NSDictionary *)CVBufferGetAttachment(buffer,kCVImageBufferPixelAspectRatioKey,nullptr);
    out->pixel_aspect=1.;
    if(aspect){
        if(![aspect isKindOfClass:[NSDictionary class]])return 0;
        NSNumber *horizontal=aspect[(id)kCVImageBufferPixelAspectRatioHorizontalSpacingKey];
        NSNumber *vertical=aspect[(id)kCVImageBufferPixelAspectRatioVerticalSpacingKey];
        if(![horizontal isKindOfClass:[NSNumber class]] || ![vertical isKindOfClass:[NSNumber class]])return 0;
        out->pixel_aspect=vertical.doubleValue>0?horizontal.doubleValue/vertical.doubleValue:NAN;
    }
    return 1;
}
}
