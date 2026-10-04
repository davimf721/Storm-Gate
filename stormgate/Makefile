# Storm Gate — top level build.
#
#   make bootstrap      check build prerequisites (macOS)
#   make runtime        build Wine, DXMT, DXVK, MoltenVK and package them (macOS)
#   make test           Rust tests, script lint, profile/database validation
#   make cli            release build of the `stormgate` CLI
#
# Build outputs go to build/ and dist/ (override with STORMGATE_BUILD_DIR /
# STORMGATE_DIST_DIR, e.g. to put multi-GB builds on an external SSD).

SHELL := /bin/bash
CARGO ?= cargo
SCRIPTS := scripts

.DEFAULT_GOAL := help

help: ## Show this help
	@awk 'BEGIN { FS = ":.*## " } /^[a-z0-9-]+:.*## / { printf "  \033[36m%-16s\033[0m %s\n", $$1, $$2 }' $(MAKEFILE_LIST)

bootstrap: ## Check build prerequisites
	$(SCRIPTS)/bootstrap.sh

wine: ## Build Proton Wine for macOS (x86_64, new WoW64)
	$(SCRIPTS)/build-wine.sh

dxmt: wine ## Build DXMT (D3D10/11 -> Metal)
	$(SCRIPTS)/build-dxmt.sh

dxvk: ## Build DXVK (D3D9/10/11 -> Vulkan)
	$(SCRIPTS)/build-dxvk.sh

vkd3d: ## Build VKD3D-Proton (D3D12 -> Vulkan, experimental)
	$(SCRIPTS)/build-vkd3d.sh

moltenvk: ## Build MoltenVK (Vulkan -> Metal)
	$(SCRIPTS)/build-moltenvk.sh

package: ## Package dist/* into a runtime archive with SHA256SUMS and SBOM
	$(SCRIPTS)/package.sh

runtime: wine dxmt dxvk moltenvk package ## Build the full default runtime

runtime-experimental: wine dxmt dxvk moltenvk vkd3d package ## Runtime including VKD3D-Proton

cli: ## Build the stormgate CLI (release)
	$(CARGO) build --release --locked -p stormgate-cli
	@echo "binary: target/release/stormgate"

install-runtime: cli ## Install the packaged runtime and make it the default
	@archive=$$(ls -t dist/stormgate-runtime-*.tar.gz | head -1); \
	sum=$$(shasum -a 256 "$$archive" | awk '{print $$1}'); \
	target/release/stormgate runtime install "$$archive" --sha256 "$$sum" --use

windows-tests: ## Build synthetic Windows test programs (needs mingw-w64)
	$(MAKE) -C tests/windows

test: ## Run all checks that do not need a Mac
	$(CARGO) test --workspace --locked
	@if command -v shellcheck >/dev/null; then shellcheck $(SCRIPTS)/*.sh $(SCRIPTS)/lib/*.sh; \
	 else echo "shellcheck not installed; skipping script lint"; fi
	$(SCRIPTS)/check-patches.sh
	$(CARGO) run -q -p stormgate-cli -- profile validate --shared profiles/template.toml
	$(CARGO) run -q -p stormgate-cli -- profile check-db compat

fmt: ## Format Rust code
	$(CARGO) fmt --all

lint: ## Formatting check and clippy
	$(CARGO) fmt --all -- --check
	$(CARGO) clippy --workspace --all-targets --locked -- -D warnings

check: lint test ## Everything CI runs

clean: ## Remove build outputs (keeps fetched sources)
	rm -rf dist build/package build/*-build*
	$(MAKE) -C tests/windows clean

.PHONY: help bootstrap wine dxmt dxvk vkd3d moltenvk package runtime runtime-experimental \
	cli install-runtime windows-tests test fmt lint check clean
