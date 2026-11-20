/* Tests of the C ABI of FerroUI, from C (docs/c-abi.md, section 9): status codes, the UI thread
 * rule, bad handles, ID lookups, string lengths, values, children, events, posted work, getters
 * with a fallback, timers, the data of the event being handled and the focus, frames, and signals
 * shared through memory (section 14). No
 * window is opened: the thread is made the UI thread with ferro_init and driven with
 * ferro_run_jobs, so the tests run without a display and without the desktop platform.
 *
 *   make -C samples/CApi test */

#define _POSIX_C_SOURCE 200809L

#include <math.h>
#include <pthread.h>
#include <stdatomic.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>

#include "ferroui.h"

static int failures;
static int checks;

#define CHECK(condition)                                                                         \
    do {                                                                                         \
        checks++;                                                                                \
        if (!(condition)) {                                                                      \
            failures++;                                                                          \
            fprintf(stderr, "%s:%d: CHECK(%s) failed; last error: %s\n", __FILE__, __LINE__,    \
                    #condition, ferro_last_error());                                             \
        }                                                                                        \
    } while (0)

#define CHECK_STATUS(expected, call) CHECK((call) == (expected))

static FerroHandle create(const char *type) {
    FerroHandle object = ferro_create(ferro_type_id(type, -1));
    CHECK(object != 0);
    return object;
}

static int64_t prop(const char *name) {
    return ferro_property_id(name, -1);
}

static void test_ids(void) {
    int64_t button = ferro_type_id("Button", -1);
    CHECK(button > 0);
    CHECK(ferro_type_id("Button", 6) == button);
    CHECK(ferro_type_id("ButtonXYZ", 6) == button);
    CHECK(ferro_type_id("FerroUI.Controls.Button", -1) == button);
    CHECK(ferro_type_id("NoSuchType", -1) == FERRO_NOT_FOUND);
    CHECK(ferro_last_error_code() == FERRO_NOT_FOUND);
    CHECK(strstr(ferro_last_error(), "NoSuchType") != NULL);
    CHECK(ferro_property_id("Text", -1) == ferro_property_id("Text", 4));
    CHECK(ferro_property_id("Text", -1) > 0);
    CHECK(ferro_property_id(NULL, 0) == FERRO_INVALID_ARGUMENT);
    CHECK(ferro_property_id(NULL, 2) == FERRO_NULL_POINTER);
    CHECK(ferro_event_id("Click", -1) > 0);
    const char not_utf8[] = {'a', (char)0xff, 0};
    CHECK(ferro_property_id(not_utf8, -1) == FERRO_INVALID_UTF8);
}

static void test_handles(void) {
    FerroHandle text = create("TextBlock");
    CHECK(ferro_type_of(text) == ferro_type_id("TextBlock", -1));
    CHECK_STATUS(FERRO_OK, ferro_retain(text));
    CHECK_STATUS(FERRO_OK, ferro_release(text));
    CHECK_STATUS(FERRO_OK, ferro_release(text));
    CHECK_STATUS(FERRO_BAD_HANDLE, ferro_release(text));
    CHECK_STATUS(FERRO_BAD_HANDLE, ferro_set_f64(text, prop("FontSize"), 10));
    CHECK_STATUS(FERRO_BAD_HANDLE, ferro_release(0));
    CHECK_STATUS(FERRO_BAD_HANDLE, ferro_set_f64((FerroHandle)0x123456789ull, prop("FontSize"), 10));
    CHECK(ferro_create(0) == 0);
    CHECK(ferro_last_error_code() == FERRO_NOT_FOUND);
}

static void test_values(void) {
    FerroHandle text = create("TextBlock");
    CHECK_STATUS(FERRO_OK, ferro_set_str(text, prop("Text"), "Hello, World!", -1));
    int64_t length = -1;
    char small[4];
    CHECK_STATUS(FERRO_BUFFER_TOO_SMALL, ferro_get_str(text, prop("Text"), small, sizeof small, &length));
    CHECK(length == 13);
    CHECK_STATUS(FERRO_BUFFER_TOO_SMALL, ferro_get_str(text, prop("Text"), NULL, 0, &length));
    char buffer[32];
    CHECK_STATUS(FERRO_OK, ferro_get_str(text, prop("Text"), buffer, sizeof buffer, &length));
    CHECK(strcmp(buffer, "Hello, World!") == 0);
    /* A length takes that many bytes of the string. */
    CHECK_STATUS(FERRO_OK, ferro_set_str(text, prop("Text"), "Hello, World!", 5));
    CHECK(strcmp(ferro_get_cstr(text, prop("Text")), "Hello") == 0);

    double size = 0;
    CHECK_STATUS(FERRO_OK, ferro_set_f64(text, prop("FontSize"), 28));
    CHECK_STATUS(FERRO_OK, ferro_get_f64(text, prop("FontSize"), &size));
    CHECK(size == 28);
    bool enabled = true;
    CHECK_STATUS(FERRO_OK, ferro_set_bool(text, prop("IsEnabled"), false));
    CHECK_STATUS(FERRO_OK, ferro_get_bool(text, prop("IsEnabled"), &enabled));
    CHECK(!enabled);
    int64_t row = 0;
    CHECK_STATUS(FERRO_OK, ferro_set_i64(text, prop("Grid.Row"), 3));
    CHECK_STATUS(FERRO_OK, ferro_get_i64(text, prop("Grid.Row"), &row));
    CHECK(row == 3);
    CHECK_STATUS(FERRO_INVALID_VALUE, ferro_set_i64(text, prop("Grid.Row"), -1));
    CHECK_STATUS(FERRO_READ_ONLY, ferro_set_f64(text, prop("Bounds"), 1));
    CHECK_STATUS(FERRO_NOT_FOUND, ferro_set_f64(text, prop("Orientation"), 1));
    CHECK_STATUS(FERRO_NULL_POINTER, ferro_get_f64(text, prop("FontSize"), NULL));

    FerroHandle panel = create("StackPanel");
    CHECK_STATUS(FERRO_OK, ferro_set_str(panel, prop("Orientation"), "Horizontal", -1));
    CHECK(strcmp(ferro_get_cstr(panel, prop("Orientation")), "Horizontal") == 0);
    CHECK_STATUS(FERRO_TYPE_MISMATCH, ferro_set_handle(text, prop("FontSize"), panel));
    ferro_release(panel);
    ferro_release(text);
}

static int64_t clicks;

static int64_t on_click(int64_t token, FerroHandle sender, int64_t arg) {
    CHECK(arg == 0);
    clicks += token;
    /* A call back into the ABI from inside a handler. */
    CHECK_STATUS(FERRO_OK, ferro_set_str(sender, prop("Content"), "Clicked", -1));
    return 0;
}

static int released;

static int64_t on_click_data(void *user_data, FerroHandle sender, int64_t arg) {
    (void)sender;
    (void)arg;
    *(int *)user_data += 1;
    return 1; /* handled */
}

static void release_data(void *user_data) {
    (void)user_data;
    released++;
}

static void test_tree_and_events(void) {
    FerroHandle panel = create("StackPanel");
    FerroHandle button = create("Button");
    FerroHandle text = create("TextBlock");
    CHECK_STATUS(FERRO_OK, ferro_add_child(panel, button));
    CHECK_STATUS(FERRO_NOT_SUPPORTED, ferro_add_child(text, button));
    CHECK_STATUS(FERRO_OK, ferro_add_child(button, text));
    CHECK_STATUS(FERRO_INVALID_STATE, ferro_add_child(button, text));

    int64_t click = ferro_event_id("Click", -1);
    int64_t subscription = ferro_subscribe(button, click, on_click, 5);
    CHECK(subscription > 0);
    CHECK_STATUS(FERRO_OK, ferro_raise(button, click));
    CHECK(clicks == 5);
    CHECK(strcmp(ferro_get_cstr(button, prop("Content")), "Clicked") == 0);

    int count = 0;
    int64_t with_data = ferro_subscribe_data(panel, ferro_event_id("Button.Click", -1), on_click_data, &count, release_data);
    CHECK(with_data > 0);
    CHECK_STATUS(FERRO_OK, ferro_raise(button, click));
    CHECK(clicks == 10);
    CHECK(count == 1);
    CHECK_STATUS(FERRO_OK, ferro_unsubscribe(with_data));
    CHECK(released == 1);
    CHECK_STATUS(FERRO_OK, ferro_unsubscribe(subscription));
    CHECK_STATUS(FERRO_NOT_FOUND, ferro_unsubscribe(subscription));
    CHECK(ferro_subscribe(button, ferro_event_id("NoSuchEvent", -1), on_click, 0) == FERRO_NOT_FOUND);

    ferro_release(text);
    ferro_release(button);
    ferro_release(panel);
}

static FerroHandle shared_handle;
static int64_t posted;

static int64_t on_posted(int64_t token) {
    CHECK(ferro_is_ui_thread());
    posted += token;
    return 0;
}

static void *other_thread(void *argument) {
    (void)argument;
    CHECK(!ferro_is_ui_thread());
    CHECK_STATUS(FERRO_WRONG_THREAD, ferro_set_f64(shared_handle, prop("FontSize"), 3));
    CHECK_STATUS(FERRO_WRONG_THREAD, ferro_release(shared_handle));
    CHECK(ferro_create(ferro_type_id("Button", -1)) == 0);
    CHECK(ferro_last_error_code() == FERRO_WRONG_THREAD);
    CHECK_STATUS(FERRO_WRONG_THREAD, ferro_init());
    CHECK_STATUS(FERRO_WRONG_THREAD, ferro_run_jobs());
    /* IDs and posting work on any thread. */
    CHECK(ferro_type_id("TextBlock", -1) > 0);
    CHECK_STATUS(FERRO_OK, ferro_post(on_posted, 7));
    CHECK_STATUS(FERRO_NULL_POINTER, ferro_post(NULL, 0));
    return NULL;
}

static void test_threads(void) {
    shared_handle = create("TextBlock");
    pthread_t thread;
    CHECK(pthread_create(&thread, NULL, other_thread, NULL) == 0);
    pthread_join(thread, NULL);
    CHECK(posted == 0);
    CHECK_STATUS(FERRO_OK, ferro_run_jobs());
    CHECK(posted == 7);
    CHECK_STATUS(FERRO_OK, ferro_release(shared_handle));
}

/* Getters that return the value, for languages that cannot pass a pointer to a result. */
static void test_getters_with_a_fallback(void) {
    FerroHandle text = create("TextBlock");
    CHECK_STATUS(FERRO_OK, ferro_set_f64(text, prop("FontSize"), 17));
    CHECK(ferro_get_f64_or(text, prop("FontSize"), -1) == 17);
    CHECK(ferro_get_f64_or(text, prop("NoSuchProperty"), -1) == -1);
    CHECK(ferro_last_error_code() == FERRO_NOT_FOUND);
    CHECK_STATUS(FERRO_OK, ferro_set_i64(text, prop("Grid.Row"), 4));
    CHECK(ferro_get_i64_or(text, prop("Grid.Row"), -1) == 4);
    CHECK(ferro_get_i64_or(0, prop("Grid.Row"), -1) == -1);
    CHECK(ferro_last_error_code() == FERRO_BAD_HANDLE);
    CHECK_STATUS(FERRO_OK, ferro_set_bool(text, prop("IsEnabled"), false));
    CHECK(!ferro_get_bool_or(text, prop("IsEnabled"), true));
    CHECK(ferro_get_bool_or(text, prop("Text"), true));
    CHECK(ferro_last_error_code() == FERRO_TYPE_MISMATCH);
    ferro_release(text);
}

static int64_t ticks;
static int64_t elapsed;

static int64_t on_tick(int64_t token, int64_t microseconds) {
    CHECK(ferro_is_ui_thread());
    ticks++;
    elapsed += microseconds;
    /* The token is the number of ticks after which the timer stops itself. */
    return ticks >= token ? 1 : 0;
}

static int64_t nothing(int64_t token) {
    (void)token;
    return 0;
}

/* Waits for `ms` milliseconds and runs the work of the UI thread, the timers that are due
 * included. */
static void run_for(long ms) {
    struct timespec wait = {ms / 1000, (ms % 1000) * 1000000L};
    nanosleep(&wait, NULL);
    CHECK_STATUS(FERRO_OK, ferro_post(nothing, 0));
    CHECK_STATUS(FERRO_OK, ferro_run_jobs());
}

static void test_timers(void) {
    int64_t timer = ferro_timer_start(2, on_tick, 3);
    CHECK(timer > 0);
    for (int i = 0; i < 50 && ticks < 3; i++) {
        run_for(10);
    }
    CHECK(ticks == 3);
    CHECK(elapsed >= 3 * 2000);
    run_for(20);
    CHECK(ticks == 3);
    CHECK_STATUS(FERRO_NOT_FOUND, ferro_timer_stop(timer));
    timer = ferro_timer_start(1000, on_tick, 100);
    CHECK(timer > 0);
    CHECK_STATUS(FERRO_OK, ferro_timer_stop(timer));
    CHECK(ferro_timer_start(0, on_tick, 0) == FERRO_INVALID_ARGUMENT);
    CHECK(ferro_timer_start(10, NULL, 0) == FERRO_NULL_POINTER);
}

static int64_t event_data_in_click;

static int64_t on_click_reads_event_data(int64_t token, FerroHandle sender, int64_t arg) {
    (void)token;
    (void)sender;
    (void)arg;
    /* A click carries no pointer position, no button and no key. */
    if (isnan(ferro_event_x(0)) && ferro_last_error_code() == FERRO_NOT_SUPPORTED &&
        ferro_event_button() == FERRO_NOT_SUPPORTED && ferro_event_key() == NULL) {
        event_data_in_click++;
    }
    return 0;
}

static void test_event_data_and_focus(void) {
    CHECK(isnan(ferro_event_y(0)));
    CHECK(ferro_last_error_code() == FERRO_INVALID_STATE);
    CHECK(ferro_event_button() == FERRO_INVALID_STATE);
    CHECK(ferro_event_key() == NULL);
    FerroHandle button = create("Button");
    int64_t click = ferro_event_id("Click", -1);
    CHECK(ferro_subscribe(button, click, on_click_reads_event_data, 0) > 0);
    CHECK_STATUS(FERRO_OK, ferro_raise(button, click));
    CHECK(event_data_in_click == 1);
    /* Not in a window: no focus. */
    CHECK_STATUS(FERRO_INVALID_STATE, ferro_focus(button));
    CHECK_STATUS(FERRO_BAD_HANDLE, ferro_focus(0));
    ferro_release(button);
}

static int64_t frame_tags;

static int64_t on_frame_event(int64_t tag, FerroHandle sender, int64_t arg) {
    (void)sender;
    (void)arg;
    frame_tags += tag;
    return 0;
}

/* Frames: the scalar writer, a commit, a second commit that patches, the dispatcher of tags, and
 * the same frame as bytes of the layout of the API model. */
static void test_frames(void) {
    FerroHandle root = create("StackPanel");
    int64_t text = prop("Text");
    int64_t block = ferro_type_id("TextBlock", -1);
    int64_t click = ferro_event_id("Click", -1);
    CHECK_STATUS(FERRO_OK, ferro_frame_dispatcher(on_frame_event));
    for (int count = 0; count < 2; count++) {
        char label[32];
        snprintf(label, sizeof label, "Count %d", count);
        CHECK_STATUS(FERRO_OK, ferro_frame_begin());
        CHECK_STATUS(FERRO_OK, ferro_frame_open(block, 1));
        CHECK_STATUS(FERRO_OK, ferro_frame_str(text, label, -1));
        CHECK_STATUS(FERRO_OK, ferro_frame_close());
        CHECK_STATUS(FERRO_OK, ferro_frame_open(ferro_type_id("Button", -1), 2));
        CHECK_STATUS(FERRO_OK, ferro_frame_str(prop("Content"), "+", -1));
        CHECK_STATUS(FERRO_OK, ferro_frame_on(click, 10 + count, 0));
        CHECK_STATUS(FERRO_OK, ferro_frame_close());
        CHECK_STATUS(FERRO_OK, ferro_frame_commit(root));
    }
    FerroHandle label = ferro_frame_find(root, 1);
    CHECK(label != 0);
    CHECK(strcmp(ferro_get_cstr(label, text), "Count 1") == 0);
    FerroHandle button = ferro_frame_find(root, 2);
    CHECK_STATUS(FERRO_OK, ferro_raise(button, click));
    CHECK(frame_tags == 11);
    CHECK(ferro_frame_dispatch(5, 0, 0) == 0);
    CHECK(frame_tags == 16);
    CHECK_STATUS(FERRO_OK, ferro_release(button));
    CHECK_STATUS(FERRO_OK, ferro_release(label));

    /* The same element as bytes: OPEN(type, key), TEXT(property, length, bytes), CLOSE. */
    int64_t words[] = {
        1 | (24ll << 32), block, 1,
        3 | (32ll << 32), text, 5, 0,
        2 | (8ll << 32),
    };
    memcpy(&words[6], "bytes\0\0\0", 8);
    CHECK_STATUS(FERRO_OK, ferro_frame_commit_bytes(root, (const uint8_t *)words, sizeof words));
    label = ferro_frame_find(root, 1);
    CHECK(strcmp(ferro_get_cstr(label, text), "bytes") == 0);
    CHECK(ferro_frame_find(root, 2) == 0);
    CHECK_STATUS(FERRO_OK, ferro_release(label));
    CHECK_STATUS(FERRO_INVALID_STATE, ferro_frame_commit_bytes(root, (const uint8_t *)words, 3 * 8));
    CHECK_STATUS(FERRO_OK, ferro_frame_clear(root));
    CHECK_STATUS(FERRO_OK, ferro_frame_dispatcher(NULL));
    CHECK(ferro_api_fingerprint() != 0);
    CHECK_STATUS(FERRO_OK, ferro_release(root));
}

/* Signals shared through memory (docs/c-abi.md, section 14): a store and its dirty bit, applied to
 * a bound property; a watched property read with a load; a bind record of a frame. */
static void test_signals(void) {
    const FerroSignalTable *table = ferro_signal_table(128);
    CHECK(table != NULL && table->layout == FERRO_SIGNAL_LAYOUT && table->capacity % 64 == 0);
    _Atomic uint64_t *values = (_Atomic uint64_t *)table->values;
    _Atomic uint64_t *dirty = (_Atomic uint64_t *)table->dirty;
    _Atomic uint64_t *changed = (_Atomic uint64_t *)table->changed;
    FerroHandle block = create("TextBlock");
    int64_t size = ferro_signal_new(FERRO_SIGNAL_F64);
    CHECK(size >= 0);
    CHECK_STATUS(FERRO_OK, ferro_signal_set_f64(size, 14.0));
    CHECK(ferro_signal_bind(size, block, prop("FontSize")) > 0);
    CHECK(ferro_get_f64_or(block, prop("FontSize"), 0) == 14.0);
    /* The store of a language that writes memory: the value, then the bit with release order. */
    double next = 22.0;
    uint64_t bits;
    memcpy(&bits, &next, sizeof bits);
    atomic_store_explicit(&values[size], bits, memory_order_relaxed);
    atomic_fetch_or_explicit(&dirty[size / 64], 1ull << (size % 64), memory_order_release);
    CHECK(ferro_signals_apply() >= 1);
    CHECK(ferro_get_f64_or(block, prop("FontSize"), 0) == 22.0);
    CHECK_STATUS(FERRO_INVALID_ARGUMENT, ferro_signal_set_f64(1 << 30, 1.0));
    CHECK_STATUS(FERRO_TYPE_MISMATCH, ferro_signal_set_i64(size, 1));

    /* A watched text: typed into the box, read from the slot. */
    FerroHandle box = create("TextBox");
    int64_t typed = ferro_signal_new(FERRO_SIGNAL_TEXT);
    CHECK(ferro_signal_watch(typed, box, prop("Text")) > 0);
    int64_t version = ferro_signal_version(typed);
    CHECK_STATUS(FERRO_OK, ferro_set_str(box, prop("Text"), "typed", -1));
    CHECK(ferro_signal_version(typed) == version + 1);
    CHECK(atomic_load_explicit(&values[typed], memory_order_acquire) == 5);
    CHECK((atomic_load_explicit(&changed[typed / 64], memory_order_acquire) >> (typed % 64)) & 1);
    CHECK(strcmp(ferro_signal_get_cstr(typed), "typed") == 0);

    /* A bind record: the property follows the slot with no new frame. */
    FerroHandle root = create("Border");
    CHECK_STATUS(FERRO_OK, ferro_frame_begin());
    CHECK_STATUS(FERRO_OK, ferro_frame_open(ferro_type_id("TextBlock", -1), 1));
    CHECK_STATUS(FERRO_OK, ferro_frame_bind(prop("FontSize"), size));
    CHECK_STATUS(FERRO_OK, ferro_frame_close());
    CHECK_STATUS(FERRO_OK, ferro_frame_commit(root));
    FerroHandle framed = ferro_frame_find(root, 1);
    CHECK(ferro_get_f64_or(framed, prop("FontSize"), 0) == 22.0);
    CHECK_STATUS(FERRO_OK, ferro_signal_set_f64(size, 30.0));
    CHECK(ferro_signals_apply() == 1);
    CHECK(ferro_get_f64_or(framed, prop("FontSize"), 0) == 30.0);
    CHECK(ferro_get_f64_or(block, prop("FontSize"), 0) == 30.0);
    CHECK_STATUS(FERRO_OK, ferro_release(framed));
    CHECK_STATUS(FERRO_OK, ferro_frame_clear(root));

    CHECK_STATUS(FERRO_OK, ferro_signal_free(typed));
    CHECK_STATUS(FERRO_OK, ferro_signal_free(size));
    CHECK_STATUS(FERRO_OK, ferro_release(box));
    CHECK_STATUS(FERRO_OK, ferro_release(block));
    CHECK_STATUS(FERRO_OK, ferro_release(root));
}

int main(void) {
    CHECK(ferro_abi_version() == FERRO_ABI_VERSION);
    CHECK(!ferro_is_ui_thread());
    CHECK(ferro_create(ferro_type_id("Button", -1)) == 0);
    CHECK(ferro_last_error_code() == FERRO_NOT_INITIALIZED);
    CHECK_STATUS(FERRO_OK, ferro_init());
    CHECK_STATUS(FERRO_OK, ferro_init());
    CHECK(ferro_is_ui_thread());
    CHECK_STATUS(FERRO_INVALID_ARGUMENT, ferro_app_run((const void *)1, NULL, 0));

    test_ids();
    test_handles();
    test_values();
    test_tree_and_events();
    test_threads();
    test_getters_with_a_fallback();
    test_timers();
    test_event_data_and_focus();
    test_frames();
    test_signals();

    if (failures != 0) {
        fprintf(stderr, "abi_test: %d of %d checks failed\n", failures, checks);
        return 1;
    }
    printf("abi_test: %d checks passed\n", checks);
    return 0;
}
