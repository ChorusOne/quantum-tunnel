FROM rust:alpine

RUN apk add --update --no-cache rust gcc

WORKDIR /usr/src/quantum-tunnel
COPY Cargo.toml .
COPY Cargo.lock .
COPY src src

RUN cargo install --path .

CMD ["quantum-tunnel"]
