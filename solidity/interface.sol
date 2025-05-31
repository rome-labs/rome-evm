// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

interface ISplToken {
    function transfer(bytes32 to, bytes32 mint, uint256 amount) external;
    function initialize_account3(bytes32 acc, bytes32 mint, bytes32 owner) external;
    function balance_ge(address caller, bytes32 mint, uint256 amount) external;
}

interface IAssociatedSplToken {
    function create_associated_token_account(address user, bytes32 mint) external;
}

interface ISystemProgram {
    function create_account(bytes32 owner, uint64 len, address user, bytes32 salt) external;
    function allocate(bytes32 acc, uint64 space) external;
    function assign(bytes32 acc, bytes32 owner) external;
    function transfer_(bytes32 to, uint64 amount) external;
}

address constant spl_token_address = address(0xff00000000000000000000000000000000000005);
address constant aspl_token_address = address(0xFF00000000000000000000000000000000000006);
address constant system_program_address = address(0xfF00000000000000000000000000000000000007);

ISplToken constant SplProgram = ISplToken(spl_token_address);
IAssociatedSplToken constant ASplProgram = IAssociatedSplToken(aspl_token_address);
ISystemProgram constant SystemProgram = ISystemProgram(system_program_address);


