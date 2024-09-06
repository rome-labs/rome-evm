#!/bin/bash

PUBKEY=$(solana address -k /opt/ci/rome-keypair.json)
GENESIS=(
      --bpf-program ${PUBKEY} BPFLoader2111111111111111111111111111111111 /opt/rome_evm.so
)
VALIDATOR=(
        --gossip-host $(hostname -i)
        --log-messages-bytes-limit 50000
)

export SOLANA_RUN_SH_GENESIS_ARGS=${GENESIS[@]}
export SOLANA_RUN_SH_VALIDATOR_ARGS=${VALIDATOR[@]}


/usr/bin/solana-run.sh
