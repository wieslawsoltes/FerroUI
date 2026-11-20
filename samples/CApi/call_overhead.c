/* The cost of calls through the C ABI of FerroUI from C (docs/c-abi.md, section 13): the
 * operations of the Rust benchmark of the crate (cargo bench -p ferroui-capi
 * --no-default-features --bench call_overhead) that a C program makes: a property write and a
 * read, the call of the frame dispatcher, and frames of 100 text blocks (one changes; the same
 * as memo elements reused through ferro_frame_reuse). No window: the thread is made the UI
 * thread with ferro_init. The median of 9 samples, in nanoseconds per operation.
 *
 *   make -C samples/CApi bench CARGO_FLAGS="--release --no-default-features" PROFILE=release
 *
 * Built against the dynamic library, every call crosses into it; built against the static
 * library with link-time optimisation across the two languages (section 13), the small
 * functions of the ABI can be inlined into the C program. */

#define _POSIX_C_SOURCE 200809L

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
    return 0;
}
