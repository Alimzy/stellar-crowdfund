.PHONY: test fmt lint check doc build size clean
# Tighten after the first CI run reports the real size (see CONTRIBUTING.md).
MAX_WASM_BYTES ?= 40000

test:
	cargo test --locked

fmt:
	cargo fmt --all

lint:
	cargo clippy --locked --all-targets -- -D warnings

doc:
	RUSTDOCFLAGS="-D warnings" cargo doc --locked --no-deps

check:
	cargo fmt --all -- --check
	$(MAKE) lint
	$(MAKE) test
	$(MAKE) doc

build:
	cargo build --locked --release --target wasm32v1-none -p crowdfund

size: build
	@f=target/wasm32v1-none/release/crowdfund.wasm; \
	size=$$(wc -c < "$$f" | tr -d '[:space:]'); \
	echo "size: $$size bytes (budget: $(MAX_WASM_BYTES) bytes)"; \
	if [ "$$size" -gt "$(MAX_WASM_BYTES)" ]; then \
		echo "ERROR: WASM size exceeds budget"; exit 1; \
	fi

clean:
	cargo clean
