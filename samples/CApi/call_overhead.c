/* The cost of calls through the C ABI of FerroUI from C (docs/c-abi.md, section 13): the
 * operations of the Rust benchmark of the crate (cargo bench -p ferroui-capi
 * --no-default-features --bench call_overhead) that a C program makes: a property write and a
 * read, the call of the frame dispatcher, and frames of 100 text blocks (one changes; the same
 * as memo elements reused through ferro_frame_reuse), and signals and the event ring shared
 * through memory (section 14): a store and an apply against the setter, 100 stores and one apply
 * against 100 setters, a load of a watched value against the getter, a raise into the ring and
 * its drain against a raise to a C callback. No window: the thread is made the UI thread with
 * ferro_init. The median of 9 samples, in nanoseconds per operation.
 *
 *   make -C samples/CApi bench CARGO_FLAGS="--release --no-default-features" PROFILE=release
 *
 * Built against the dynamic library, every call crosses into it; built against the static
 * library with link-time optimisation across the two languages (section 13), the small
 * functions of the ABI can be inlined into the C program. */

#define _POSIX_C_SOURCE 200809L

#include <stdatomic.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>

#include "ferroui.h"

#define SAMPLES 9

static int64_t now_ns(void) {
    struct timespec time;
    clock_gettime(CLOCK_MONOTONIC, &time);
    return (int64_t)time.tv_sec * 1000000000 + time.tv_nsec;
}

static int compare(const void *a, const void *b) {
    double x = *(const double *)a, y = *(const double *)b;
    return (x > y) - (x < y);
}

typedef void (*operation)(int64_t i);

/* The median time of one operation: `iterations` operations per sample, after one sample of
 * warm-up. */
static double measure(operation run, int64_t iterations) {
    double samples[SAMPLES];
    for (int64_t i = 0; i < iterations; i++) run(i);
    for (int s = 0; s < SAMPLES; s++) {
        int64_t start = now_ns();
        for (int64_t i = 0; i < iterations; i++) run(i);
        samples[s] = (double)(now_ns() - start) / (double)iterations;
    }
    qsort(samples, SAMPLES, sizeof samples[0], compare);
    return samples[SAMPLES / 2];
}

static FerroHandle text_block, root;
static int64_t text_type, panel_type, font_size, text;
static volatile double sink;
static char labels[100][16];
static const char *texts[2] = {"Hello, World!", "Clicked 1 time"};

static void write_font_size(int64_t i) {
    ferro_set_f64(text_block, font_size, (i & 1) ? 13.0 : 12.0);
}

static void read_font_size(int64_t i) {
    (void)i;
    sink = ferro_get_f64_or(text_block, font_size, -1.0);
}

static int64_t dispatcher(int64_t tag, FerroHandle sender, int64_t arg) {
    (void)sender;
    (void)arg;
    return tag;
}

static void call_dispatcher(int64_t i) {
    sink = (double)ferro_frame_dispatch(i & 1, 0, 0);
}

static void frame_100(int64_t i) {
    ferro_frame_begin();
    ferro_frame_open(panel_type, 0);
    for (int n = 0; n < 100; n++) {
        ferro_frame_open(text_type, n + 1);
        ferro_frame_str(text, n == 0 ? texts[i & 1] : labels[n], -1);
        ferro_frame_close();
    }
    ferro_frame_close();
    ferro_frame_commit(root);
}

static void memo_frame_100(int64_t i) {
    ferro_frame_begin();
    ferro_frame_open(panel_type, 0);
    for (int n = 0; n < 100; n++) {
        int64_t version = n == 0 ? (i & 1) + 1 : 1;
        if (ferro_frame_reuse(root, n + 1, version, 0) >= 0) continue;
        ferro_frame_open(text_type, n + 1);
        ferro_frame_memo(version, 0);
        ferro_frame_str(text, n == 0 ? texts[i & 1] : labels[n], -1);
        ferro_frame_close();
    }
    ferro_frame_close();
    ferro_frame_commit(root);
}

static _Atomic uint64_t *values, *dirty;
static _Atomic uint64_t *ring_head, *ring_tail;
static int64_t size_slot;
static FerroHandle blocks[100];
static int64_t slots[100];
static FerroHandle ringed, called;
static int64_t click, clicks;

static uint64_t bits_of(double value) {
    uint64_t bits;
    memcpy(&bits, &value, sizeof bits);
    return bits;
}

static void store(int64_t slot, double value) {
    atomic_store_explicit(&values[slot], bits_of(value), memory_order_relaxed);
    atomic_fetch_or_explicit(&dirty[slot / 64], 1ull << (slot % 64), memory_order_release);
}

static void signal_store(int64_t i) {
    store(size_slot, (i & 1) ? 13.0 : 12.0);
    ferro_signals_apply();
}

static void setters_100(int64_t i) {
    for (int n = 0; n < 100; n++) ferro_set_f64(blocks[n], font_size, (i & 1) ? 13.0 : 12.0);
}

static void signals_100(int64_t i) {
    for (int n = 0; n < 100; n++) store(slots[n], (i & 1) ? 13.0 : 12.0);
    ferro_signals_apply();
}

static void signal_load(int64_t i) {
    (void)i;
    uint64_t bits = atomic_load_explicit(&values[size_slot], memory_order_acquire);
    double value;
    memcpy(&value, &bits, sizeof value);
    sink = value;
}

static int64_t on_click(int64_t token, FerroHandle sender, int64_t arg) {
    (void)sender;
    (void)arg;
    clicks += token;
    return 0;
}

static void raise_callback(int64_t i) {
    (void)i;
    ferro_raise(called, click);
}

static void raise_ring(int64_t i) {
    (void)i;
    ferro_raise(ringed, click);
    uint64_t end = atomic_load_explicit(ring_head, memory_order_acquire);
    clicks += (int64_t)(end - atomic_load_explicit(ring_tail, memory_order_relaxed));
    atomic_store_explicit(ring_tail, end, memory_order_release);
}

int main(void) {
    if (ferro_init() != FERRO_OK) {
        fprintf(stderr, "ferro_init: %s\n", ferro_last_error());
        return 1;
    }
    text_type = ferro_type_id("TextBlock", -1);
    panel_type = ferro_type_id("StackPanel", -1);
    font_size = ferro_property_id("FontSize", -1);
    text = ferro_property_id("Text", -1);
    text_block = ferro_create(text_type);
    root = ferro_create(ferro_type_id("Border", -1));
    for (int n = 0; n < 100; n++) snprintf(labels[n], sizeof labels[n], "Item %d", n);
    ferro_frame_dispatcher(dispatcher);
    printf("call_overhead (C): the median of %d samples\n", SAMPLES);
    printf("%-44s %9.1f ns/op\n", "write FontSize (double)", measure(write_font_size, 1000000));
    printf("%-44s %9.1f ns/op\n", "read FontSize with a fallback", measure(read_font_size, 1000000));
    printf("%-44s %9.1f ns/op\n", "call the frame dispatcher", measure(call_dispatcher, 1000000));
    printf("%-44s %9.1f ns/op\n", "frame: 100 text blocks, one changes", measure(frame_100, 10000));
    ferro_frame_clear(root);
    printf("%-44s %9.1f ns/op\n", "frame: 100 memo text blocks, one changes", measure(memo_frame_100, 10000));
    ferro_frame_clear(root);

    const FerroSignalTable *table = ferro_signal_table(1024);
    values = (_Atomic uint64_t *)table->values;
    dirty = (_Atomic uint64_t *)table->dirty;
    size_slot = ferro_signal_new(FERRO_SIGNAL_F64);
    ferro_signal_set_f64(size_slot, 12.0);
    ferro_signal_bind(size_slot, text_block, font_size);
    ferro_signals_apply();
    printf("%-44s %9.1f ns/op\n", "signal: a store and an apply (FontSize)", measure(signal_store, 1000000));
    for (int n = 0; n < 100; n++) {
        blocks[n] = ferro_create(text_type);
        slots[n] = ferro_signal_new(FERRO_SIGNAL_F64);
        ferro_signal_set_f64(slots[n], 12.0);
        ferro_signal_bind(slots[n], blocks[n], font_size);
    }
    ferro_signals_apply();
    printf("%-44s %9.1f ns/op\n", "100 setter calls", measure(setters_100, 10000));
    printf("%-44s %9.1f ns/op\n", "signal: 100 stores and one apply", measure(signals_100, 10000));
    int64_t watched = ferro_signal_new(FERRO_SIGNAL_F64);
    ferro_signal_watch(watched, text_block, font_size);
    size_slot = watched;
    printf("%-44s %9.1f ns/op\n", "signal: a load of a watched value", measure(signal_load, 1000000));

    const FerroEventRing *ring = ferro_event_ring(1024);
    ring_head = (_Atomic uint64_t *)ring->head;
    ring_tail = (_Atomic uint64_t *)ring->tail;
    click = ferro_event_id("Click", -1);
    called = ferro_create(ferro_type_id("Button", -1));
    ringed = ferro_create(ferro_type_id("Button", -1));
    ferro_subscribe(called, click, on_click, 1);
    ferro_subscribe_ring(ringed, click, 1);
    printf("%-44s %9.1f ns/op\n", "raise Click to a C callback", measure(raise_callback, 1000000));
    printf("%-44s %9.1f ns/op\n", "raise Click into the ring, drained", measure(raise_ring, 1000000));
    return 0;
}
