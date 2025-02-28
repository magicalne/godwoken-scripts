use gw_common::{
    h256_ext::H256Ext,
    smt::{Blake2bHasher, CompiledMerkleProof},
    H256,
};
use gw_types::{
    core::Status,
    packed::{BlockMerkleState, Byte32, GlobalState, RawL2Block, RollupConfig},
    prelude::*,
};
use gw_utils::gw_types;
use gw_utils::{
    cells::{
        lock_cells::{
            collect_burn_cells, collect_stake_cells, fetch_capacity_and_sudt_value,
            find_challenge_cell,
        },
        types::ChallengeCell,
        utils::search_lock_hashes,
    },
    ckb_std::{
        ckb_constants::Source,
        debug,
        high_level::load_input_since,
        since::{LockValue, Since},
    },
};
use gw_utils::{
    gw_common,
    gw_types::packed::{RawL2BlockReader, RollupRevertReader},
};

use super::{check_rollup_lock_cells_except_stake, check_status};
use alloc::{collections::BTreeSet, vec::Vec};
use gw_utils::error::Error;

/// Check challenge cell is maturity(on the layer1)
fn check_challenge_maturity(
    config: &RollupConfig,
    challenge_cell: &ChallengeCell,
) -> Result<(), Error> {
    let challenge_maturity_blocks: u64 = config.challenge_maturity_blocks().unpack();
    let since = Since::new(load_input_since(challenge_cell.index, Source::Input)?);
    if let Some(LockValue::BlockNumber(n)) = since.extract_lock_value() {
        if since.is_relative() && n >= challenge_maturity_blocks {
            return Ok(());
        }
    }
    Err(Error::InvalidChallengeCell)
}

fn check_challenge_cell(
    rollup_type_hash: &H256,
    config: &RollupConfig,
    challenge_cell: &ChallengeCell,
    revert_target_block_hash: &H256,
) -> Result<(), Error> {
    // check challenge maturity
    check_challenge_maturity(config, challenge_cell)?;
    // check other challenge cells
    let has_output_challenge =
        find_challenge_cell(rollup_type_hash, config, Source::Output)?.is_some();
    if has_output_challenge {
        return Err(Error::InvalidChallengeCell);
    }
    // check challenge target
    let challenge_target = challenge_cell.args.target();
    let challenge_block_hash: H256 = challenge_target.block_hash().unpack();
    if &challenge_block_hash != revert_target_block_hash {
        return Err(Error::InvalidChallengeCell);
    }
    Ok(())
}

pub fn get_receiver_cells_capacity(
    config: &RollupConfig,
    lock_hash: &[u8; 32],
    source: Source,
) -> Result<u128, Error> {
    let capacity = search_lock_hashes(lock_hash, source)
        .into_iter()
        .map(|index| {
            fetch_capacity_and_sudt_value(config, index, source).map(|value| value.capacity.into())
        })
        .collect::<Result<Vec<u128>, Error>>()?
        .into_iter()
        .sum();
    Ok(capacity)
}

/// Check rewards
fn check_rewards(
    rollup_type_hash: &H256,
    config: &RollupConfig,
    reverted_blocks: &[RawL2BlockReader],
    challenge_cell: &ChallengeCell,
) -> Result<(), Error> {
    let reverted_block_stake_set: BTreeSet<_> = reverted_blocks
        .iter()
        .map(|b| b.stake_cell_owner_lock_hash().to_entity())
        .collect();

    let stake_cells = collect_stake_cells(rollup_type_hash, config, Source::Input)?;
    let reverted_stake_cells_set: BTreeSet<_> = stake_cells
        .iter()
        .map(|cell| cell.args.owner_lock_hash())
        .collect();
    // ensure stake cells are all belongs to reverted blocks and no missing stake cells
    if reverted_block_stake_set != reverted_stake_cells_set {
        debug!("reverted stake cells isn't according to reverted block stake set");
        return Err(Error::InvalidStakeCell);
    }

    // calcuate rewards assets & burn assets
    let total_stake_capacity: u128 = stake_cells.iter().map(|cell| cell.capacity as u128).sum();
    let reward_burn_rate: u8 = config.reward_burn_rate().into();
    let expected_reward_capacity =
        total_stake_capacity.saturating_mul(reward_burn_rate.into()) / 100;
    let expected_burn_capacity = total_stake_capacity.saturating_sub(expected_reward_capacity);
    // collect rewards receiver cells capacity
    let received_capacity: u128 = {
        let rewards_receiver_lock_hash = challenge_cell.args.rewards_receiver_lock().hash();
        let input_capacity =
            get_receiver_cells_capacity(config, &rewards_receiver_lock_hash, Source::Input)?;
        let output_capacity =
            get_receiver_cells_capacity(config, &rewards_receiver_lock_hash, Source::Output)?;
        output_capacity.saturating_sub(input_capacity)
    };
    // make sure rewards are sent to the challenger
    if received_capacity
        < expected_reward_capacity.saturating_add(challenge_cell.value.capacity.into())
    {
        return Err(Error::InvalidChallengeReward);
    }
    // check burned assets
    let burned_capacity: u128 = {
        let input_burned_capacity: u128 = collect_burn_cells(config, Source::Input)?
            .into_iter()
            .map(|c| c.value.capacity as u128)
            .sum();
        let output_burned_capacity: u128 = collect_burn_cells(config, Source::Output)?
            .into_iter()
            .map(|c| c.value.capacity as u128)
            .sum();
        output_burned_capacity.saturating_sub(input_burned_capacity)
    };
    if burned_capacity < expected_burn_capacity {
        return Err(Error::InvalidChallengeReward);
    }
    Ok(())
}

fn check_reverted_blocks(
    config: &RollupConfig,
    reverted_blocks: &[RawL2BlockReader],
    revert_args: &RollupRevertReader,
    prev_global_state: &GlobalState,
    post_global_state: &GlobalState,
) -> Result<GlobalState, Error> {
    if reverted_blocks.is_empty() {
        return Err(Error::InvalidRevertedBlocks);
    }
    let reverted_block_hashes: Vec<H256> =
        reverted_blocks.iter().map(|b| b.hash().into()).collect();
    let reverted_block_smt_keys: Vec<H256> = reverted_blocks
        .iter()
        .map(|b| RawL2Block::compute_smt_key(b.number().unpack()).into())
        .collect();
    // check reverted_blocks is continues
    {
        let mut prev_hash: Byte32 = reverted_blocks[0].hash().pack();
        let mut prev_number = reverted_blocks[0].number().unpack();
        for b in reverted_blocks[1..].iter() {
            let hash = b.parent_block_hash();
            if hash.as_slice() != prev_hash.as_slice() {
                return Err(Error::InvalidRevertedBlocks);
            }
            let number: u64 = b.number().unpack();
            if number != prev_number + 1 {
                return Err(Error::InvalidRevertedBlocks);
            }
            prev_hash = hash.to_entity();
            prev_number = number;
        }

        // must revert from current point to the tip block
        let count: u64 = prev_global_state.block().count().unpack();
        let tip_number = count - 1;
        if prev_number != tip_number {
            return Err(Error::InvalidRevertedBlocks);
        }
    }
    // prove the target block exists in the main chain
    let block_merkle_proof = CompiledMerkleProof(revert_args.block_proof().unpack());
    let is_main_chain_block = {
        let leaves = reverted_block_smt_keys
            .clone()
            .into_iter()
            .zip(reverted_block_hashes.clone())
            .collect();
        block_merkle_proof
            .verify::<Blake2bHasher>(&prev_global_state.block().merkle_root().unpack(), leaves)?
    };
    if !is_main_chain_block {
        return Err(Error::InvalidRevertedBlocks);
    }
    // prove the target block isn't in the prev reverted block root
    let reverted_block_merkle_proof =
        CompiledMerkleProof(revert_args.reverted_block_proof().unpack());
    let is_not_prev_reverted_block = {
        let reverted_block_root: H256 = prev_global_state.reverted_block_root().unpack();
        let leaves: Vec<_> = reverted_block_hashes
            .clone()
            .into_iter()
            .map(|hash| (hash, H256::zero()))
            .collect();
        reverted_block_merkle_proof.verify::<Blake2bHasher>(&reverted_block_root, leaves)?
    };
    if !is_not_prev_reverted_block {
        return Err(Error::InvalidRevertedBlocks);
    }
    // prove the target block in the post reverted block root
    let is_post_reverted_block = {
        let leaves: Vec<_> = reverted_block_hashes
            .into_iter()
            .map(|hash| (hash, H256::one()))
            .collect();
        reverted_block_merkle_proof
            .verify::<Blake2bHasher>(&post_global_state.reverted_block_root().unpack(), leaves)?
    };
    if !is_post_reverted_block {
        return Err(Error::InvalidRevertedBlocks);
    }
    let reverted_block_root = post_global_state.reverted_block_root();
    // calculate the prev block merkle state (delete reverted block hashes)
    let block_merkle_state = {
        let leaves = reverted_block_smt_keys
            .into_iter()
            .map(|smt_key| (smt_key, H256::zero()))
            .collect();
        let block_root = block_merkle_proof.compute_root::<Blake2bHasher>(leaves)?;
        let block_count = reverted_blocks[0].number();
        BlockMerkleState::new_builder()
            .merkle_root(block_root.pack())
            .count(block_count.to_entity())
            .build()
    };
    let account_merkle_state = reverted_blocks[0].prev_account();
    let tip_block_hash = reverted_blocks[0].parent_block_hash();
    let last_finalized_block_number = {
        let number: u64 = reverted_blocks[0].number().unpack();
        number
            .saturating_sub(1)
            .saturating_sub(config.finality_blocks().unpack())
    };
    // check post global state
    let reverted_post_global_state = {
        let status: u8 = Status::Running.into();
        prev_global_state
            .clone()
            .as_builder()
            .account(account_merkle_state.to_entity())
            .block(block_merkle_state)
            .tip_block_hash(tip_block_hash.to_entity())
            .last_finalized_block_number(last_finalized_block_number.pack())
            .reverted_block_root(reverted_block_root)
            .status(status.into())
            .build()
    };
    Ok(reverted_post_global_state)
}

/// Verify revert
/// 1. check revert merkle roots
/// 2. check reverted block root
/// 3. check other lock cells
pub fn verify(
    rollup_type_hash: H256,
    config: &RollupConfig,
    revert_args: RollupRevertReader,
    prev_global_state: &GlobalState,
    post_global_state: &GlobalState,
) -> Result<(), Error> {
    check_status(prev_global_state, Status::Halting)?;
    // check rollup lock cells,
    // we do not handle the reverting of lock cells in here,
    // instead we handle them in the submitting layer2 block action
    check_rollup_lock_cells_except_stake(&rollup_type_hash, config)?;
    // do not accept stake cells in the output
    if !collect_stake_cells(&rollup_type_hash, config, Source::Output)?.is_empty() {
        return Err(Error::InvalidStakeCell);
    }
    // load reverted blocks
    let reverted_blocks_vec = revert_args.reverted_blocks();
    let reverted_blocks: Vec<_> = reverted_blocks_vec.iter().collect();
    // check challenge cells
    let challenge_cell = find_challenge_cell(&rollup_type_hash, config, Source::Input)?
        .ok_or(Error::InvalidChallengeCell)?;
    // the first reverted block is challenged target block
    let challenged_block = reverted_blocks.get(0).ok_or(Error::InvalidRevertedBlocks)?;
    check_challenge_cell(
        &rollup_type_hash,
        config,
        &challenge_cell,
        &challenged_block.hash().into(),
    )?;
    check_rewards(&rollup_type_hash, config, &reverted_blocks, &challenge_cell)?;
    let reverted_global_state = check_reverted_blocks(
        config,
        &reverted_blocks,
        &revert_args,
        prev_global_state,
        post_global_state,
    )?;
    if post_global_state != &reverted_global_state {
        return Err(Error::InvalidPostGlobalState);
    }
    Ok(())
}
