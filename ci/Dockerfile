FROM solanalabs/rust:1.75.0 as build

RUN sh -c "$(curl -sSfL https://release.solana.com/v1.18.17/install)" && \
    /root/.local/share/solana/install/active_release/bin/sdk/sbf/scripts/install.sh
ENV PATH=${PATH}:/root/.local/share/solana/install/active_release/bin

COPY rome-evm /opt/rome-evm

WORKDIR /opt/rome-evm/program
RUN cargo test
RUN cargo build-sbf

FROM solanalabs/solana:v1.18.17
COPY --from=build /opt/rome-evm/target/deploy/rome_evm.so /opt/
COPY --from=build /opt/rome-evm/ci /opt/ci

ENTRYPOINT [ "/opt/ci/start_solana.sh" ]



