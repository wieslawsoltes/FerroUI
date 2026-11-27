// Hello, World! in Go through the generated bindings of FerroUI (bindings/README.md): a window
// with a text and a button whose clicks change its text, the Go version of hello_world.c.
//
//	make -C samples/CApi run-go
//
// FERROUI_SMOKE_EXIT_MS=<n> closes the window after n milliseconds.
package main

import (
	"errors"
	"fmt"
	"os"

	"hello_world/ferroui"
)

func start() error {
	text, err := ferroui.NewTextBlock()
	if err != nil {
		return err
	}
	button, err := ferroui.NewButton()
	if err != nil {
		return err
	}
	panel, err := ferroui.NewStackPanel()
	if err != nil {
		return err
	}
	window, err := ferroui.NewWindow()
	if err != nil {
		return err
	}
	clicks := 0
	_, subscribed := button.OnClick(func(sender ferroui.Button) {
		clicks++
		label := fmt.Sprintf("Clicked %d times", clicks)
		if clicks == 1 {
			label = "Clicked 1 time"
		}
		if err := sender.SetContent(label); err != nil {
			fmt.Fprintln(os.Stderr, err)
		}
	})
	err = errors.Join(
		subscribed,
		text.SetText("Hello, World!"),
		text.SetFontSize(28),
		button.SetHorizontalAlignment(ferroui.HorizontalAlignmentCenter),
		button.SetContent("Click me"),
		panel.SetHorizontalAlignment(ferroui.HorizontalAlignmentCenter),
		panel.SetVerticalAlignment(ferroui.VerticalAlignmentCenter),
		panel.SetSpacing(12),
		panel.AddChild(text),
		panel.AddChild(button),
		window.SetTitle("Hello, World!"),
		window.SetWidth(400),
		window.SetHeight(240),
		window.AddChild(panel),
		window.Show(),
	)
	// The window holds its tree and the platform holds the open window.
	return errors.Join(err, text.Release(), button.Release(), panel.Release(), window.Release())
}

func main() {
	if err := ferroui.RunApp(start); err != nil {
		fmt.Fprintln(os.Stderr, err)
		os.Exit(1)
	}
}
