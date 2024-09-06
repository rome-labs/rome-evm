ARG BUILDER_REF_NAME
ARG IMAGE_NAME

FROM ${IMAGE_NAME}:${BUILDER_REF_NAME} as builder

# Build Rome-EVM smart-contract with test-environment parameters
WORKDIR /opt/rome-evm/program
RUN CHAIN_ID=1001 CONTRACT_OWNER=8q76RPN5Tm6thVoQAUFhUP2diddGgtDLA6B6eShSazB2 cargo build-sbf

FROM solanalabs/solana:v1.18.17

# Copy binaries
COPY --from=builder /opt/rome-evm/target/deploy/rome_evm.so /opt/
COPY rome-evm/ci /opt/ci

ENTRYPOINT [ "/opt/ci/start_solana.sh" ]
