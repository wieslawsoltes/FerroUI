// Silent H.264 recording consumes existing native capture samples. It opens no
// second camera or microphone and never maps a preview pixel buffer to the CPU.
static std::atomic<bool> FerroPortableRecordingBusy{false};
static constexpr uint64_t FerroRecordingLimit = 256ull * 1024 * 1024;
@interface FerroPortableRecording : NSObject
@property(nonatomic,strong) dispatch_queue_t queue;
@property(nonatomic,strong) AVAssetWriter *writer;
@property(nonatomic,strong) AVAssetWriterInput *input;
@property(nonatomic,strong) NSURL *url;
@property(nonatomic,strong) NSData *data;
@property(nonatomic) int status;
@property(nonatomic) BOOL finishing,finalizing,cancelScheduled,holdsSlot;
@property(nonatomic) uint32_t width,height;
@property(nonatomic) CMTime first,last;
@property(nonatomic) double duration;
- (void)append:(CMSampleBufferRef)sample;
- (void)finish;
- (void)cancel;
@end
@implementation FerroPortableRecording
- (instancetype)init {
    if ((self=[super init])) {
        _first=kCMTimeInvalid; _last=kCMTimeInvalid; _holdsSlot=YES;
        _url=[NSURL fileURLWithPath:[NSTemporaryDirectory() stringByAppendingPathComponent:
            [NSString stringWithFormat:@"ferroui-camera-%@.mp4", NSUUID.UUID.UUIDString]]];
    }
    return self;
}
- (void)append:(CMSampleBufferRef)sample {
    // Called only on the existing serial camera callback queue.
    @synchronized(self) {
        if (_status!=0 || _finishing) return;
        CVPixelBufferRef buffer=CMSampleBufferGetImageBuffer(sample);
        CMTime timestamp=CMSampleBufferGetPresentationTimeStamp(sample);
        if (!buffer || !CMTIME_IS_NUMERIC(timestamp) ||
            CVPixelBufferGetPixelFormatType(buffer)!=kCVPixelFormatType_32BGRA) {
            _status=-1; [self cancel]; return;
        }
        if (!_writer) {
            _width=(uint32_t)CVPixelBufferGetWidth(buffer); _height=(uint32_t)CVPixelBufferGetHeight(buffer);
            NSError *error=nil;
            _writer=[[AVAssetWriter alloc] initWithURL:_url fileType:AVFileTypeMPEG4 error:&error];
            NSDictionary *settings=@{AVVideoCodecKey: AVVideoCodecTypeH264,
                AVVideoWidthKey: @(_width), AVVideoHeightKey: @(_height),
                AVVideoColorPropertiesKey: @{AVVideoColorPrimariesKey: AVVideoColorPrimaries_ITU_R_709_2,
                    AVVideoTransferFunctionKey: AVVideoTransferFunction_ITU_R_709_2,
                    AVVideoYCbCrMatrixKey: AVVideoYCbCrMatrix_ITU_R_709_2}};
            _input=[AVAssetWriterInput assetWriterInputWithMediaType:AVMediaTypeVideo outputSettings:settings];
            _input.expectsMediaDataInRealTime=YES;
            if (!_writer || ![_writer canAddInput:_input]) { _status=-1; [self cancel]; return; }
            [_writer addInput:_input];
            if (![_writer startWriting]) { _status=-1; [self cancel]; return; }
            _first=timestamp; [_writer startSessionAtSourceTime:timestamp];
        }
        if (_width!=CVPixelBufferGetWidth(buffer) || _height!=CVPixelBufferGetHeight(buffer)) {
            _status=-1; [self cancel]; return;
        }
        if (CMTIME_IS_NUMERIC(_last) && CMTimeCompare(timestamp,_last)<=0) return;
        double elapsed=CMTimeGetSeconds(CMTimeSubtract(timestamp,_first));
        if (elapsed>=60.) { [self finish]; return; }
        if (!_input.readyForMoreMediaData) return; // Bounded native encoder backpressure.
        if (![_input appendSampleBuffer:sample]) { _status=-1; [self cancel]; return; }
        _last=timestamp;
        CMTime duration=CMSampleBufferGetDuration(sample);
        _duration=std::fmin(60.,elapsed +
            (CMTIME_IS_NUMERIC(duration) && CMTimeCompare(duration,kCMTimeZero)>0 ? CMTimeGetSeconds(duration) : 1./30.));
        NSNumber *length=[[NSFileManager defaultManager] attributesOfItemAtPath:_url.path error:nil][NSFileSize];
        if (length.unsignedLongLongValue>FerroRecordingLimit) { _status=-1; [self cancel]; return; }
        if (_duration>=60.) [self finish];
    }
}
- (void)finish {
    @synchronized(self) {
        if (_status!=0 || _finishing) return;
        _finishing=YES;
    }
    dispatch_async(_queue, ^{
        @synchronized(self) {
            if (self.status!=0) return;
            if (!self.writer || !CMTIME_IS_NUMERIC(self.last)) {
                self.status=-1; [self cancel]; return;
            }
            self.finalizing=YES;
            [self.input markAsFinished];
            AVAssetWriter *writer=self.writer;
            [writer finishWritingWithCompletionHandler:^{
                BOOL wanted;
                @synchronized(self) { wanted=self.status==0; }
                NSData *data=nil;
                if (wanted && writer.status==AVAssetWriterStatusCompleted) {
                    NSNumber *length=[[NSFileManager defaultManager] attributesOfItemAtPath:self.url.path error:nil][NSFileSize];
                    if (length.unsignedLongLongValue>0 && length.unsignedLongLongValue<=FerroRecordingLimit)
                        data=[NSData dataWithContentsOfURL:self.url options:0 error:nil];
                }
                [[NSFileManager defaultManager] removeItemAtURL:self.url error:nil];
                @synchronized(self) {
                    if (self.status==0) {
                        self.data=data.length>0 && data.length<=FerroRecordingLimit ? data : nil;
                        self.status=self.data ? 1 : -1;
                    }
                    self.input=nil; self.writer=nil;
                    if (self.holdsSlot) { self.holdsSlot=NO; FerroPortableRecordingBusy.store(false); }
                }
            }];
        }
    });
}
- (void)cancel {
    @synchronized(self) {
        if (_status==1 || _cancelScheduled) return;
        _cancelScheduled=YES;
        if (_status==0) _status=-2;
        _data=nil;
    }
    dispatch_async(_queue, ^{
        @synchronized(self) {
            if (self.finalizing) return; // Finish callback retains the slot until native completion.
            [self.writer cancelWriting]; self.writer=nil; self.input=nil;
            [[NSFileManager defaultManager] removeItemAtURL:self.url error:nil];
            if (self.holdsSlot) { self.holdsSlot=NO; FerroPortableRecordingBusy.store(false); }
        }
    });
}
- (void)dealloc { [[NSFileManager defaultManager] removeItemAtURL:_url error:nil]; }
@end
