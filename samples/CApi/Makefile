# The C sample and the C tests of the ABI of FerroUI (docs/c-abi.md), on macOS and Linux:
#
#   make -C samples/CApi run       # the hello world window (the library with the desktop platform)
#   make -C samples/CApi run-gauge # a window with gauges, a control written in C (gauge.h)
#   make -C samples/CApi test      # the C tests of the ABI (no window, no display)
#   make -C samples/CApi run-cpp   # the hello world window in C++, over the generated bindings
#   make -C samples/CApi test-cpp  # the tests of the C++ bindings (no window, no display)
#   make -C samples/CApi bench-cpp # the cost of the C++ bindings against the C calls
#   make -C samples/CApi run-go    # the hello world window in Go (needs a Go toolchain)
#   make -C samples/CApi test-go   # the tests of the Go bindings (no window, no display)
#   make -C samples/CApi run-dart  # the hello world window in Dart (needs a Dart SDK; not on macOS)
#   make -C samples/CApi test-dart # the tests of the Dart bindings (no window, no display)
#   make -C samples/CApi run-csharp  # the hello world window in C# (needs a .NET SDK)
#   make -C samples/CApi test-csharp # the tests of the C# bindings (no window, no display)
#   make -C samples/CApi bench CARGO_FLAGS="--release --no-default-features --features type-tables" PROFILE=release
#                                  # the cost of the calls from C (call_overhead.c)
#
# The library is built with cargo first. CARGO_FLAGS go to that build: `--no-default-features
# --features type-tables` builds the library without the desktop platform (no renderer) and with
# the tables of types the tests and the benchmark look classes up in; `--no-default-features
# --features desktop` builds it by use (docs/c-abi.md, section 16), for the samples, which
# register their classes.
# CC, CFLAGS, CARGO_TARGET_DIR and PROFILE (the cargo profile directory, `debug` by default) are
# taken from the environment.

CARGO ?= cargo
CARGO_FLAGS ?=
ROOT := $(abspath ../..)
TARGET_DIR := $(if $(CARGO_TARGET_DIR),$(CARGO_TARGET_DIR),$(ROOT)/target)
PROFILE ?= debug
LIBDIR := $(TARGET_DIR)/$(PROFILE)
OUT := $(TARGET_DIR)/capi
INCLUDE := $(ROOT)/src/FerroUI.CApi/include
CFLAGS ?= -std=c11 -O1 -g
CXXFLAGS ?= -std=c++17 -O1 -g
GO ?= go
DART ?= dart
DOTNET ?= dotnet
LIBRARY_FILE := $(LIBDIR)/$(if $(filter Darwin,$(shell uname -s)),libferroui_capi.dylib,libferroui_capi.so)
GO_ENV = CGO_ENABLED=1 CGO_CFLAGS="-I$(INCLUDE)" CGO_LDFLAGS="-L$(LIBDIR) -lferroui_capi -Wl,-rpath,$(LIBDIR) -lpthread"
WARNINGS := -Wall -Wextra -Werror
LIBS := -L$(LIBDIR) -lferroui_capi -Wl,-rpath,$(LIBDIR) -lpthread

.PHONY: all library run run-gauge run-cpp run-go run-dart run-csharp test test-cpp test-go test-dart test-csharp bench bench-cpp clean

all: $(OUT)/hello_world $(OUT)/gauge $(OUT)/abi_test

library:
	cd $(ROOT) && $(CARGO) build --locked -p ferroui-capi $(CARGO_FLAGS)

$(OUT)/%: %.c gauge.h $(INCLUDE)/ferroui.h library
	mkdir -p $(OUT)
	$(CC) $(CFLAGS) $(WARNINGS) -I$(INCLUDE) $< -o $@ $(LIBS)

$(OUT)/hello_world_cpp: cpp/hello_world/main.cpp cpp/hello_world/ferroui/ferroui.hpp $(INCLUDE)/ferroui.h library
	mkdir -p $(OUT)
	$(CXX) $(CXXFLAGS) $(WARNINGS) -I$(INCLUDE) $< -o $@ $(LIBS)

$(OUT)/binding_test_cpp: cpp/binding_test/main.cpp $(ROOT)/bindings/cpp/full/ferroui.hpp $(INCLUDE)/ferroui.h library
	mkdir -p $(OUT)
	$(CXX) $(CXXFLAGS) $(WARNINGS) -I$(ROOT)/bindings/cpp/full -I$(INCLUDE) $< -o $@ $(LIBS)

$(OUT)/call_overhead_cpp: cpp/call_overhead/main.cpp $(ROOT)/bindings/cpp/full/ferroui.hpp $(INCLUDE)/ferroui.h library
	mkdir -p $(OUT)
	$(CXX) $(CXXFLAGS) $(WARNINGS) -I$(ROOT)/bindings/cpp/full -I$(INCLUDE) $< -o $@ $(LIBS)

run: $(OUT)/hello_world
	$(OUT)/hello_world

run-gauge: $(OUT)/gauge
	$(OUT)/gauge

run-cpp: $(OUT)/hello_world_cpp
	$(OUT)/hello_world_cpp

test: $(OUT)/abi_test
	$(OUT)/abi_test

test-cpp: $(OUT)/binding_test_cpp
	$(OUT)/binding_test_cpp

bench: CFLAGS = -std=c11 -O2
bench: $(OUT)/call_overhead
	$(OUT)/call_overhead

run-go: library
	cd go/hello_world && $(GO_ENV) $(GO) run .

test-go: library
	cd go/binding_test && $(GO_ENV) $(GO) run .

run-dart: library
	cd dart/hello_world && $(DART) pub get && FERROUI_LIBRARY=$(LIBRARY_FILE) $(DART) run main.dart

test-dart: library
	cd dart/binding_test && $(DART) pub get && FERROUI_LIBRARY=$(LIBRARY_FILE) $(DART) run main.dart

run-csharp: library
	cd csharp/hello_world && FERROUI_LIBRARY=$(LIBRARY_FILE) $(DOTNET) run

test-csharp: library
	cd csharp/binding_test && FERROUI_LIBRARY=$(LIBRARY_FILE) $(DOTNET) run

bench-cpp: CXXFLAGS = -std=c++17 -O2
bench-cpp: $(OUT)/call_overhead_cpp
	$(OUT)/call_overhead_cpp

clean:
	rm -rf $(OUT)
