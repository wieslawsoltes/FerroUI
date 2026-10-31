// MIT. Generate the framework's synthetic, audible AVC/AAC fixture on macOS:
// swift tests/media/make-fixture.swift tests/media/fixtures/avc-aac.mp4
import AVFoundation
import CoreVideo
import Foundation

let destination = URL(fileURLWithPath: CommandLine.arguments[1])
try FileManager.default.createDirectory(at: destination.deletingLastPathComponent(), withIntermediateDirectories: true)
// Refuse replacement: fixture regeneration should be an intentional change.
let writer = try AVAssetWriter(outputURL: destination, fileType: .mp4)
let video = AVAssetWriterInput(mediaType: .video, outputSettings: [
    AVVideoCodecKey: AVVideoCodecType.h264, AVVideoWidthKey: 160, AVVideoHeightKey: 90,
    AVVideoColorPropertiesKey: [AVVideoColorPrimariesKey: AVVideoColorPrimaries_ITU_R_709_2,
        AVVideoTransferFunctionKey: AVVideoTransferFunction_ITU_R_709_2, AVVideoYCbCrMatrixKey: AVVideoYCbCrMatrix_ITU_R_709_2],
    AVVideoCompressionPropertiesKey: [AVVideoAverageBitRateKey: 160_000]
])
let adaptor = AVAssetWriterInputPixelBufferAdaptor(assetWriterInput: video,
    sourcePixelBufferAttributes: [kCVPixelBufferPixelFormatTypeKey as String: kCVPixelFormatType_32BGRA,
        kCVPixelBufferWidthKey as String: 160, kCVPixelBufferHeightKey as String: 90])
var asbd = AudioStreamBasicDescription(mSampleRate: 48_000, mFormatID: kAudioFormatLinearPCM,
    mFormatFlags: kAudioFormatFlagIsFloat | kAudioFormatFlagIsPacked,
    mBytesPerPacket: 4, mFramesPerPacket: 1, mBytesPerFrame: 4, mChannelsPerFrame: 1, mBitsPerChannel: 32, mReserved: 0)
var format: CMAudioFormatDescription?
precondition(CMAudioFormatDescriptionCreate(allocator: kCFAllocatorDefault, asbd: &asbd,
    layoutSize: 0, layout: nil, magicCookieSize: 0, magicCookie: nil, extensions: nil, formatDescriptionOut: &format) == noErr)
let audio = AVAssetWriterInput(mediaType: .audio, outputSettings: [
    AVFormatIDKey: kAudioFormatMPEG4AAC, AVSampleRateKey: 48_000, "AV" + "NumberOfChannelsKey": 1, AVEncoderBitRateKey: 64_000
], sourceFormatHint: format)
writer.add(video); writer.add(audio)
video.expectsMediaDataInRealTime=true;audio.expectsMediaDataInRealTime=true
precondition(writer.startWriting(), "\(String(describing: writer.error))")
writer.startSession(atSourceTime: .zero)
func ready(_ input: AVAssetWriterInput) {
    let deadline=Date().addingTimeInterval(10)
    while !input.isReadyForMoreMediaData {
        precondition(Date()<deadline,"Writer readiness timed out: \(input.mediaType), status=\(writer.status), error=\(String(describing:writer.error))")
        precondition(writer.status == .writing, "\(String(describing: writer.error))")
        Thread.sleep(forTimeInterval: 0.001)
    }
}
for frame in 0..<90 {
    ready(video)
    var pixel: CVPixelBuffer?
    precondition(CVPixelBufferPoolCreatePixelBuffer(nil, adaptor.pixelBufferPool!, &pixel) == kCVReturnSuccess)
    let buffer = pixel!
    CVPixelBufferLockBaseAddress(buffer, [])
    let bytes = CVPixelBufferGetBaseAddress(buffer)!.assumingMemoryBound(to: UInt8.self)
    let stride = CVPixelBufferGetBytesPerRow(buffer)
    for y in 0..<90 { for x in 0..<160 {
        let color: (UInt8,UInt8,UInt8) = y<45 ? (x<80 ? (255,0,0):(0,255,0)) : (x<80 ? (0,0,255):(255,255,0))
        let offset = y*stride+x*4
        bytes[offset]=color.2; bytes[offset+1]=color.1; bytes[offset+2]=color.0; bytes[offset+3]=255
    }}
    CVPixelBufferUnlockBaseAddress(buffer, [])
    precondition(adaptor.append(buffer, withPresentationTime: CMTime(value: Int64(frame), timescale: 30)))
    ready(audio)
    let samples = (0..<1600).map { index in Float(sin(2 * Double.pi * 440 * Double(frame*1600+index)/48_000) * 0.2) }
    var block: CMBlockBuffer?
    precondition(CMBlockBufferCreateWithMemoryBlock(allocator: kCFAllocatorDefault, memoryBlock: nil,
        blockLength: samples.count*4, blockAllocator: kCFAllocatorDefault, customBlockSource: nil,
        offsetToData: 0, dataLength: samples.count*4, flags: 0, blockBufferOut: &block) == noErr)
    samples.withUnsafeBytes { raw in precondition(CMBlockBufferReplaceDataBytes(with: raw.baseAddress!, blockBuffer: block!, offsetIntoDestination: 0, dataLength: raw.count) == noErr) }
    var timing = CMSampleTimingInfo(duration: CMTime(value: 1,timescale: 48_000), presentationTimeStamp: CMTime(value: Int64(frame*1600),timescale: 48_000), decodeTimeStamp: .invalid)
    var sample: CMSampleBuffer?
    var size = 4
    precondition(CMSampleBufferCreateReady(allocator: kCFAllocatorDefault, dataBuffer: block!, formatDescription: format!,
        sampleCount: samples.count, sampleTimingEntryCount: 1, sampleTimingArray: &timing,
        sampleSizeEntryCount: 1, sampleSizeArray: &size, sampleBufferOut: &sample) == noErr)
    precondition(audio.append(sample!))
}
video.markAsFinished(); audio.markAsFinished()
let finished=DispatchSemaphore(value:0)
writer.finishWriting { finished.signal() }
precondition(finished.wait(timeout:.now()+20) == .success)
precondition(writer.status == .completed, "\(String(describing: writer.error))")
print("Synthetic AVC/AAC fixture: 160×90, 30 fps, 3 s, mono 440 Hz at 48 kHz")
