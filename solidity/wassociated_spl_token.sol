// SPDX-License-Identifier: MIT
pragma solidity =0.8.28;

import "./interface.sol";
import "./shared.sol";


contract WAssociatedSplToken is Shared {
    event Message(string account);

    function program_id() public view returns (string memory) {
        bytes32 key = AssociatedSplToken.program_id();
        bytes memory b58 = SystemProgram.bytes32_to_base58(key);
        return string(b58);
    }

    function associated_token_address_seeds(bytes32 user, bytes32 mint) private view returns(ISystemProgram.Seed[] memory) {
        bytes32 spl_program_id = SplToken.program_id();
        bytes memory spl_program_id_ = Shared.bytes32_to_bytes(spl_program_id);
        bytes memory user_ = Shared.bytes32_to_bytes(user);
        bytes memory mint_ = Shared.bytes32_to_bytes(mint);

        ISystemProgram.Seed[] memory seeds = new ISystemProgram.Seed[](3);
        seeds[0] = ISystemProgram.Seed(user_);
        seeds[1] = ISystemProgram.Seed(spl_program_id_);
        seeds[2] = ISystemProgram.Seed(mint_);

        return seeds;
    }

    function associated_token_address(bytes32 user, bytes32 mint) public view returns(bytes32, uint8) {
        bytes32 program_id_ = AssociatedSplToken.program_id();
        ISystemProgram.Seed[] memory seeds = associated_token_address_seeds(user, mint);

        (bytes32 key, uint8 bump) = SystemProgram.find_program_address(program_id_, seeds);

        return (key, bump);
    }

    function associated_token_address(string memory user, string memory mint) public view returns(string memory, uint8) {
        bytes32 program_id_ = AssociatedSplToken.program_id();
        bytes32 user_ = SystemProgram.base58_to_bytes32(bytes(user));
        bytes32 mint_ = SystemProgram.base58_to_bytes32(bytes(mint));

        ISystemProgram.Seed[] memory seeds = associated_token_address_seeds(user_, mint_);

        (bytes32 key, uint8 bump) = SystemProgram.find_program_address(program_id_, seeds);
        bytes memory b58 = SystemProgram.bytes32_to_base58(key);

        return (string(b58), bump);
    }

    function associated_token_address(address user, string memory mint) public view returns(string memory, uint8) {
        bytes32 user_ = pda(user);

        bytes32 program_id_ = AssociatedSplToken.program_id();
        bytes32 mint_ = SystemProgram.base58_to_bytes32(bytes(mint));

        ISystemProgram.Seed[] memory seeds = associated_token_address_seeds(user_, mint_);

        (bytes32 key, uint8 bump) = SystemProgram.find_program_address(program_id_, seeds);
        bytes memory b58 = SystemProgram.bytes32_to_base58(key);

        return (string(b58), bump);
    }


    function associated_token_address(string memory mint) public view returns(string memory, uint8) {
        bytes32 user = pda(msg.sender);
        bytes32 program_id_ = AssociatedSplToken.program_id();
        bytes32 mint_ = SystemProgram.base58_to_bytes32(bytes(mint));

        ISystemProgram.Seed[] memory seeds = associated_token_address_seeds(user, mint_);

        (bytes32 key, uint8 bump) = SystemProgram.find_program_address(program_id_, seeds);
        bytes memory b58 = SystemProgram.bytes32_to_base58(key);

        return (string(b58), bump);
    }


    function create_associated_token_account(string memory user, string memory mint) public returns (string memory) {
        bytes32 user_ = SystemProgram.base58_to_bytes32(bytes(user));
        bytes32 mint_ = SystemProgram.base58_to_bytes32(bytes(mint));

        bytes32 key = AssociatedSplToken.create_associated_token_account(user_, mint_);
        bytes memory b58 = SystemProgram.bytes32_to_base58(key);
        
        emit Message(string(b58));
        return string(b58);
    }

    function create_associated_token_account(bytes32 user, bytes32 mint) public returns (bytes32) {

        bytes32 key = AssociatedSplToken.create_associated_token_account(user, mint);

        return key;
    }

    function pda(address user) private view returns (bytes32) {
        bytes32 rome_program = SystemProgram.rome_evm_program_id();
        ISystemProgram.Seed[] memory pda_seeds = Shared.balance_key_seeds(user, block.chainid);
        (bytes32 key,) = SystemProgram.find_program_address(rome_program, pda_seeds);
        return key;
    }
}

