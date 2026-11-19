/* Tests of the C ABI of FerroUI, from C (docs/c-abi.md, section 9): status codes, the UI thread
 * rule, bad handles, ID lookups, string lengths, values, children, events and posted work. No
 * window is opened: the thread is made the UI thread with ferro_init and driven with
 * ferro_run_jobs, so the tests run without a display and without the desktop platform.
 *
 *   make -C samples/CApi test */

#include <pthread.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

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

    if (failures != 0) {
        fprintf(stderr, "abi_test: %d of %d checks failed\n", failures, checks);
        return 1;
    }
    printf("abi_test: %d checks passed\n", checks);
    return 0;
}
