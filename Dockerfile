FROM rust

RUN rustup install nightly-2024-05-16
RUN rustup default nightly-2024-05-16
RUN apt-get update
RUN apt-get -y install cmake

COPY . .
RUN --mount=type=ssh SQLX_OFFLINE=true CARGO_NET_FETCH_WITH_CLI=true cargo build --release 

CMD ./target/release/blitzserver
