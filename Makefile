.DEFAULT_GOAL := help

NPM ?= npm
CARGO ?= cargo
DESKTOP_MANIFEST := desktop/Cargo.toml
LINUX_TARGET ?= x86_64-unknown-linux-gnu
WINDOWS_TARGET ?= x86_64-pc-windows-gnu
MINGW_BIN ?= $$HOME/.local/share/gamepad-navigator/toolchains/mingw/usr/bin
DESKTOP_DIST_DIR := desktop/dist
WINDOWS_BINARY := desktop/target/$(WINDOWS_TARGET)/release/gamepad-desktop.exe
WINDOWS_RELEASE_BINARY := $(DESKTOP_DIST_DIR)/GamepadDesktop-Windows-x64.exe
LINUX_BINARY := desktop/target/$(LINUX_TARGET)/release/gamepad-desktop
LINUX_PACKAGE := $(DESKTOP_DIST_DIR)/GamepadDesktop-Linux-x64.tar.gz

.PHONY: help dev run build check lint format format-check test brand-assets \
	web-install web-dev web-build web-package web-typecheck web-lint web-format web-format-check web-test \
	desktop-run desktop-build desktop-clean-dist desktop-release desktop-linux desktop-linux-package desktop-windows desktop-packages \
	desktop-check desktop-lint desktop-format desktop-format-check desktop-test desktop-preview

help:
	@printf '%s\n' \
		'Gamepad Desktop and browser extension' \
		'' \
		'Main: make dev | run | build | check | lint | format | format-check | test' \
		'  brand-assets   Generate every brand icon from assets/gamepad-desktop.svg' \
		'  dev            Run the browser extension in WXT development mode' \
		'  run            Run the Rust desktop app' \
		'  build          Build the extension and the desktop app for this host' \
		'  check          Type-check the extension and check the Rust app' \
		'  lint           Lint the extension and run Clippy' \
		'  format         Format TypeScript and Rust files' \
		'  format-check   Check formatting without changing files' \
		'  test           Run browser E2E and Rust tests' \
		'' \
		'Extension: web-install, web-dev, web-build, web-package, web-typecheck, web-lint, web-format, web-format-check, web-test' \
		'Desktop:   desktop-run, desktop-build, desktop-clean-dist, desktop-release, desktop-linux, desktop-linux-package, desktop-windows, desktop-packages, desktop-check, desktop-lint, desktop-format, desktop-format-check, desktop-test, desktop-preview' \
		'' \
		'desktop-build and desktop-packages clean desktop/dist and build a Linux x64 archive and Windows x64 executable; install the Windows build tools first.' \
		'desktop-windows builds the Windows x64 executable; install the Rust target, MinGW, and CMake first.'

dev: web-dev

brand-assets:
	$(NPM) --prefix extension run brand:generate

run: desktop-run

build: web-build desktop-release

check: web-typecheck desktop-check

lint: web-lint desktop-lint

format: web-format desktop-format

format-check: web-format-check desktop-format-check

test: web-test desktop-test

web-install:
	$(NPM) --prefix extension install

web-dev:
	$(NPM) --prefix extension run dev

web-build:
	$(NPM) --prefix extension run build

web-package: web-build
	$(NPM) --prefix extension run zip

web-typecheck:
	$(NPM) --prefix extension run typecheck

web-lint:
	$(NPM) --prefix extension run lint

web-format:
	$(NPM) --prefix extension run format

web-format-check:
	$(NPM) --prefix extension run format:check

web-test:
	$(NPM) --prefix extension run test:e2e

desktop-run:
	$(CARGO) run --manifest-path $(DESKTOP_MANIFEST)

desktop-build: desktop-packages

desktop-release:
	$(CARGO) build --manifest-path $(DESKTOP_MANIFEST) --release

desktop-linux:
	$(CARGO) build --manifest-path $(DESKTOP_MANIFEST) --release --target $(LINUX_TARGET)

desktop-linux-package: desktop-clean-dist desktop-linux
	mkdir -p $(DESKTOP_DIST_DIR)
	tar -czf $(LINUX_PACKAGE) -C $(dir $(LINUX_BINARY)) $(notdir $(LINUX_BINARY))

desktop-windows:
	PATH="$(if $(MINGW_BIN),$(MINGW_BIN):)$$PATH" $(CARGO) build --manifest-path $(DESKTOP_MANIFEST) --release --target $(WINDOWS_TARGET)
	mkdir -p $(DESKTOP_DIST_DIR)
	cp $(WINDOWS_BINARY) $(WINDOWS_RELEASE_BINARY)
	rm -f $(DESKTOP_DIST_DIR)/GamepadDesktop-Windows-x64.zip

desktop-clean-dist:
	mkdir -p $(DESKTOP_DIST_DIR)
	find $(DESKTOP_DIST_DIR) -mindepth 1 -maxdepth 1 \
		! -name .stfolder \
		! -name $(notdir $(WINDOWS_RELEASE_BINARY)) \
		! -name $(notdir $(LINUX_PACKAGE)) \
		-exec rm -rf -- {} +

desktop-packages: desktop-clean-dist
	$(MAKE) desktop-linux-package
	$(MAKE) desktop-windows

desktop-check:
	$(CARGO) check --manifest-path $(DESKTOP_MANIFEST)

desktop-lint:
	$(CARGO) clippy --manifest-path $(DESKTOP_MANIFEST) --all-targets -- -D warnings

desktop-format:
	$(CARGO) fmt --manifest-path $(DESKTOP_MANIFEST)

desktop-format-check:
	$(CARGO) fmt --manifest-path $(DESKTOP_MANIFEST) -- --check

desktop-test:
	$(CARGO) test --manifest-path $(DESKTOP_MANIFEST)

desktop-preview:
	$(CARGO) run --manifest-path $(DESKTOP_MANIFEST) --example keyboard_preview
