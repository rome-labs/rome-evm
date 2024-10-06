CONFIG_DIR := ../rome-config

INIT_CMD := $(or $(shell which initializer), cargo run --bin initializer --)

include ../rome-scripts/config/Makefile

MINT_TO := ${ROME_EVM_MINT_ADDR}
MINT_AMOUNT := 1000000000000000000

.PHONY: build

build: config
	cargo build-sbf

airdrop: config
	@echo "Airdropping to ${PAYER_ADDR}"
	solana airdrop 5000 --url ${SOLANA_URL} ${PAYER_ADDR} 

deploy: build
	@echo "Deploying ROME_EVM program"
	@if [ -z "${CONTRACT_OWNER}" ]; then echo "CONTRACT_OWNER is not set"; exit 1; fi
	@if [ -z "${CHAIN_ID}" ]; then echo "CHAIN_ID is not set"; exit 1; fi
	solana program deploy \
  		--program-id "${ROME_EVM_KEYPAIR_FILE}" ./target/deploy/rome_evm.so \
  		--upgrade-authority "${PAYER_KEYPAIR_FILE}" \
  		--fee-payer "${PAYER_KEYPAIR_FILE}"

init: build
	env SOLANA_RPC=${SOLANA_URL} MINT_AMOUNT=${MINT_AMOUNT} MINT_TO=${MINT_TO} $(INIT_CMD)
