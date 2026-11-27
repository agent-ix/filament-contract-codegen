# =============================================================================
# Filament Contract Codegen Makefile
# =============================================================================
#
# Native orchestration: every target calls the toolchain that owns the job.
# =============================================================================

override CARGO := cargo
override MSRV := $(shell sed -n 's/^rust-version = "\([^"]*\)"/\1/p' Cargo.toml)
QUIRE ?= quire
# --locked only when no local patch is active: a patch rewrites the resolution.
LOCKED ?= $(if $(wildcard .cargo/config.toml),,--locked)


.PHONY: help
help:
	@echo "Available targets:"
	@echo "  make figures          - Regenerate docs/images from docs/images/figures.json (ixd build)"
	@echo "  make figures-check    - Fail when docs/images are stale; writes nothing"
	@echo "  make live-solver-test - Opt-in: real-cvc5 refinement test (needs FILAMENT_CVC5_BIN)"
	@echo "  make fmt              - Format with rustfmt"
	@echo "  make fmt-check        - Verify formatting (CI gate)"
	@echo "  make lint             - Clippy with -D warnings"
	@echo "  make test             - cargo test"
	@echo "  make fspec-preflight  - Verify the explicit FSpec fixture root"
	@echo "  make build            - Release build"
	@echo "  make kani             - Run the Kani obligation lane serially"
	@echo "  make kani-scope       - Show whether changed paths require the real-Kani lane"
	@echo "  make kani-gate        - Run and verify the full real-Kani lane"
	@echo "  make kani-mutations   - Require every declared mutant to fail its owning proof (standalone merge gate)"
	@echo "  make tools            - Install the host tools the tests drive (llvm-tools, cargo-llvm-cov, Kani)"
	@echo "  make msrv             - Check the crate with Rust $(MSRV)"
	@echo "  make spec             - Quire-validate the specification"
	@echo "  make clean            - cargo clean"
	@echo "  make deny             - Run all configured cargo-deny checks"
	@echo "  make rustdoc          - Build warning-free API documentation"
	@echo "  make use-local        - Patch first-party git deps to sibling checkouts (.cargo/config.toml)"
	@echo "  make use-remote       - Remove the local patch file and restore Cargo.lock; build from GitHub"
	@echo "  make ci               - All local CI gates"

# =============================================================================
# Format / Lint / Test
# =============================================================================

.PHONY: fmt
fmt:
	$(CARGO) fmt --all

.PHONY: fmt-check
fmt-check:
	$(CARGO) fmt --all -- --check

.PHONY: lint
lint:
	$(CARGO) clippy $(LOCKED) --all-targets -- -D warnings

export FSPEC_DIR
FSPEC_FIXTURES := positive-union-nodes positive-clause-operations positive-all-families
GATES_BIN := target/cg-gates
GATES_CARGO_TARGET_DIR := $(if $(strip $(CARGO_TARGET_DIR)),$(CARGO_TARGET_DIR),target)

$(GATES_BIN): scripts/cg_gates.rs Cargo.toml Cargo.lock Makefile
	@mkdir -p target
	$(CARGO) build $(LOCKED) --bin cg_gates --target-dir "$(GATES_CARGO_TARGET_DIR)"
	install -m 755 "$(GATES_CARGO_TARGET_DIR)/debug/cg_gates" $@

.PHONY: fspec-preflight
fspec-preflight:
	@if [ -z "$$FSPEC_DIR" ]; then \
	  echo 'FSPEC_DIR is required: pass the filament-specification checkout root explicitly' >&2; exit 2; \
	fi; \
	if [ ! -d "$$FSPEC_DIR" ]; then \
	  echo "FSPEC_DIR is not a directory: $$FSPEC_DIR" >&2; exit 2; \
	fi; \
	for name in $(FSPEC_FIXTURES); do \
	  fixture="$$FSPEC_DIR/proposals/checked-package-v2/fixtures/$$name.json"; \
	  if [ ! -f "$$fixture" ]; then \
	    echo "FSPEC_DIR is missing required fixture: $$fixture" >&2; exit 2; \
	  fi; \
	done

.PHONY: fspec-preflight-selftest
fspec-preflight-selftest: $(GATES_BIN)
	$(GATES_BIN) fspec-selftest

.PHONY: test
test: fspec-preflight fspec-preflight-selftest
	$(CARGO) test $(LOCKED)

# The Kani lane (FR-015, TC-025). Kani and CBMC are memory-heavy, so the lane
# holds a host-wide lock, runs one harness at a time, and builds in its own
# target directory.
#
# Integration tests live in the single `it` binary (`tests/it/main.rs`); the
# `kani_obligations`, `skeleton_spine`,
# `bounded_kani_corpus`, `kani_generation` and `kani_batching` filters select those modules'
# tests, and `--ignored` runs only their `#[ignore]`d real-prover tests: the
# real-Kani obligation runs, the bounded corpus cases, the
# generation adapter proofs and the harness batching runs (the launcher process
# count before and after, and a real mixed batch) run through `cargo kani`. The filters
# follow `--` because libtest accepts several; cargo's own positional filter
# takes one.
# The test suite drives real tools: the native-coverage tests run `cargo
# llvm-cov` and read llvm-cov/llvm-profdata from that toolchain's sysroot, and the Kani
# tests run `cargo kani`. Install them once per machine.
.PHONY: tools
tools:
	rustup component add llvm-tools-preview
	$(CARGO) install --locked cargo-llvm-cov
	$(CARGO) install --locked kani-verifier
	$(CARGO) kani setup

KANI_LOCK ?= /tmp/agent-e-heavy-build.lock
# Keep each ignored installed-Kani test module in this lane; kani-scope selects the gate by path.
# The three integer_division_harness general-route reproducers (about 600 s and 8 GiB each, all
# expected TimedOut) are deliberately outside this gate. Its native-route tests are inside it: seven
# constant-divisor Verified runs (17 to 23 s each) and the mutant test (healthy run plus four
# Falsified mutants, 331 s and 7.2 GB scope peak), about 8 minutes together. Run the reproducers by
# name with --ignored (see
# docs/qualification/ir854-general-route-division-measurements.md).
KANI_FILTERS := kani_obligations kani_obligations_symbolic_domains skeleton_spine kani_stdout_capture bounded_kani_corpus kani_generation kani_batching enum_scalar_harness float_scalar_harness shadow_kani native_constant_divisor_8_bit native_div_floor_mutants
KANI_TEST := $(CARGO) test $(LOCKED) -j 4 --test it --target-dir target-codex-backends -- --ignored --test-threads=1 $(KANI_FILTERS)

# Opt-in live-solver lane (TC-059, FR-045-AC-4/5): real cvc5 proofs through the refinement
# adapter. The test is #[ignore]d because it needs a cvc5 binary; run it with
#   make live-solver-test FILAMENT_CVC5_BIN=/path/to/cvc5
# It is not part of `make test` or `make ci`.
.PHONY: live-solver-test
live-solver-test:
	@test -n "$(FILAMENT_CVC5_BIN)" || { echo 'FILAMENT_CVC5_BIN must name a cvc5 binary' >&2; exit 2; }
	FILAMENT_CVC5_BIN="$(FILAMENT_CVC5_BIN)" $(CARGO) test $(LOCKED) --lib -- --ignored --test-threads=1 production_refinement_live_solver_handoff_reaches_fsl_checker

.PHONY: kani kani-scope kani-gate
kani:
	flock $(KANI_LOCK) $(KANI_TEST)

kani-scope: $(GATES_BIN)
	$(GATES_BIN) kani-scope

kani-gate: $(GATES_BIN)
	$(GATES_BIN) kani-gate $(KANI_LOCK) -- $(KANI_TEST)

# The mutant gate (FR-055): standalone, never part of `make ci`, a Kani-affecting merge gate
# alongside `make kani-gate`. Each declared mutant (scripts/kani-mutants.json) is injected into a
# scratch copy and its owning proof must fail with `VERIFICATION:- FAILED`. Serial, under the host lock.
# The per-mutant timeout and the total budget (seconds) are this target's configuration. SIGINT, SIGTERM
# and SIGHUP kill the running proof's process group and remove the scratch copy and its build cache.
KANI_MUTANTS ?= scripts/kani-mutants.json
KANI_MUTANT_TIMEOUT ?= 3600
KANI_MUTATIONS_BUDGET ?= 21600
KANI_MUTANT_TEST := $(CARGO) test $(LOCKED) -j 4 --test it -- --ignored --test-threads=1 --nocapture --exact

.PHONY: kani-mutations
kani-mutations: $(GATES_BIN)
	flock $(KANI_LOCK) $(GATES_BIN) kani-mutations $(KANI_MUTANTS) $(KANI_MUTANT_TIMEOUT) $(KANI_MUTATIONS_BUDGET) -- $(KANI_MUTANT_TEST)

.PHONY: build
build:
	$(CARGO) build $(LOCKED) --release

.PHONY: msrv
msrv:
	rustup toolchain install $(MSRV) --profile minimal
	rustup run $(MSRV) cargo check $(LOCKED) --all-targets

.PHONY: spec
spec:
	$(QUIRE) validate --scope . 'spec/**/*.md' 'plan/**/*.md' 'reviews/**/*.md'

.PHONY: clean
clean:
	$(CARGO) clean

# =============================================================================
# Supply chain & safety
# =============================================================================

.PHONY: deny
deny:
	$(CARGO) deny check

.PHONY: cargo-audit
cargo-audit:
	$(CARGO) audit

.PHONY: rustdoc
rustdoc:
	RUSTDOCFLAGS=-Dwarnings $(CARGO) doc $(LOCKED) --no-deps --document-private-items

# =============================================================================
# Local development against sibling checkouts
#
# `use-local` writes a gitignored .cargo/config.toml that patches each
# first-party git dependency to its working tree at $(SIBLINGS)/<repo>, uncommitted
# edits included. `use-local` first snapshots Cargo.lock to the gitignored
# .cargo/Cargo.lock.pre-local; `use-remote` deletes the config and restores the
# lock from that snapshot (and does nothing to the lock if there is none). So the
# lock returns to its state before the first `use-local`; lock changes made while
# a patch is active are discarded, and a `cargo update -p` made without a patch is kept.
# Format: <repo>:<crate>:<crate-dir>; entries are grouped by repo here, in any
# order, so each repo gets exactly one [patch] table.
# SIBLINGS is the directory holding the sibling clones: the parent of the main
# checkout, so it is also right from a linked worktree. Override to relocate.
# =============================================================================

SIBLINGS ?= $(abspath $(shell git rev-parse --path-format=absolute --git-common-dir)/../..)
LOCAL_PATCHES ?= filament-contract-ir:filament-contract-ir:. filament-contract-ir:filament-contract-model:crates/filament-contract-model \
	filament-contract-runtime:filament-contract-runtime:. \
	filament-verification-contracts:filament-verification-contracts:. \
	filament-spec-language:fsl-cst:fsl-cst filament-spec-language:fsl-eval:fsl-eval \
	filament-spec-language:fsl-forms:fsl-forms filament-spec-language:fsl-foundation:fsl-foundation \
	filament-spec-language:fsl-package:fsl-package filament-spec-language:fsl-replay:fsl-replay \
	filament-spec-language:fsl-semantics:fsl-semantics \
	filament-exact:filament-exact:. filament-semantic-value:filament-semantic-value:.

.PHONY: use-local
use-local:
	@set -e; mkdir -p .cargo; \
	for spec in $(LOCAL_PATCHES); do \
	  if [ "$$(printf '%s' "$$spec" | tr -cd ':' | wc -c)" != 2 ] || printf '%s' "$$spec" | grep -q '::\|^:\|:$$'; then \
	    echo "use-local: malformed LOCAL_PATCHES entry '$$spec' (want repo:crate:dir)" >&2; exit 1; \
	  fi; \
	  repo=$${spec%%:*}; rest=$${spec#*:}; dir=$${rest#*:}; \
	  if [ ! -f "$(SIBLINGS)/$$repo/$$dir/Cargo.toml" ]; then \
	    echo "use-local: $(SIBLINGS)/$$repo is not cloned (no Cargo.toml at $(SIBLINGS)/$$repo/$$dir); clone agent-ix/$$repo next to this repo" >&2; exit 1; \
	  fi; \
	done; \
	[ -f .cargo/Cargo.lock.pre-local ] || cp Cargo.lock .cargo/Cargo.lock.pre-local; \
	: > .cargo/config.toml; \
	repos=$$(for spec in $(LOCAL_PATCHES); do printf '%s\n' "$${spec%%:*}"; done | awk '!seen[$$0]++'); \
	first=1; \
	for repo in $$repos; do \
	  [ "$$first" = 1 ] || printf '\n' >> .cargo/config.toml; first=0; \
	  printf '[patch."https://github.com/agent-ix/%s"]\n' "$$repo" >> .cargo/config.toml; \
	  for spec in $(LOCAL_PATCHES); do \
	    [ "$${spec%%:*}" = "$$repo" ] || continue; \
	    rest=$${spec#*:}; crate=$${rest%%:*}; dir=$${rest#*:}; \
	    printf '%s = { path = "%s/%s/%s" }\n' "$$crate" "$(SIBLINGS)" "$$repo" "$$dir" >> .cargo/config.toml; \
	  done; \
	done; echo "wrote .cargo/config.toml"; \
	meta=$$(mktemp); \
	if ! $(CARGO) metadata --format-version 1 >/dev/null 2>"$$meta"; then \
	  cat "$$meta" >&2; rm -f "$$meta" .cargo/config.toml; \
	  [ ! -f .cargo/Cargo.lock.pre-local ] || { cp .cargo/Cargo.lock.pre-local Cargo.lock; rm -f .cargo/Cargo.lock.pre-local; }; \
	  echo "use-local: cargo metadata failed under the patch" >&2; exit 1; \
	fi; \
	if grep -q 'patch .* was not used' "$$meta"; then \
	  cat "$$meta" >&2; rm -f "$$meta" .cargo/config.toml; \
	  [ ! -f .cargo/Cargo.lock.pre-local ] || { cp .cargo/Cargo.lock.pre-local Cargo.lock; rm -f .cargo/Cargo.lock.pre-local; }; \
	  echo "use-local: a patch was not used; the sibling's version does not satisfy the requirement" >&2; exit 1; \
	fi; \
	rm -f "$$meta"

.PHONY: use-remote
use-remote:
	rm -f .cargo/config.toml
	@if [ -f .cargo/Cargo.lock.pre-local ]; then mv .cargo/Cargo.lock.pre-local Cargo.lock; echo "restored Cargo.lock from the pre-local snapshot"; fi

# =============================================================================
# Composite
# =============================================================================

.NOTPARALLEL: ci
.PHONY: ci
ci: fspec-preflight
	$(MAKE) -j1 fmt-check spec lint msrv deny rustdoc test

# =============================================================================
# Documentation figures
# =============================================================================
# docs/images/figures.json defines the figures; ixd-figures (agent-ix/ix-diagrams) renders them.
# Install it with: cargo install --git https://github.com/agent-ix/ix-diagrams ix-diagrams

IXD ?= ixd

.PHONY: figures
figures:
	$(IXD) build docs/images/figures.json --out docs/images

.PHONY: figures-check
figures-check:
	$(IXD) build docs/images/figures.json --out docs/images --check
