# The C sample and the C tests of the ABI of FerroUI (docs/c-abi.md), on macOS and Linux:
#
#   make -C samples/CApi run       # the hello world window (the library with the desktop platform)
#   make -C samples/CApi test      # the C tests of the ABI (no window, no display)
#
# The library is built with cargo first. CARGO_FLAGS go to that build: `--no-default-features`
# builds the library without the desktop platform (no renderer), which is all the tests need.
# CC, CFLAGS and CARGO_TARGET_DIR are taken from the environment.

CARGO ?= cargo
CARGO_FLAGS ?=
ROOT := $(abspath ../..)
TARGET_DIR := $(if $(CARGO_TARGET_DIR),$(CARGO_TARGET_DIR),$(ROOT)/target)
LIBDIR := $(TARGET_DIR)/debug
OUT := $(TARGET_DIR)/capi
INCLUDE := $(ROOT)/src/FerroUI.CApi/include
CFLAGS ?= -std=c11 -O1 -g
WARNINGS := -Wall -Wextra -Werror
LIBS := -L$(LIBDIR) -lferroui_capi -Wl,-rpath,$(LIBDIR) -lpthread

.PHONY: all library run test clean

all: $(OUT)/hello_world $(OUT)/abi_test

library:
	cd $(ROOT) && $(CARGO) build --locked -p ferroui-capi $(CARGO_FLAGS)

$(OUT)/%: %.c $(INCLUDE)/ferroui.h library
	mkdir -p $(OUT)
	$(CC) $(CFLAGS) $(WARNINGS) -I$(INCLUDE) $< -o $@ $(LIBS)

run: $(OUT)/hello_world
	$(OUT)/hello_world

test: $(OUT)/abi_test
	$(OUT)/abi_test

clean:
	rm -rf $(OUT)
