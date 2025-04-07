.PHONY: all
all: clean build docs-no-open check run

.PHONY: clean
clean:
	@cargo clean
	@rm -f test_buddy_c test_buddy_cpp

build:
	@cargo build --release
	@cbindgen --config cbindgen.toml --crate buddy_memory_manager --output src/buddy_memory_manager.h --lang c
	@cbindgen --config cbindgen.toml --crate buddy_memory_manager --output src/buddy_memory_manager.hpp --lang c++
	@gcc src/tests/tests.c -L./target/release -lbuddy_memory_manager -o test_buddy_c
	@g++ src/tests/tests.cpp -L./target/release -lbuddy_memory_manager -o test_buddy_cpp

check:
	@cargo test --no-fail-fast --release
	@LD_LIBRARY_PATH=./target/release ./test_buddy_c
	@LD_LIBRARY_PATH=./target/release ./test_buddy_cpp

run:
	@cargo -q run --release

docs:
	@cargo -q doc --open

docs-no-open:
	@cargo -q doc

.PHONY: install-deps
install-deps:
	sudo apt-get update -y
	sudo apt-get install -y libio-socket-ssl-perl libmime-tools-perl
