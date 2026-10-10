CC ?= cc
CFLAGS ?= -std=c11 -O2 -g -Wall -Wextra -Wpedantic -Werror
CPPFLAGS ?= -Iinclude
LDLIBS ?= -lcrypto -lm
SOURCES := src/weave.c
.PHONY: all test research sanitizer clean
all: build/weave-research build/weave-tests
build:
	mkdir -p build
build/weave-research: $(SOURCES) src/research.c include/weave.h | build
	$(CC) $(CPPFLAGS) $(CFLAGS) $(SOURCES) src/research.c -o $@ $(LDLIBS)
build/weave-tests: $(SOURCES) tests/test_weave.c include/weave.h | build
	$(CC) $(CPPFLAGS) $(CFLAGS) $(SOURCES) tests/test_weave.c -o $@ $(LDLIBS)
test: all
	./build/weave-tests
research: all
	./build/weave-research --width 3 --waves 4 --bits 8
	./build/weave-research --width 5 --waves 4 --bits 8
	./build/weave-research --attack --trials 20000 --rounds 100 --seed 42
sanitizer:
	$(MAKE) clean
	$(MAKE) CFLAGS='-std=c11 -O1 -g -Wall -Wextra -Wpedantic -Werror -fsanitize=address,undefined -fno-omit-frame-pointer' test
clean:
	rm -rf build
