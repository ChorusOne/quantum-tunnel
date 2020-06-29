build:
	cargo build

format:
	find . -name "*.rs" -exec rustfmt --edition=2018 {} \;

release:
	cargo build --release

run:
	cargo run -- start
