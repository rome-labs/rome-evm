// SPDX-License-Identifier: MIT
pragma solidity <=0.8.28;

import "./interface.sol";

contract Shared {

    function bytes32_to_bytes(bytes32 src) internal pure returns (bytes memory) {
        bytes memory dst = new bytes(32);

        assembly {
            mstore(add(dst, 32), src)
        }
        return dst;
    }

    function uint_to_bytes(uint src) internal pure returns (bytes memory) {
        bytes memory dst = new bytes(32);

        assembly {
            mstore(add(dst, 32), src)
        }
        return dst;
    }

    function bytes_to_bytes32(bytes memory src) internal pure returns (bytes32) {
        bytes32 dst;

        assembly {
            dst := mload(add(src, 32))
        }
        return  dst;
    }

    function chain_id_le(uint chain_id) internal pure returns (bytes memory ) {
        return abi.encodePacked(
                            uint8( chain_id),
                            uint8(chain_id >> 8 ),
                            uint8(chain_id >> 16),
                            uint8(chain_id >> 24),
                            uint8(chain_id >> 32),
                            uint8(chain_id >> 40),
                            uint8(chain_id >> 48),
                            uint8(chain_id >> 56)
                            );
    }

    function balance_key_seeds(address user, uint chain_id) internal pure returns(ISystemProgram.Seed[] memory) {
        bytes memory chain_le = chain_id_le(chain_id);

        ISystemProgram.Seed[] memory seeds = new ISystemProgram.Seed[](3);
        seeds[0] = ISystemProgram.Seed(chain_le);
        seeds[1] = ISystemProgram.Seed(bytes("ACCOUN_SEED"));
        seeds[2] = ISystemProgram.Seed(abi.encodePacked(user));

        return seeds;
    }
    
    function revert_msg(bytes memory _returnData) internal pure returns (string memory) {
        if (_returnData.length < 68) return '';

        bytes memory mes;
        assembly {
            mes := add(_returnData, 0x04)
        }
        return abi.decode(mes, (string));
    }
}

