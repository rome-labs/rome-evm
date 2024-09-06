# rome-evm

build contract:
- cd program
- cargo build-sbf

deploy contract:
- solana program deploy --program-id /opt/ci/upgradeable-rome-keypair.json ../target/deploy/rome_evm.so 

extend program account data:
- solana program extend CaQC27sVhdPyZF7defivoTQ48E8ws4tXvJfXYPRXboaH 5000

where CaQC27sVhdPyZF7defivoTQ48E8ws4tXvJfXYPRXboaH - is Pubkey of the keypair upgradeable-rome-keypair.json
