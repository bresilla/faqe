SHELL := /bin/bash

PROJECT_NAME := faqe
PROJECT_VERSION := $(shell sed -n 's/^version[[:space:]]*=[[:space:]]*"\([^"]*\)".*/\1/p' Cargo.toml crates/faqe-cli/Cargo.toml | head -1)
CARGO := cargo
WEB_TARGET := wasm32-unknown-unknown
SERVE_BIND ?= 0.0.0.0:3000
PRESENTATION ?= content/talks/code/theme-showcase.md
WASM_BINDGEN_VERSION := $(shell sed -n 's/^wasm-bindgen[[:space:]]*=[[:space:]]*"=\([^"]*\)".*/\1/p' Cargo.toml)
WEB_PROFILE ?= debug
WEB_CARGO_DIR := $(CURDIR)/target/$(WEB_TARGET)/$(WEB_PROFILE)
EMBED_DIR := $(CURDIR)/target/faqe-embedded/$(WEB_PROFILE)
FAQE_EMBED_DIR := $(EMBED_DIR)
export FAQE_EMBED_DIR
TMPDIR := $(CURDIR)/target/tmp
export TMPDIR

ifeq ($(WEB_PROFILE),release)
WEB_CARGO_FLAGS := --release
else
WEB_CARGO_FLAGS :=
endif

$(info ------------------------------------------)
$(info Project: $(PROJECT_NAME) v$(PROJECT_VERSION))
$(info ------------------------------------------)

.PHONY: build b compile c web-bundle release-web-bundle release-build serve run r present check check-all content-check fmt fmt-check clippy rustdoc clean verify verify-all toolchain-check package-files package release help h

build: web-bundle
	@$(CARGO) build -p faqe-cli

b: build

compile: clean build

c: compile

web-bundle: toolchain-check
	@rm -rf "$(EMBED_DIR)"
	@mkdir -p "$(EMBED_DIR)" "$(TMPDIR)"
	@$(CARGO) build -p faqe-web --target $(WEB_TARGET) $(WEB_CARGO_FLAGS)
	@wasm-bindgen "$(WEB_CARGO_DIR)/faqe_web.wasm" --target web --out-dir "$(EMBED_DIR)" --out-name faqe_web
	@cp ./LICENSE "$(EMBED_DIR)/LICENSE"
	@cp ./THIRD_PARTY.md "$(EMBED_DIR)/THIRD_PARTY.md"
	@rm -f "$(EMBED_DIR)/YUBIKEY-GUIDE-MIT.txt"
	@rm -rf "$(EMBED_DIR)/licenses"
	@cp -R ./LICENSES "$(EMBED_DIR)/licenses"

release-web-bundle:
	@$(MAKE) web-bundle WEB_PROFILE=release
	@wasm-opt -Oz --enable-bulk-memory --enable-nontrapping-float-to-int "$(CURDIR)/target/faqe-embedded/release/faqe_web_bg.wasm" -o "$(CURDIR)/target/faqe-embedded/release/faqe_web_bg.wasm.opt"
	@mv "$(CURDIR)/target/faqe-embedded/release/faqe_web_bg.wasm.opt" "$(CURDIR)/target/faqe-embedded/release/faqe_web_bg.wasm"

release-build: release-web-bundle
	@FAQE_EMBED_DIR="$(CURDIR)/target/faqe-embedded/release" $(CARGO) build -p faqe-cli --release

serve: build
	@./target/debug/faqe serve ./content --bind $(SERVE_BIND)

run: serve

r: run

present:
	@presenterm "$(PRESENTATION)"

check: web-bundle
	@$(CARGO) check --workspace --all-targets

check-all: web-bundle
	@$(CARGO) check --workspace --all-targets --all-features

content-check: build
	@./target/debug/faqe check ./content

fmt:
	@$(CARGO) fmt --all

fmt-check:
	@$(CARGO) fmt --all -- --check

clippy: web-bundle
	@$(CARGO) clippy --workspace --all-targets --all-features -- -D warnings
	@$(CARGO) clippy -p faqe-web --target $(WEB_TARGET) -- -D warnings

rustdoc: web-bundle
	@RUSTDOCFLAGS="-Dwarnings" $(CARGO) doc --workspace --all-features --no-deps

toolchain-check:
	@target_libdir="$$(rustc --print target-libdir --target $(WEB_TARGET))"; \
		test -d "$$target_libdir" && find "$$target_libdir" -maxdepth 1 -name 'libcore-*.rlib' -print -quit | grep -q . || { \
			echo "Rust target $(WEB_TARGET) is unavailable; reload the Nix development shell" >&2; \
			exit 1; \
		}
	@test -n "$(WASM_BINDGEN_VERSION)" || { echo "cannot derive wasm-bindgen crate version" >&2; exit 1; }
	@test "$$(wasm-bindgen --version | awk '{print $$2}')" = "$(WASM_BINDGEN_VERSION)" || { \
		echo "wasm-bindgen CLI must match crate version $(WASM_BINDGEN_VERSION)" >&2; \
		exit 1; \
	}

verify: toolchain-check fmt-check check check-all content-check clippy rustdoc

verify-all: verify package

package-files: release-build
	@rm -rf ./target/package
	@mkdir -p ./target/package/LICENSES
	@cp ./target/release/faqe ./target/package/faqe
	@cp ./LICENSE ./target/package/LICENSE
	@cp ./THIRD_PARTY.md ./target/package/THIRD_PARTY.md
	@cp -R ./LICENSES/. ./target/package/LICENSES/

package: package-files

clean:
	@$(CARGO) clean

release:
	@if ! command -v git-rel >/dev/null 2>&1; then \
		echo "git-rel is not installed. Please install it first."; \
		exit 1; \
	fi
	@if [ -z "$(TYPE)" ]; then \
		echo "Release type not specified. Use make release TYPE=[patch|minor|major|M.m.p]"; \
		exit 1; \
	fi
	@git rel $(TYPE)

help:
	@echo
	@echo "Usage: make [target]"
	@echo
	@echo "Available targets:"
	@echo "  build          Build WASM assets and the native faqe binary"
	@echo "  release-build  Build an optimized one-binary release"
	@echo "  serve          Build and preview ./content"
	@echo "  present        Open PRESENTATION with Presenterm"
	@echo "  check          Check every workspace target"
	@echo "  check-all      Check all workspace targets and features"
	@echo "  content-check  Validate the website content submodule"
	@echo "  clippy         Lint native and WASM targets with warnings denied"
	@echo "  rustdoc        Build docs with warnings denied"
	@echo "  toolchain-check Verify Rust target and wasm-bindgen alignment"
	@echo "  verify         Run formatting, compile, lint, and documentation checks"
	@echo "  verify-all     Run verify and construct the release package"
	@echo "  package-files  Construct release package files"
	@echo "  package        Construct release package files"
	@echo "  clean          Remove build artifacts"
	@echo

h: help
