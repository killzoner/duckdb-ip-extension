LOCALPATH  ?= `pwd`
DOCKER     ?= docker compose run \
		$(DOCKERFLAGS) \
		--rm \
		$(shell [ -t 0 ] || echo "-T") \
		-e"USER=${USER}" \
		-e"WORKDIR=${WORKDIR}" \
		-e"LOCALPATH=${LOCALPATH}" \
		-e"DUCKDB_DOWNLOAD_LIB=${DUCKDB_DOWNLOAD_LIB}" \
		--volume="/etc/passwd:/etc/passwd" \
		--volume="/etc/shadow:/etc/shadow" \
		--volume="/etc/group:/etc/group" \
		--volume="/var/run/docker.sock:/var/run/docker.sock" \
		--volume="${HOME}:${HOME}" \
		--volume="$(shell pwd):$(WORKDIR)" \
		--workdir="$(WORKDIR)" \
		local
WORKDIR    ?= /workdir

LIB_LOCAL_DIR      ?= local_lib
LIB_DUCKDB_ARCH    ?= linux-amd64
LIB_DUCKDB_VERSION ?= 1.5.3

DUCKDB_DOWNLOAD_LIB ?= 1
export DUCKDB_DOWNLOAD_LIB

EXTENSION_NAME     ?= duckdb_ip_extension
EXTENSION_VERSION  ?= v0.1.0
DUCKDB_API_VERSION ?= v1.2.0
DUCKDB_PLATFORM    ?= linux_amd64
PROFILE            ?= release
OUTPUT_FILE        ?= $(EXTENSION_NAME).duckdb_extension

# Required for extension-ci-tools sub-makefiles
TARGET_DUCKDB_VERSION := $(DUCKDB_API_VERSION)

# Cargo writes the `dev` profile to target/debug/, not target/dev/.
ifeq ($(PROFILE),dev)
  CARGO_PROFILE_DIR := debug
else
  CARGO_PROFILE_DIR := $(PROFILE)
endif

ifneq ("$(wildcard /.dockerenv)","")
    DOCKER=
endif

# --- Include DuckDB Extension Makefiles for WASM + multi-arch release support ---
# These provide: configure, wasm_mvp, wasm_eh, wasm_threads, release, debug.
# Soft `-include`: a missing/uninitialized submodule only disables these targets
# rather than breaking unrelated ones (fmt, check-duckdb-pins).
-include extension-ci-tools/makefiles/c_api_extensions/base.Makefile
-include extension-ci-tools/makefiles/c_api_extensions/rust.Makefile

# --- Official DuckDB Template Glue Targets ---
.PHONY: configure
configure: venv platform extension_version

.PHONY: release
release: build_extension_library_release build_extension_with_metadata_release

.PHONY: debug
debug: build_extension_library_debug build_extension_with_metadata_debug

.PHONY: fmt
fmt: ## format files
	@$(DOCKER) sh -c '\
		cargo fmt ; \
		taplo fmt ; \
	'

.PHONY: lint
lint: ## format files
	@$(DOCKER) sh -c '\
		cargo clippy --all --all-targets --all-features -- -D warnings ; \
		cargo shear ; \
		cargo neat -m -p -f --package-workspace-meta-values rust-version,edition,license,homepage,repository,publish ; \
	'

.PHONY: shell
shell: ## start a shell
	@$(DOCKER)

.PHONY: install_lib_duckdb
install_lib_duckdb: ## install duckdb libs
	sh -c '\
		curl -LO -fsS https://github.com/duckdb/duckdb/releases/download/v$(LIB_DUCKDB_VERSION)/libduckdb-$(LIB_DUCKDB_ARCH).zip && \
		unzip -o libduckdb-$(LIB_DUCKDB_ARCH).zip -d $(LIB_LOCAL_DIR) && \
		rm -f libduckdb-$(LIB_DUCKDB_ARCH).zip ; \
	'

.PHONY: build-extension
build-extension: ## build .duckdb_extension (.so + metadata footer); override PROFILE=dev for faster builds, OUTPUT_FILE for per-arch naming
	cargo build --profile $(PROFILE)
	python3 extension-ci-tools/scripts/append_extension_metadata.py \
		-l target/$(CARGO_PROFILE_DIR)/lib$(EXTENSION_NAME).so \
		-o $(OUTPUT_FILE) \
		-n $(EXTENSION_NAME) \
		-dv $(DUCKDB_API_VERSION) \
		-ev $(EXTENSION_VERSION) \
		-p $(DUCKDB_PLATFORM)

.PHONY: update-ci-tools
update-ci-tools: sync-versions ## sync .gitmodules branch + pull latest on that branch
	git submodule update --init --remote extension-ci-tools

.PHONY: sync-versions
sync-versions: ## sync .gitmodules submodule branch to LIB_DUCKDB_VERSION
	git config -f .gitmodules submodule.extension-ci-tools.branch v$(LIB_DUCKDB_VERSION)

.PHONY: check-duckdb-pins
check-duckdb-pins: ## verify .gitmodules + Cargo.lock libduckdb-sys are pinned to LIB_DUCKDB_VERSION
	@cargo run --quiet -p check-duckdb-pins -- $(LIB_DUCKDB_VERSION)

.DEFAULT_GOAL := help
.PHONY: help
help:
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | sort | awk 'BEGIN {FS = ":.*?## "}; {printf "\033[36m%-30s\033[0m %s\n", $$1, $$2}'
