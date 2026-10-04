# Builds the synthetic Windows test programs with mingw-w64.
#   make -C tests/windows          -> tests/windows/build/*.exe
#   stormgate test hello           -> runs build/hello.exe in a test prefix
#
# Every program prints "STORMGATE-TEST-PASS <name>" and exits 0 on success.

CC64 ?= x86_64-w64-mingw32-gcc
CFLAGS ?= -O2 -Wall -Wextra -Werror -Wno-unused-parameter -Wno-missing-field-initializers
BUILD := build

TESTS := hello win32-window d3d11-triangle
EXES := $(TESTS:%=$(BUILD)/%.exe)

LIBS_hello :=
LIBS_win32-window := -luser32 -lgdi32
LIBS_d3d11-triangle := -ld3d11 -ldxguid

all: $(EXES)

define test_rule
$(BUILD)/$(1).exe: $(1)/$(1).c | $(BUILD)
	$$(CC64) $$(CFLAGS) -o $$@ $$< $$(LIBS_$(1))
endef
$(foreach t,$(TESTS),$(eval $(call test_rule,$(t))))

$(BUILD):
	mkdir -p $@

clean:
	rm -rf $(BUILD)

.PHONY: all clean
