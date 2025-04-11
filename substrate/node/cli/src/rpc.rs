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

    let mut module = RpcModule::new(());
    let FullDeps { client, pool, deny_unsafe } = deps;

    module.merge(System::new(client, pool, deny_unsafe).into_rpc())?;

    Ok(module)
} 