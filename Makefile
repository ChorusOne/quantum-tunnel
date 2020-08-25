build:
	cargo +nightly build

format:
	find . -name "*.rs" -exec rustfmt --edition=2018 {} \;

release:
	cargo +nightly build --release

run:
	cargo run -- start
