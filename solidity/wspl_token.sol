// SPDX-License-Identifier: MIT
pragma solidity <=0.8.28;

import "./interface.sol";
import "./shared.sol";

contract WSplToken  is Shared{
    struct AccountBase58 {
        string mint;
        string owner;
        uint64 amount;
        string delegate;
        ISplToken.AccountState state;
        bool is_native;
        uint64 native_value;
        uint64 delegated_amount;
        string close_authority;
    }

    function program_id() public view returns (string memory) {
        bytes32 key = SplToken.program_id();
        bytes memory b58 = SystemProgram.bytes32_to_base58(key);

        return string(b58);
    }

    function transfer(bytes32 from, bytes32 to, uint64 amount) public {
        ISplToken.Seed[] memory seeds = new ISplToken.Seed[](0);
        (bool success, bytes memory result) = spl_token_address.delegatecall(
            abi.encodeWithSignature("transfer(bytes32,bytes32,uint64,(bytes)[])", from, to, amount, seeds)
        );

        require (success, string(Shared.revert_msg(result)));
    }

    function transfer(string memory from, string memory to, uint64 amount) public   {
        bytes32 from_ = SystemProgram.base58_to_bytes32(bytes(from));
        bytes32 to_ = SystemProgram.base58_to_bytes32(bytes(to));
        transfer(from_, to_, amount);
    }


    function init_account(string memory acc, string memory mint, string memory owner) public {
        bytes32 acc_ = SystemProgram.base58_to_bytes32(bytes(acc));
        bytes32 mint_ = SystemProgram.base58_to_bytes32(bytes(mint));
        bytes32 owner_ = SystemProgram.base58_to_bytes32(bytes(owner));
        
        SplToken.initialize_account3(acc_, mint_, owner_);
    }

    function account_state(string memory key) public view returns(AccountBase58 memory) {
        bytes32 key_ = SystemProgram.base58_to_bytes32(bytes(key));
        ISplToken.Account memory acc = SplToken.account_state(key_);
        
        AccountBase58 memory acc_b58;
        acc_b58.mint = string(SystemProgram.bytes32_to_base58(acc.mint));
        acc_b58.owner = string(SystemProgram.bytes32_to_base58(acc.owner));
        acc_b58.amount = acc.amount;
        acc_b58.delegate = string(SystemProgram.bytes32_to_base58(acc.delegate));
        acc_b58.state = acc.state;
        acc_b58.is_native = acc.is_native;
        acc_b58.native_value = acc.native_value;
        acc_b58.delegated_amount = acc.delegated_amount;
        acc_b58.close_authority = string(SystemProgram.bytes32_to_base58(acc.close_authority));

        return acc_b58;
    } 
}
