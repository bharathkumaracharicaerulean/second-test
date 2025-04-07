use jsonrpsee::RpcModule;
use kitchensink_runtime::opaque::Block;
use sc_transaction_pool::ChainApi;
use std::sync::Arc;

/// Full client dependencies.
pub struct FullDeps<C, P> {
    /// The client instance to use.
    pub client: Arc<C>,
    pub pool: Arc<P>,
    pub deny_unsafe: bool,
}

/// Instantiate all Full RPC extensions.
pub fn create_full<C, P>(
    deps: FullDeps<C, P>,
) -> Result<RpcModule<()>, Box<dyn std::error::Error + Send + Sync>>
where
    C: sp-api::ProvideRuntimeApi<Block>,
    C: sc_client_api::BlockBackend<Block>,
    C: Send + Sync + 'static,
    C::Api: substrate_frame_rpc_system::AccountNonceApi<Block, C::AccountId, C::Index>,
    P: ChainApi<Block, C> + 'static,
{
    let mut module = RpcModule::new(());
    let FullDeps { client, pool, deny_unsafe } = deps;

    module.merge(substrate_frame_rpc_system::System::new(client.clone(), pool, deny_unsafe).into_rpc())?;

    Ok(module)
} 