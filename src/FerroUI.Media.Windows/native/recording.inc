// The recorder owns a bounded queue and a seekable, size-limited MP4 stream.
static constexpr uint64_t movie_limit = 256ull * 1024 * 1024;
static constexpr LONGLONG movie_time_limit = 60 * 10000000ll;

class MovieStream final : public Microsoft::WRL::RuntimeClass<
    Microsoft::WRL::RuntimeClassFlags<Microsoft::WRL::ClassicCom>, IStream> {
    std::mutex mutex_;
    uint64_t position_ = 0;
    std::vector<BYTE> bytes_;

public:
    STDMETHODIMP Read(void *out, ULONG count, ULONG *read) override {
        std::lock_guard<std::mutex> guard(mutex_);
        size_t available = bytes_.size() - std::min<uint64_t>(position_, bytes_.size());
        ULONG length = static_cast<ULONG>(std::min<size_t>(count, available));
        if (length) memcpy(out, bytes_.data() + position_, length);
        position_ += length;
        if (read) *read = length;
        return length == count ? S_OK : S_FALSE;
    }
    STDMETHODIMP Write(const void *data, ULONG count, ULONG *written) override {
        std::lock_guard<std::mutex> guard(mutex_);
        if (written) *written = 0;
        if (count > movie_limit - position_) return STG_E_MEDIUMFULL;
        try {
            if (position_ + count > bytes_.capacity())
                bytes_.reserve(std::min<uint64_t>(movie_limit,
                    std::max<uint64_t>(position_ + count, 2 * bytes_.capacity())));
            if (position_ + count > bytes_.size()) bytes_.resize(position_ + count);
        } catch (...) { return E_OUTOFMEMORY; }
        if (count) memcpy(bytes_.data() + position_, data, count);
        position_ += count;
        if (written) *written = count;
        return S_OK;
    }
    STDMETHODIMP Seek(LARGE_INTEGER offset, DWORD origin, ULARGE_INTEGER *result) override {
        std::lock_guard<std::mutex> guard(mutex_);
        LONGLONG base;
        switch (origin) {
            case STREAM_SEEK_SET: base = 0; break;
            case STREAM_SEEK_CUR: base = position_; break;
            case STREAM_SEEK_END: base = bytes_.size(); break;
            default: return STG_E_INVALIDFUNCTION;
        }
        if (offset.QuadPart < -base || offset.QuadPart > static_cast<LONGLONG>(movie_limit) - base)
            return STG_E_INVALIDFUNCTION;
        position_ = base + offset.QuadPart;
        if (result) result->QuadPart = position_;
        return S_OK;
    }
    STDMETHODIMP SetSize(ULARGE_INTEGER size) override {
        std::lock_guard<std::mutex> guard(mutex_);
        if (size.QuadPart > movie_limit) return STG_E_MEDIUMFULL;
        try { bytes_.resize(size.QuadPart); } catch (...) { return E_OUTOFMEMORY; }
        return S_OK;
    }
    STDMETHODIMP Stat(STATSTG *stat, DWORD) override {
        std::lock_guard<std::mutex> guard(mutex_);
        *stat = {};
        stat->type = STGTY_STREAM;
        stat->cbSize.QuadPart = bytes_.size();
        stat->grfMode = STGM_READWRITE;
        return S_OK;
    }
    STDMETHODIMP Commit(DWORD) override { return S_OK; }
    STDMETHODIMP Revert() override { return E_NOTIMPL; }
    STDMETHODIMP CopyTo(IStream *, ULARGE_INTEGER, ULARGE_INTEGER *, ULARGE_INTEGER *) override {
        return E_NOTIMPL;
    }
    STDMETHODIMP LockRegion(ULARGE_INTEGER, ULARGE_INTEGER, DWORD) override { return E_NOTIMPL; }
    STDMETHODIMP UnlockRegion(ULARGE_INTEGER, ULARGE_INTEGER, DWORD) override { return E_NOTIMPL; }
    STDMETHODIMP Clone(IStream **) override { return E_NOTIMPL; }

    std::vector<BYTE> take() {
        std::lock_guard<std::mutex> guard(mutex_);
        return std::move(bytes_);
    }
};

struct Recording {
    std::mutex lock;
    std::condition_variable changed;
    const UINT width, height, fps_n, fps_d;
    const LONGLONG frame_duration;
    FerroWindowsRecordingInfo info{};
    ComPtr<IMFSample> pending;
    LONGLONG timestamp = 0;
    bool finish = false;
    std::atomic<bool> worker_done{false};
    int status = 0;
    std::vector<BYTE> encoded;

    Recording(UINT w, UINT h, UINT n, UINT d)
        : width(w), height(h), fps_n(n), fps_d(d),
          frame_duration(10000000ll * d / n) {}

    void cancel() {
        std::lock_guard<std::mutex> guard(lock);
        status = -2;
        pending.Reset();
        std::vector<BYTE>().swap(encoded);
        changed.notify_one();
    }
    void enqueue(const Packet &packet) {
        std::lock_guard<std::mutex> guard(lock);
        if (status != 1 || finish || pending) return;
        pending = packet.sample;
        timestamp = packet.pts;
        changed.notify_one();
    }
};
struct FerroWindowsRecording { std::shared_ptr<Recording> value; };

static bool recording_profile(UINT w, UINT h, UINT n, UINT d) {
    return w >= 16 && h >= 16 && w <= 1920 && h <= 1080 && !(w % 2) && !(h % 2) &&
        n && d && n <= 60ull * d;
}

static bool has_h264_encoder() {
    MFT_REGISTER_TYPE_INFO output{MFMediaType_Video, MFVideoFormat_H264};
    IMFActivate **encoders = nullptr;
    UINT32 count = 0;
    HRESULT result = MFTEnumEx(MFT_CATEGORY_VIDEO_ENCODER, MFT_ENUM_FLAG_SYNCMFT,
                               nullptr, &output, &encoders, &count);
    for (UINT32 i = 0; i < count; ++i) encoders[i]->Release();
    CoTaskMemFree(encoders);
    return SUCCEEDED(result) && count != 0;
}

static ComPtr<IMFMediaType> recording_type(const Recording &r, REFGUID subtype) {
    ComPtr<IMFMediaType> type;
    check(MFCreateMediaType(&type));
    check(type->SetGUID(MF_MT_MAJOR_TYPE, MFMediaType_Video));
    check(type->SetGUID(MF_MT_SUBTYPE, subtype));
    check(type->SetUINT32(MF_MT_INTERLACE_MODE, MFVideoInterlace_Progressive));
    check(MFSetAttributeSize(type.Get(), MF_MT_FRAME_SIZE, r.width, r.height));
    check(MFSetAttributeRatio(type.Get(), MF_MT_FRAME_RATE, r.fps_n, r.fps_d));
    check(MFSetAttributeRatio(type.Get(), MF_MT_PIXEL_ASPECT_RATIO, 1, 1));
    return type;
}

static ComPtr<IMFSample> recording_sample(IMFSample *source, const Recording &r,
                                         LONGLONG pts, LONGLONG duration) {
    ComPtr<IMFMediaBuffer> source_buffer;
    check(source->GetBufferByIndex(0, &source_buffer));
    ComPtr<IMF2DBuffer> pixels;
    check(source_buffer.As(&pixels));
    DWORD length = r.width * r.height * 4;
    DWORD contiguous = 0;
    check(pixels->GetContiguousLength(&contiguous));
    if (contiguous != length) throw Failure{MF_E_INVALIDMEDIATYPE};

    ComPtr<IMFMediaBuffer> buffer;
    check(MFCreateMemoryBuffer(length, &buffer));
    BYTE *data = nullptr;
    check(buffer->Lock(&data, nullptr, nullptr));
    // The Sink Writer negotiates from linear RGB to H.264. Its copy and
    // normalized timestamps belong to a separate sample from the preview.
    HRESULT copied = pixels->ContiguousCopyTo(data, length);
    buffer->Unlock();
    check(copied);
    check(buffer->SetCurrentLength(length));

    ComPtr<IMFSample> sample;
    check(MFCreateSample(&sample));
    check(sample->AddBuffer(buffer.Get()));
    check(sample->SetSampleTime(pts));
    check(sample->SetSampleDuration(duration));
    return sample;
}

static void encode_recording(const std::shared_ptr<Recording> &owner) {
    auto &r = *owner;
    HRESULT initialized = CoInitializeEx(nullptr, COINIT_MULTITHREADED);
    try {
        check(initialized);
        auto stream = Microsoft::WRL::Make<MovieStream>();
        if (!stream) throw Failure{E_OUTOFMEMORY};
        ComPtr<IMFByteStream> bytes;
        check(MFCreateMFByteStreamOnStream(stream.Get(), &bytes));
        ComPtr<IMFAttributes> attributes;
        check(MFCreateAttributes(&attributes, 1));
        check(attributes->SetGUID(MF_TRANSCODE_CONTAINERTYPE, MFTranscodeContainerType_MPEG4));
        ComPtr<IMFSinkWriter> writer;
        check(MFCreateSinkWriterFromURL(nullptr, bytes.Get(), attributes.Get(), &writer));
        auto output = recording_type(r, MFVideoFormat_H264);
        check(output->SetUINT32(MF_MT_AVG_BITRATE, 8000000));
        DWORD track = 0;
        check(writer->AddStream(output.Get(), &track));
        auto input = recording_type(r, MFVideoFormat_RGB32);
        check(input->SetUINT32(MF_MT_DEFAULT_STRIDE, r.width * 4));
        check(writer->SetInputMediaType(track, input.Get(), nullptr));
        check(writer->BeginWriting());
        {
            std::lock_guard<std::mutex> guard(r.lock);
            if (r.status == 0) r.status = 1;
        }

        auto deadline = Clock::now() + std::chrono::seconds(60);
        LONGLONG first = -1, end = 0;
        while (true) {
            ComPtr<IMFSample> source;
            LONGLONG timestamp;
            {
                std::unique_lock<std::mutex> guard(r.lock);
                r.changed.wait_until(guard, deadline, [&] { return r.status < 0 || r.finish || r.pending; });
                if (r.status < 0 || !r.pending || Clock::now() >= deadline) break;
                source = std::move(r.pending);
                timestamp = r.timestamp;
            }
            if (first < 0) first = std::max<LONGLONG>(0, timestamp);
            LONGLONG pts = std::max<LONGLONG>(0, timestamp) - first;
            if (pts >= movie_time_limit) break;
            if (pts < end) continue;
            auto duration = std::min(r.frame_duration, movie_time_limit - pts);
            auto sample = recording_sample(source.Get(), r, pts, duration);
            source.Reset();
            check(writer->WriteSample(track, sample.Get()));
            end = pts + duration;
        }

        bool cancelled;
        {
            std::lock_guard<std::mutex> guard(r.lock);
            cancelled = r.status < 0;
        }
        if (!cancelled) {
            if (!end) throw Failure{MF_E_INVALIDREQUEST};
            check(writer->Finalize());
            writer.Reset();
            bytes.Reset();
            auto encoded = stream->take();
            if (encoded.empty()) throw Failure{E_FAIL};
            std::lock_guard<std::mutex> guard(r.lock);
            if (r.status == 1) {
                r.pending.Reset();
                r.encoded = std::move(encoded);
                r.info.width = r.width;
                r.info.height = r.height;
                r.info.duration_ns = end * 100;
                r.info.length = r.encoded.size();
                r.worker_done = true;
                r.status = 2;
            }
        }
    } catch (Failure failure) {
        std::lock_guard<std::mutex> guard(r.lock);
        if (r.status >= 0) {
            r.pending.Reset();
            strncpy_s(r.info.error, hresult(failure.hr).c_str(), _TRUNCATE);
            r.status = -1;
        }
    } catch (...) {
        std::lock_guard<std::mutex> guard(r.lock);
        if (r.status >= 0) {
            r.pending.Reset();
            strncpy_s(r.info.error, "Native recording worker failed", _TRUNCATE);
            r.status = -1;
        }
    }
    if (SUCCEEDED(initialized)) CoUninitialize();
    r.worker_done = true;
}
