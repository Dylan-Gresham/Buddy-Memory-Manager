all: clean build docs-no-open check run

.PHONY: clean
clean:
	@cargo clean

build:
	@cargo build --release

check:
	@cargo test --no-fail-fast --release

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