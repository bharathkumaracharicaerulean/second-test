use sc_rpc::DenyUnsafe;
use sc_transaction_pool_api::TransactionPool;
use sp_blockchain::HeaderBackend;
use std::sync::Arc;
use jsonrpsee::RpcModule;
use sc_client_api::BlockBackend;

use kitchensink_runtime::opaque::Block;
use sp_blockchain::{Error as BlockChainError, HeaderMetadata};

/// Full client dependencies.
pub struct FullDeps<C, P> {
    /// The client instance to use.
    pub client: Arc<C>,
    /// Transaction pool instance.
    pub pool: Arc<P>,
    /// Whether to deny unsafe calls
    pub deny_unsafe: bool,
}

/// Instantiate all Full RPC extensions.
pub fn create_full<C, P>(deps: FullDeps<C, P>) -> Result<RpcModule<()>, Box<dyn std::error::Error + Send + Sync>>
where
    C: HeaderBackend<Block>
        + BlockBackend<Block>
        + HeaderMetadata<Block, Error = BlockChainError>
        + Send
        + Sync
        + 'static,
    P: TransactionPool + 'static,
{
    let mut module = RpcModule::new(());
    let FullDeps { client: _, pool: _, deny_unsafe: _ } = deps;

    // We're not adding any RPC methods since our runtime is minimal
    Ok(module)
} 