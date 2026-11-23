// The cost of the C++ bindings against the C calls they wrap (#636): a property write and a read,
// a text write, and the dispatch of a routed event. The median of 9 samples, in nanoseconds per
// operation. No window: the thread is made the UI thread with `ferro_init`.
//
//   make -C samples/CApi bench-cpp CARGO_FLAGS="--release --no-default-features --features type-tables" PROFILE=release

#include <algorithm>
#include <array>
#include <chrono>
#include <cstdio>

#include "ferroui.hpp"

using namespace ferroui;

constexpr int samples = 9;
constexpr std::int64_t iterations = 200000;
constexpr std::int64_t event_iterations = 20000;

static volatile double sink;

template <class Operation>
static double measure(std::int64_t count, Operation operation) {
    using Clock = std::chrono::steady_clock;
    std::array<double, samples> times{};
    for (std::int64_t i = 0; i < count; i++) {
        operation(i);
    }
    for (double& time : times) {
        const auto start = Clock::now();
        for (std::int64_t i = 0; i < count; i++) {
            operation(i);
        }
        const std::chrono::duration<double, std::nano> elapsed = Clock::now() - start;
        time = elapsed.count() / static_cast<double>(count);
    }
    std::sort(times.begin(), times.end());
    return times[samples / 2];
}

static void row(const char* name, double c, double cpp) {
    std::printf("%-24s %10.1f %10.1f %+9.1f\n", name, c, cpp, cpp - c);
}

int main() {
    if (ferro_init() != FERRO_OK) {
        std::fprintf(stderr, "ferro_init: %s\n", ferro_last_error());
        return 1;
    }
    check_library();
    auto text = TextBlock::create();
    auto button = Button::create();
    const FerroHandle handle = text.handle();
    const std::int64_t font_size = ferro_property_id("FontSize", -1);
    const std::int64_t text_id = ferro_property_id("Text", -1);
    const std::int64_t click = ferro_event_id("Click", -1);

    std::printf("%-24s %10s %10s %9s\n", "ns per operation", "C", "C++", "added");
    row("write a number",
        measure(iterations, [&](std::int64_t i) { ferro_set_f64(handle, font_size, (i & 1) ? 13.0 : 12.0); }),
        measure(iterations, [&](std::int64_t i) { text.font_size((i & 1) ? 13.0 : 12.0); }));
    row("read a number",
        measure(iterations, [&](std::int64_t) {
            double value = 0.0;
            ferro_get_f64(handle, font_size, &value);
            sink = value;
        }),
        measure(iterations, [&](std::int64_t) { sink = text.font_size(); }));
    row("write a text",
        measure(iterations, [&](std::int64_t i) { ferro_set_str(handle, text_id, (i & 1) ? "one" : "two", 3); }),
        measure(iterations, [&](std::int64_t i) { text.text((i & 1) ? "one" : "two"); }));

    std::int64_t calls = 0;
    const double raised_without = measure(event_iterations, [&](std::int64_t) { ferro_raise(button.handle(), click); });
    button.on_click([&calls](Button&) { calls += 1; });
    const double raised_with = measure(event_iterations, [&](std::int64_t) { ferro_raise(button.handle(), click); });
    row("raise, then a handler", raised_without, raised_with);
    sink = static_cast<double>(calls);
    return 0;
}
