use gw_types::{
    core::Status,
    packed::{GlobalState, RollupConfig},
};
use validator_utils::ckb_std::ckb_constants::Source;

use crate::{
    cells::{
        collect_burn_cells, collect_custodian_locks, collect_deposition_locks,
        collect_withdrawal_locks, find_stake_cell,
    },
    error::Error,
};

pub mod challenge;
pub mod revert;
pub mod reward;
pub mod submit_block;

/// this function ensure transaction doesn't contains any deposition / withdrawal / custodian / stake cells
pub fn check_rollup_lock_cells(
    rollup_type_hash: &[u8; 32],
    config: &RollupConfig,
) -> Result<(), Error> {
    if !collect_deposition_locks(rollup_type_hash, config, Source::Input)?.is_empty() {
        return Err(Error::Challenge);
    }
    if !collect_deposition_locks(rollup_type_hash, config, Source::Output)?.is_empty() {
        return Err(Error::Challenge);
    }
    if !collect_withdrawal_locks(rollup_type_hash, config, Source::Input)?.is_empty() {
        return Err(Error::Challenge);
    }
    if !collect_withdrawal_locks(rollup_type_hash, config, Source::Output)?.is_empty() {
        return Err(Error::Challenge);
    }
    if !collect_custodian_locks(rollup_type_hash, config, Source::Input)?.is_empty() {
        return Err(Error::Challenge);
    }
    if !collect_custodian_locks(rollup_type_hash, config, Source::Output)?.is_empty() {
        return Err(Error::Challenge);
    }
    if find_stake_cell(rollup_type_hash, config, Source::Input, None)?.is_some() {
        return Err(Error::Challenge);
    }
    if find_stake_cell(rollup_type_hash, config, Source::Output, None)?.is_some() {
        return Err(Error::Challenge);
    }
    Ok(())
}

pub fn check_status(global_state: &GlobalState, status: Status) -> Result<(), Error> {
    let expected_status: u8 = status.into();
    let status: u8 = global_state.status().into();
    if status != expected_status {
        return Err(Error::InvalidStatus);
    }
    Ok(())
}
