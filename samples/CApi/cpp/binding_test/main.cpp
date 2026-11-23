// The tests of the C++ bindings over the library (no window, no display):
//
//   make -C samples/CApi test-cpp CARGO_FLAGS="--no-default-features --features type-tables"

#include <cstdio>
#include <cstdlib>
#include <stdexcept>
#include <string>
#include <utility>

#include "ferroui.hpp"

using namespace ferroui;

static int checks = 0;

#define CHECK(condition)                                                          \
    do {                                                                          \
        checks += 1;                                                              \
        if (!(condition)) {                                                       \
            std::fprintf(stderr, "%s:%d: failed: %s\n", __FILE__, __LINE__, #condition); \
            std::exit(1);                                                         \
        }                                                                         \
    } while (false)

static std::int64_t event(const char* name) {
    return ferro_event_id(name, -1);
}

static void test_values() {
    auto text = TextBlock::create();
    CHECK(static_cast<bool>(text));
    // Writes chain on the type of the object, through the members of its bases.
    text.text("Hello").font_size(20).opacity(0.5).is_visible(false);
    CHECK(text.text() == "Hello");
    CHECK(text.font_size() == 20.0);
    CHECK(text.opacity() == 0.5);
    CHECK(!text.is_visible());

    // A text longer than the first buffer of the getter, and one with an inner NUL-free UTF-8.
    const std::string longer(500, 'x');
    text.text(longer);
    CHECK(text.text() == longer);
    text.text("Za\xC5\xBC\xC3\xB3\xC5\x82\xC4\x87");
    CHECK(text.text() == "Za\xC5\xBC\xC3\xB3\xC5\x82\xC4\x87");

    auto button = Button::create();
    button.horizontal_alignment(HorizontalAlignment::Center);
    CHECK(button.horizontal_alignment() == HorizontalAlignment::Center);
}

static void test_content_and_children() {
    auto button = Button::create();
    button.content("Click me");
    CHECK(button.content() == "Click me");

    auto inner = TextBlock::create();
    button.content(inner);
    CHECK(button.content_object().handle() == inner.handle());

    auto panel = StackPanel::create();
    auto child = Border::create();
    panel.spacing(4).add_child(child);
    CHECK(ferro_panel_child_count(panel.handle()) == 1);
    panel.remove_child(child);
    CHECK(ferro_panel_child_count(panel.handle()) == 0);
}

static void test_attached_properties() {
    auto grid = Grid::create();
    auto child = Border::create();
    grid.add_child(child);
    Grid::set_row(child, 2);
    Grid::set_column(child, 1);
    CHECK(Grid::get_row(child) == 2);
    CHECK(Grid::get_column(child) == 1);
}

static void test_handles() {
    auto first = Border::create();
    auto handle = first.handle();
    {
        Border copy = first;
        CHECK(copy.handle() == handle);
        Border moved = std::move(copy);
        CHECK(moved.handle() == handle);
        CHECK(!copy);
    }
    // The copies released their references: the object is still there.
    first.width(10);
    CHECK(first.width() == 10.0);

    // The same object through another wrapper.
    auto control = first.as<Control>();
    CHECK(control.handle() == handle);
    CHECK(control.width() == 10.0);
}

static void test_events() {
    auto button = Button::create();
    const auto click = event("Click");
    int clicks = 0;
    button.on_click([&clicks](Button& sender) {
        clicks += 1;
        sender.content("Clicked");
    });
    CHECK(ferro_raise(button.handle(), click) == FERRO_OK);
    CHECK(clicks == 1);
    CHECK(button.content() == "Clicked");

    // A subscription ends with its value.
    int counted = 0;
    {
        Subscription subscription = button.subscribe_click([&counted](Button&) { counted += 1; });
        CHECK(ferro_raise(button.handle(), click) == FERRO_OK);
        CHECK(counted == 1);
    }
    CHECK(ferro_raise(button.handle(), click) == FERRO_OK);
    CHECK(counted == 1);
    CHECK(clicks == 3);

    // What a handler throws stops at the boundary of the C ABI.
    button.on_click([](Button&) { throw std::runtime_error("thrown by the handler"); });
    CHECK(ferro_raise(button.handle(), click) == FERRO_OK);
    CHECK(clicks == 4);
}

static void test_errors() {
    auto border = Border::create();
    bool thrown = false;
    try {
        // `Text` is not a property of a border.
        TextBlock wrong = border.as<TextBlock>();
        wrong.text("no");
    } catch (const Error& error) {
        thrown = true;
        CHECK(error.status() == FERRO_NOT_FOUND);
        CHECK(std::string(error.what()).find("Text") != std::string::npos);
    }
    CHECK(thrown);
}

int main() {
    if (ferro_init() != FERRO_OK) {
        std::fprintf(stderr, "ferro_init: %s\n", ferro_last_error());
        return 1;
    }
    try {
        check_library();
        test_values();
        test_content_and_children();
        test_attached_properties();
        test_handles();
        test_events();
        test_errors();
    } catch (const std::exception& error) {
        std::fprintf(stderr, "unexpected: %s\n", error.what());
        return 1;
    }
    std::printf("ok: %d checks\n", checks);
    return 0;
}
