use std::sync::{Arc};
use std::sync::atomic::{AtomicUsize, AtomicBool};
use std::time::Duration;
use kitchensink_runtime::{self, opaque::Block, RuntimeApi};
use sc_executor::WasmExecutor;
use sc_service::{error::Error as ServiceError, Configuration, TaskManager};
use sc_telemetry::{Telemetry, TelemetryWorker};
use sc_consensus_aura::{SlotProportion, StartAuraParams};
use sc_client_api::BlockBackend;
use sp_consensus_aura::sr25519::AuthorityPair as AuraPair;
use sc_transaction_pool::FullChainApi;
use sc_transaction_pool_api::OffchainTransactionPoolFactory;
use sc_consensus_slots::BackoffAuthoringOnFinalizedHeadLagging;
use sc_consensus_grandpa::{
    SharedVoterState, Config as GrandpaConfig, GrandpaParams, VotingRulesBuilder,
};
use sc_network::config::FullNetworkConfiguration;
use sc_network_sync::SyncingService;
use sc_service::build_network;
use sc_utils::mpsc::tracing_unbounded;

/// The full client type definition.
pub type FullClient = sc_service::TFullClient<Block, RuntimeApi, WasmExecutor>;
type FullBackend = sc_service::TFullBackend<Block>;
type FullSelectChain = sc_consensus::LongestChain<FullBackend, Block>;
type TransactionPool = sc_transaction_pool::BasicPool<FullChainApi<FullClient, Block>, Block>;

/// Creates a new partial service with all the necessary components.
pub fn new_partial(
    config: &Configuration,
) -> Result<sc_service::PartialComponents<
    FullClient,
    FullBackend,
    FullSelectChain,
    sc_consensus::DefaultImportQueue<Block>,
    TransactionPool,
    (
        sc_consensus_grandpa::GrandpaBlockImport<FullBackend, Block, FullClient, FullSelectChain>,
        sc_consensus_grandpa::LinkHalf<Block, FullClient, FullSelectChain>,
        Option<Telemetry>,
    ),
>, ServiceError> {
    let telemetry = config
        .telemetry_endpoints
        .clone()
        .filter(|x| !x.is_empty())
        .map(|endpoints| -> Result<_, sc_telemetry::Error> {
            let worker = TelemetryWorker::new(16)?;
            let telemetry = worker.handle().new_telemetry(endpoints);
            Ok((worker, telemetry))
        })
        .transpose()?;

    let executor = WasmExecutor::builder()
        .with_execution_method(config.executor.wasm_method)
        .with_max_runtime_instances(config.executor.max_runtime_instances)
        .with_runtime_cache_size(config.executor.runtime_cache_size)
        .build();

    let (client, backend, keystore_container, task_manager) =
        sc_service::new_full_parts::<Block, RuntimeApi, _>(
            config,
            telemetry.as_ref().map(|(_, telemetry)| telemetry.handle()),
            executor,
        )?;

    let client = Arc::new(client);

    let telemetry = telemetry.map(|(worker, telemetry)| {
        task_manager.spawn_handle().spawn("telemetry", None, worker.run());
        telemetry
    });

    let select_chain = sc_consensus::LongestChain::new(backend.clone());

    let transaction_pool = Arc::new(sc_transaction_pool::BasicPool::new_full(
        sc_transaction_pool::Options::default(),
        config.role.is_authority().into(),
        config.prometheus_registry(),
        task_manager.spawn_essential_handle(),
        client.clone(),
    ));

    let (grandpa_block_import, grandpa_link) = sc_consensus_grandpa::block_import(
        client.clone(),
        0u32,
        &(client.clone() as Arc<_>),
        select_chain.clone(),
        telemetry.as_ref().map(|x| x.handle()),
    )?;

    let slot_duration = sc_consensus_aura::slot_duration(&*client)?;

    let import_queue = sc_consensus_aura::import_queue::<AuraPair, _, _, _, _, _>(
        sc_consensus_aura::ImportQueueParams {
            block_import: grandpa_block_import.clone(),
            justification_import: Some(Box::new(grandpa_block_import.clone())),
            client: client.clone(),
            create_inherent_data_providers: move |_, ()| async move {
                let timestamp = sp_timestamp::InherentDataProvider::from_system_time();
                let slot = sp_consensus_aura::inherents::InherentDataProvider::from_timestamp_and_slot_duration(
                    *timestamp,
                    slot_duration,
                );
                Ok((slot, timestamp))
            },
            spawner: &task_manager.spawn_essential_handle(),
            registry: config.prometheus_registry(),
            telemetry: telemetry.as_ref().map(|x| x.handle()),
            compatibility_mode: Default::default(),
            check_for_equivocation: Default::default(),
        },
    )?;

    Ok(sc_service::PartialComponents {
        client,
        backend,
        task_manager,
        import_queue,
        keystore_container,
        select_chain,
        transaction_pool,
        other: (grandpa_block_import, grandpa_link, telemetry),
    })
}

/// Creates a new full service with all components initialized and running.
pub async fn new_full(config: Configuration) -> Result<TaskManager, ServiceError> {
    let sc_service::PartialComponents {
        client,
        backend,
        mut task_manager,
        import_queue,
        keystore_container,
        select_chain,
        transaction_pool,
        other: (block_import, grandpa_link, mut telemetry),
    } = new_partial(&config)?;

    let mut net_config = FullNetworkConfiguration::new(&config.network, config.prometheus_registry().cloned());

    let (sync_tx, _) = tracing_unbounded("sync", 1000); // Provide a buffer size
    let sync_service = SyncingService::new(sync_tx, Arc::new(AtomicUsize::new(0)), Arc::new(AtomicBool::new(false)));

    let (network, _, _, _) = build_network(sc_service::BuildNetworkParams {
        config: &config,
        client: client.clone(),
        transaction_pool: transaction_pool.clone(),
        spawn_handle: task_manager.spawn_handle(),
        import_queue,
        block_announce_validator_builder: None,
        warp_sync_config: None,
        block_relay: None,
        metrics: sc_network::NotificationMetrics::new(config.prometheus_registry()),
        net_config,
    })?;

    if config.role.is_authority() {
        let proposer = sc_basic_authorship::ProposerFactory::new(
            task_manager.spawn_handle(),
            client.clone(),
            transaction_pool.clone(),
            config.prometheus_registry(),
            telemetry.as_ref().map(|x| x.handle()),
        );

        let slot_duration = sc_consensus_aura::slot_duration(&*client)?;

        let aura = sc_consensus_aura::start_aura::<AuraPair, _, _, _, _, _, _, _, _, _, _>(
            StartAuraParams {
                slot_duration,
                client: client.clone(),
                select_chain,
                block_import,
                proposer_factory: proposer,
                create_inherent_data_providers: move |_, ()| async move {
                    let timestamp = sp_timestamp::InherentDataProvider::from_system_time();
                    let slot = sp_consensus_aura::inherents::InherentDataProvider::from_timestamp_and_slot_duration(
                        *timestamp,
                        slot_duration,
                    );
                    Ok((slot, timestamp))
                },
                force_authoring: config.force_authoring,
                backoff_authoring_blocks: Some(BackoffAuthoringOnFinalizedHeadLagging::default()),
                keystore: keystore_container.keystore(),
                sync_oracle: sync_service.clone(),
                justification_sync_link: sync_service.clone(),
                block_proposal_slot_portion: SlotProportion::new(2f32 / 3f32),
                max_block_proposal_slot_portion: None,
                telemetry: telemetry.as_ref().map(|x| x.handle()),
                compatibility_mode: Default::default(),
            },
        )?;

        task_manager.spawn_essential_handle().spawn_blocking("aura", None, aura);
    }

    if !config.disable_grandpa {
        let grandpa_config = GrandpaConfig {
            gossip_duration: Duration::from_millis(1000),
            justification_generation_period: 512,
            name: Some(config.network.node_name.clone()),
            observer_enabled: false,
            keystore: Some(keystore_container.keystore()),
            local_role: config.role.clone(),
            telemetry: telemetry.as_ref().map(|x| x.handle()),
            protocol_name: sc_consensus_grandpa::protocol_standard_name(
                &client.block_hash(0).ok().flatten().expect("Genesis block exists; qed"),
                &config.chain_spec,
            ),
        };

        let grandpa_params = GrandpaParams {
            config: grandpa_config,
            link: grandpa_link,
            telemetry: telemetry.as_ref().map(|x| x.handle()),
            voting_rule: VotingRulesBuilder::default().build(),
            prometheus_registry: config.prometheus_registry().cloned(),
            shared_voter_state: SharedVoterState::empty(),
            offchain_tx_pool_factory: OffchainTransactionPoolFactory::new(transaction_pool.clone()),
            sync: sync_service.clone(),
            network: network.clone(),
            notification_service: Box::new(network.clone()), // Added notification_service
        };

        task_manager.spawn_essential_handle().spawn_blocking(
            "grandpa-voter",
            None,
            sc_consensus_grandpa::run_grandpa_voter(grandpa_params)?,
        );
    }

    Ok(task_manager)
}