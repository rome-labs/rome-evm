FROM ubuntu:24.04 as build

RUN apt update && \
    apt upgrade -y && \
    apt install -y curl build-essential

RUN apt install -y rustup
RUN rustup default stable
RUN sh -c "$(curl -sSfL https://release.anza.xyz/v2.1.7/install)" && \
    /root/.local/share/solana/install/active_release/bin/sdk/sbf/scripts/install.sh

ENV PATH=${PATH}:/root/.local/share/solana/install/active_release/bin

COPY rome-evm /opt/rome-evm

WORKDIR /opt/rome-evm/program
RUN cargo test
RUN cargo build-sbf

FROM anzaxyz/agave:v2.1.7
COPY --from=build /opt/rome-evm/target/deploy/rome_evm.so /opt/
COPY --from=build /opt/rome-evm/ci /opt/ci

ENTRYPOINT [ "/opt/ci/start_solana.sh" ]



