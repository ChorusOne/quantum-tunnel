FROM rust:slim-buster

RUN apt update && apt install -y gcc libprotobuf-dev
WORKDIR /usr/src/quantum-tunnel
COPY Cargo.toml .
COPY Cargo.lock .
COPY src src

RUN cargo build --release

RUN cp target/release/quantum-tunnel /usr/local/bin/quantum-tunnel
RUN chmod +x /usr/local/bin/quantum-tunnel

CMD ["quantum-tunnel"]
