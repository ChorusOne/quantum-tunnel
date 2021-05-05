build:
	cargo build --features $(CHAIN)

format:
	find . -name "*.rs" -exec rustfmt --edition=2018 {} \;

release:
	cargo build --release --features $(CHAIN)

run:
	cargo run --features $(CHAIN) -- start
