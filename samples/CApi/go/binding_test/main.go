// The tests of the Go bindings over the library (no window, no display):
//
//	make -C samples/CApi test-go CARGO_FLAGS="--no-default-features --features type-tables"
package main

import (
	"errors"
	"fmt"
	"os"
	"strings"

	"ferroui.local/ferroui"
)

var checks int

func check(condition bool, what string) {
	checks++
	if !condition {
		fmt.Fprintln(os.Stderr, "failed:", what)
		os.Exit(1)
	}
}

func must[T any](value T, err error) T {
	if err != nil {
		fmt.Fprintln(os.Stderr, "unexpected:", err)
		os.Exit(1)
	}
	return value
}

func ok(err error) {
	if err != nil {
		fmt.Fprintln(os.Stderr, "unexpected:", err)
		os.Exit(1)
	}
}

func testValues() {
	text := must(ferroui.NewTextBlock())
	defer text.Release()
	ok(text.SetText("Hello"))
	ok(text.SetFontSize(20))
	ok(text.SetOpacity(0.5))
	ok(text.SetIsVisible(false))
	check(text.Text() == "Hello", "text")
	check(text.FontSize() == 20, "font size")
	check(text.Opacity() == 0.5, "a property of a base")
	check(!text.IsVisible(), "a boolean")

	longer := strings.Repeat("x", 500)
	ok(text.SetText(longer))
	check(text.Text() == longer, "a text longer than the first buffer")
	ok(text.SetText("Zażółć"))
	check(text.Text() == "Zażółć", "UTF-8")
	ok(text.SetText(""))
	check(text.Text() == "", "an empty text")

	button := must(ferroui.NewButton())
	defer button.Release()
	ok(button.SetHorizontalAlignment(ferroui.HorizontalAlignmentCenter))
	check(button.HorizontalAlignment() == ferroui.HorizontalAlignmentCenter, "an enumeration")
}

func testContentAndChildren() {
	button := must(ferroui.NewButton())
	defer button.Release()
	ok(button.SetContent("Click me"))
	check(button.Content() == "Click me", "content as text")

	inner := must(ferroui.NewTextBlock())
	defer inner.Release()
	ok(button.SetContentObject(inner))
	held := button.ContentObject()
	check(held.Handle() == inner.Handle(), "content as an object")
	ok(held.Release())

	panel := must(ferroui.NewStackPanel())
	defer panel.Release()
	child := must(ferroui.NewBorder())
	defer child.Release()
	ok(panel.SetSpacing(4))
	ok(panel.AddChild(child))
	ok(panel.RemoveChild(child))
}

func testAttachedProperties() {
	grid := must(ferroui.NewGrid())
	defer grid.Release()
	child := must(ferroui.NewBorder())
	defer child.Release()
	ok(grid.AddChild(child))
	ok(ferroui.GridSetRow(child, 2))
	ok(ferroui.GridSetColumn(child, 1))
	check(ferroui.GridRow(child) == 2, "attached row")
	check(ferroui.GridColumn(child) == 1, "attached column")
}

func testWrappers() {
	border := must(ferroui.NewBorder())
	defer border.Release()
	ok(border.SetWidth(10))
	control := ferroui.As[ferroui.Control](border)
	check(control.Handle() == border.Handle(), "the same handle")
	check(control.Width() == 10, "through another wrapper")
}

func testEvents() {
	button := must(ferroui.NewButton())
	defer button.Release()
	click := ferroui.EventID("Click")
	clicks := 0
	must(button.OnClick(func(sender ferroui.Button) {
		clicks++
		ok(sender.SetContent("Clicked"))
	}))
	ok(ferroui.Raise(button, click))
	check(clicks == 1, "the handler ran")
	check(button.Content() == "Clicked", "with its sender")

	counted := 0
	subscription := must(button.OnClick(func(ferroui.Button) { counted++ }))
	ok(ferroui.Raise(button, click))
	check(counted == 1, "a second handler")
	ok(subscription.Stop())
	ok(ferroui.Raise(button, click))
	check(counted == 1, "stopped")
	check(clicks == 3, "the first one still runs")

	// A panic in a handler stops at the boundary of the C ABI.
	must(button.OnClick(func(ferroui.Button) { panic("from the handler") }))
	ok(ferroui.Raise(button, click))
	check(clicks == 4, "after a panic")
}

func testErrors() {
	border := must(ferroui.NewBorder())
	defer border.Release()
	// `Text` is not a property of a border.
	err := ferroui.As[ferroui.TextBlock](border).SetText("no")
	var failure *ferroui.Error
	check(errors.As(err, &failure), "a typed error")
	check(strings.Contains(failure.Message, "Text"), "with the message of the library")
}

func main() {
	ok(ferroui.Init())
	ok(ferroui.CheckLibrary())
	testValues()
	testContentAndChildren()
	testAttachedProperties()
	testWrappers()
	testEvents()
	testErrors()
	fmt.Printf("ok: %d checks\n", checks)
}
