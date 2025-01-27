# # ----------------Commands----------------
#
# # change the 20 value in printf to adjust width
# # Use ' ## some comment' behind a command and it will be added to the help message automatically

help: ## Show this help message
	@awk 'BEGIN {FS = ":.*?## "}; /^[a-zA-Z0-9_-]+:.*?## / {printf "\033[36m%-20s\033[0m %s\n", $$1, $$2}' $(MAKEFILE_LIST) | grep -v '^help:.*?## '

format-check: ## cargo fmt --check
	cargo fmt --all -- --check

format: ## cargo fmt
	cargo fmt

clippy: ## cargo clippy 
	cargo clippy -- -D warnings

check: ## cargo check 
	cargo check

# Ensure cargo-nextest is installed
setup_nextest:
	@which cargo-nextest >/dev/null || cargo install cargo-nextest

test: setup_nextest ## Run cargo nextest
	cargo nextest run --jobs 16

all: ## Run all steps in parallel: format, check, test, clippy
	$(MAKE) format &
	$(MAKE) check &
	$(MAKE) test &
	$(MAKE) clippy &
	wait

ci: ## Run only essential CI pipeline (check, test, clippy)
	$(MAKE) check &
	$(MAKE) test &
	$(MAKE) clippy &
	wait

# --------------Configuration-------------
#
.EXPORT_ALL_VARIABLES: # send all vars to shell
.NOTPARALLEL:          # wait for this target to finish

# Enable incremental builds
export CARGO_INCREMENTAL=1

.PHONY: docs all ci # All targets are accessible for user
.DEFAULT: help # Running Make will run the help target 

MAKEFLAGS += --no-print-directory # don't add message about entering and leaving the working directory
