FROM solanalabs/solana:v1.18.17 AS solana

FROM solanalabs/rust:1.75.0 as initializer-builder

COPY rome-evm /opt/rome-evm

# Test Rome-EVM smart contract
WORKDIR /opt/rome-evm/program
RUN CHAIN_ID=1001 CONTRACT_OWNER=8q76RPN5Tm6thVoQAUFhUP2diddGgtDLA6B6eShSazB2 cargo test

# Build Initializer with default test values - it does not actually affect behavior of initializer
# still can be used with any rollup
WORKDIR /opt/rome-evm/
RUN CHAIN_ID=1001 CONTRACT_OWNER=8q76RPN5Tm6thVoQAUFhUP2diddGgtDLA6B6eShSazB2 cargo build --release --bin initializer

FROM solanalabs/rust:1.75.0 as evm-builder

RUN sh -c "$(curl -sSfL https://release.solana.com/v1.18.17/install)" && \
    /root/.local/share/solana/install/active_release/bin/sdk/sbf/scripts/install.sh
ENV PATH=${PATH}:/root/.local/share/solana/install/active_release/bin

COPY rome-evm/ci/build.sh  /opt/build.sh
COPY rome-evm/ci/deploy.sh  /opt/deploy.sh
COPY rome-evm/ci/initialize.sh /opt/initialize.sh
COPY rome-evm /opt/rome-evm

COPY --from=initializer-builder /opt/rome-evm/target/release/initializer /usr/bin/initializer
COPY --from=solana /usr/bin/solana /usr/bin/solana

WORKDIR /opt
