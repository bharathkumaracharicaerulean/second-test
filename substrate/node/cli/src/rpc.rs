// This file is part of Substrate.

// Copyright (C) Parity Technologies (UK) Ltd.
// SPDX-License-Identifier: GPL-3.0-or-later WITH Classpath-exception-2.0

// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.

//! RPC functionality for the Substrate node.
//! This module provides the RPC server implementation and related functionality.

use std::sync::Arc;
use jsonrpsee::RpcModule;
use sc_client_api::{
    backend::{Backend, StateBackend, StorageProvider},
    client::BlockchainEvents,
};
use sc_rpc::SubscriptionTaskExecutor;
use sc_transaction_pool_api::TransactionPool;
use sp_api::ProvideRuntimeApi;
use sp_blockchain::{Error as BlockChainError, HeaderBackend, HeaderMetadata};
use sp_runtime::traits::Block as BlockT;

/// Full client dependencies for RPC functionality.
/// This struct holds all the necessary components for setting up the RPC server.
pub struct FullDeps<C, P> {
    /// The client instance to use for blockchain interactions.
    pub client: Arc<C>,
    /// Transaction pool instance for handling transactions.
    pub pool: Arc<P>,
    /// Whether to deny unsafe RPC calls.
    /// When true, potentially dangerous RPC calls will be rejected.
    pub deny_unsafe: bool,
}

/// Instantiate all Full RPC extensions.
/// This function sets up the RPC server with all necessary extensions and dependencies.
///
/// # Arguments
///
/// * `deps` - The full dependencies required for RPC functionality.
///
/// # Returns
///
/// * `Ok(RpcModule)` - The configured RPC module ready to be served.
/// * `Err(Box<dyn Error>)` - If there was an error setting up the RPC module.
///
/// # Type Parameters
///
/// * `Block` - The block type used by the blockchain.
/// * `C` - The client type that provides blockchain functionality.
/// * `P` - The transaction pool type.
pub fn create_full<C, P, Block>(
    deps: FullDeps<C, P>,
) -> Result<RpcModule<()>, Box<dyn std::error::Error + Send + Sync>>
where
    Block: BlockT,
    C: ProvideRuntimeApi<Block>
        + HeaderBackend<Block>
        + BlockchainEvents<Block>
        + HeaderMetadata<Block, Error = BlockChainError>
        + Send
        + Sync
        + 'static,
    C::Api: substrate_frame_rpc_system::AccountNonceApi<Block, sp_runtime::AccountId32, u32>,
    P: TransactionPool + 'static,
{
    use substrate_frame_rpc_system::{System, SystemApiServer};

    // Create a new RPC module
    let mut module = RpcModule::new(());
    let FullDeps { client, pool, deny_unsafe } = deps;

    // Merge the system RPC extension into the module
    module.merge(System::new(client, pool, deny_unsafe).into_rpc())?;

    Ok(module)
} 