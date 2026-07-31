FROM ubuntu:24.04 AS builder

ENV DEBIAN_FRONTEND=non-interactive

RUN apt-get update && apt-get install -y \
    git \
    build-essential cmake pkgconf python3 \
    libevent-dev libboost-dev \
    libsqlite3-dev \
    libcapnp-dev capnproto \
    libzmq3-dev

RUN git clone -b 29.x https://github.com/bitcoin-inquisition/bitcoin.git /bitcoin

WORKDIR /bitcoin

RUN cmake -B build
RUN cmake --build build -j$(nproc)

FROM ubuntu:24.04

ENV DEBIAN_FRONTEND=non-interactive

# Only install what we need at runtime
RUN apt-get update && apt-get install -y \
    libevent-dev libzmq3-dev libsqlite3-dev

COPY --from=builder /bitcoin/build/bin/bitcoind /usr/local/bin/
COPY --from=builder /bitcoin/build/bin/bitcoin-cli /usr/local/bin/

EXPOSE 18443 18444

COPY bitcoin.conf /root/.bitcoin/bitcoin.conf

CMD ["bitcoind", "-printtoconsole"]