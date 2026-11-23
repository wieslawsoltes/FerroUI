// Hello, World! in C++ through the generated bindings of FerroUI (bindings/README.md): a window
// with a text and a button whose clicks change its text, the C++ version of hello_world.c.
//
//   make -C samples/CApi run-cpp
//
// FERROUI_SMOKE_EXIT_MS=<n> closes the window after n milliseconds.

#include <cstdio>
#include <exception>
#include <string>

#include "ferroui/ferroui.hpp"

using namespace ferroui;

int main() {
    try {
        run_app([] {
            auto text = TextBlock::create();
            text.text("Hello, World!").font_size(28);

            auto button = Button::create();
            button.horizontal_alignment(HorizontalAlignment::Center).content("Click me");
            button.on_click([clicks = 0](Button& sender) mutable {
                clicks += 1;
                sender.content("Clicked " + std::to_string(clicks) + (clicks == 1 ? " time" : " times"));
            });

            auto panel = StackPanel::create();
            panel.horizontal_alignment(HorizontalAlignment::Center)
                .vertical_alignment(VerticalAlignment::Center)
                .spacing(12)
                .add_child(text)
                .add_child(button);

            auto window = Window::create();
            window.title("Hello, World!").width(400).height(240).add_child(panel).show();
            // The window holds its tree and the platform holds the open window: the handles
            // here are released when they go out of scope.
        });
    } catch (const std::exception& error) {
        std::fprintf(stderr, "%s\n", error.what());
        return 1;
    }
    return 0;
}
