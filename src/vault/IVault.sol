interface IVault {
    struct UnlockRequest {
        uint64 creation_time;
        uint128 share_amount;
        uint64 batch_id;
    }

    // Write Functions
    function stake() external returns (uint128 new_shares);
    function request_unlock(uint128 shares) external;
    function cancel_unlock_request(uint128 user_unlock_id) external;
    function send_batch_unlock_requests(uint64[] batch_ids) external;
    function redeem(address user, uint64 unlock_id) external;
    function redeem_with_withdraw(address user, uint64 unlock_id) external;
    function delegate_withdraw_unbonded() external;
    function compound() external returns (uint128 incentive);

    // Restricted Functions
    function withdraw_fees() external;
    function adjust_minimum_stake(uint128 new_minimum_stake) external;
    function transfer_role_owner(address new_account) external;
    function adjust_fee(uint16 new_fee) external;
    function adjust_incentive(uint16 new_incentive) external;
    function transfer_role_adjust_fee(address new_account) external;
    function transfer_role_adjust_fee_admin(address new_account) external;

    // Read-only Functions
    function get_batch_id() external view returns (uint64);
    function get_creation_time() external view returns (uint64);
    function get_role_owner() external view returns (address);
    function get_role_adjust_fee() external view returns (uint64);
    function get_role_adjust_fee_admin() external view returns (uint64);
    function get_total_pooled() external view returns (uint128);
    function get_total_shares() external view returns (uint128);
    function get_current_virtual_shares() external view returns (uint128);
    function get_minimum_stake() external view returns (uint128);
    function get_fee_percentage() external view returns (uint16);
    function get_incentive_percentage() external view returns (uint16);
    function get_share_token_contract() external view returns (address);
    function get_registry_contract() external view returns (address);
    function get_shares_from_azero(uint128 azero) external view returns (uint128);
    function get_azero_from_shares(uint128 shares) external view returns (uint128);
    function get_unlock_requests(address user) external view returns (UnlockRequest[]);
    function get_unlock_request_count(address user) external view returns (uint128);
    function get_batch_unlock_requests(uint64 batch_id) external view returns (uint128, uint128, uint64);
    function get_weight_imbalances(uint128 total_pooled) external view returns (uint128, uint128, uint128, uint128[], int128[]);
}
