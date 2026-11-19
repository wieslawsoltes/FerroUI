# The C sample and the C tests of the ABI of FerroUI (docs/c-abi.md), on macOS and Linux:
#
#   make -C samples/CApi run       # the hello world window (the library with the desktop platform)
#   make -C samples/CApi run-gauge # a window with gauges, a control written in C (gauge.h)
#   make -C samples/CApi test      # the C tests of the ABI (no window, no display)
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
WARNINGS := -Wall -Wextra -Werror
LIBS := -L$(LIBDIR) -lferroui_capi -Wl,-rpath,$(LIBDIR) -lpthread

.PHONY: all library run run-gauge test bench clean

all: $(OUT)/hello_world $(OUT)/gauge $(OUT)/abi_test

library:
	cd $(ROOT) && $(CARGO) build --locked -p ferroui-capi $(CARGO_FLAGS)

$(OUT)/%: %.c gauge.h $(INCLUDE)/ferroui.h library
	mkdir -p $(OUT)
	$(CC) $(CFLAGS) $(WARNINGS) -I$(INCLUDE) $< -o $@ $(LIBS)

run: $(OUT)/hello_world
	$(OUT)/hello_world

run-gauge: $(OUT)/gauge
	$(OUT)/gauge

test: $(OUT)/abi_test
	$(OUT)/abi_test

bench: CFLAGS = -std=c11 -O2
bench: $(OUT)/call_overhead
	$(OUT)/call_overhead

clean:
	rm -rf $(OUT)
