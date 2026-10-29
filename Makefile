APP_DIR ?= $(HOME)/Applications
APP := $(APP_DIR)/Quick X.app

.PHONY: install uninstall run build test lint icon

## Build a release and install it as "Quick X.app" (APP_DIR=... to change where).
install:
	scripts/bundle-macos.sh "$(APP_DIR)"

uninstall:
	rm -rf "$(APP)"

run:
	cargo run --release

build:
	cargo build --release

test:
	cargo test

lint:
	cargo clippy --all-targets -- -D warnings

## Regenerate assets/AppIcon.icns from assets/icon.svg (needs rsvg-convert).
icon:
	scripts/make-icon.sh
