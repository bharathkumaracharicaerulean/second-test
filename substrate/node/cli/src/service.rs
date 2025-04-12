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

//! Service implementation. Specialized wrapper over substrate service.

use std::result::Result;
use std::sync::Arc;
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
use sc_network::{
	config::FullNetworkConfiguration,
};
use sc_network_sync::SyncingService;
use std::sync::atomic::{AtomicUsize, AtomicBool};
use sc_service::build_network;

/// The full client type definition.
pub type FullClient = sc_service::TFullClient<Block, RuntimeApi, WasmExecutor>;
type FullBackend = sc_service::TFullBackend<Block>;
type FullSelectChain = sc_consensus::LongestChain<FullBackend, Block>;
type TransactionPool = sc_transaction_pool::BasicPool<FullChainApi<FullClient, Block>, Block>;

/// Creates a new partial service with all the necessary components.
/// This includes telemetry, executor, client, backend, and various consensus components.
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
	// Initialize telemetry if endpoints are provided
	let telemetry = config.telemetry_endpoints.clone()
		.filter(|x| !x.is_empty())
		.map(|endpoints| -> Result<_, sc_telemetry::Error> {
			let worker = TelemetryWorker::new(16)?;
			let telemetry = worker.handle().new_telemetry(endpoints);
			Ok((worker, telemetry))
		})
		.transpose()?;

	// Configure and build the WASM executor
	let executor = WasmExecutor::builder()
		.with_execution_method(config.executor.wasm_method)
		.with_max_runtime_instances(config.executor.max_runtime_instances)
		.with_runtime_cache_size(config.executor.runtime_cache_size)
		.build();

	// Create the core service components
	let (client, backend, keystore_container, task_manager) =
		sc_service::new_full_parts::<Block, RuntimeApi, _>(
			config,
			telemetry.as_ref().map(|(_, telemetry)| telemetry.handle()),
			executor,
		)?;

	let client = Arc::new(client);

	// Spawn telemetry worker if enabled
	let telemetry = telemetry.map(|(worker, telemetry)| {
		task_manager.spawn_handle().spawn("telemetry", None, worker.run());
		telemetry
	});

	// Initialize the chain selection mechanism
	let select_chain = sc_consensus::LongestChain::new(backend.clone());

	// Create the transaction pool
	let transaction_pool = Arc::new(sc_transaction_pool::BasicPool::new_full(
		sc_transaction_pool::Options::default(),
		config.role.is_authority().into(),
		config.prometheus_registry(),
		task_manager.spawn_essential_handle(),
		client.clone(),
	));

	// Set up Grandpa consensus components
	let (grandpa_block_import, grandpa_link) = sc_consensus_grandpa::block_import(
		client.clone(),
		0u32,
		&(client.clone() as Arc<_>),
		select_chain.clone(),
		telemetry.as_ref().map(|x| x.handle()),
	)?;

	// Get the slot duration for Aura consensus
	let slot_duration = sc_consensus_aura::slot_duration(&*client)?;

	// Create the import queue for Aura consensus
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
			check_for_equivocation: Default::default(),
			telemetry: telemetry.as_ref().map(|x| x.handle()),
			compatibility_mode: Default::default(),
		},
	)?;

	// Return all the partial components
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
/// This includes network, consensus, and various other services.
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

	// Configure the network
	let mut net_config = FullNetworkConfiguration::new(&config.network, config.prometheus_registry().cloned());

	// Build the network service
	let (network, system_rpc_tx, tx_handler_controller, network_starter) = build_network(sc_service::BuildNetworkParams {
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

	// Create the sync service
	let sync_service = {
		let (tx, rx) = sc_utils::mpsc::tracing_unbounded("sync-service", 100_000);
		let counter = Arc::new(AtomicUsize::new(0));
		let is_major_syncing = Arc::new(AtomicBool::new(false));
		let sync = SyncingService::new(
			tx,
			counter,
			is_major_syncing,
		);
		Arc::new(sync)
	};

	// Extract configuration parameters
	let role = config.role.clone();
	let force_authoring = config.force_authoring;
	let backoff_authoring_blocks = Some(BackoffAuthoringOnFinalizedHeadLagging::default());
	let name = config.network.node_name.clone();
	let enable_grandpa = !config.disable_grandpa;
	let prometheus_registry = config.prometheus_registry().cloned();

	// Initialize block authoring if this node is an authority
	if role.is_authority() {
		// Create the block proposer
		let proposer = sc_basic_authorship::ProposerFactory::new(
			task_manager.spawn_handle(),
			client.clone(),
			transaction_pool.clone(),
			prometheus_registry.as_ref(),
			telemetry.as_ref().map(|x| x.handle()),
		);

		// Get the slot duration for Aura
		let slot_duration = sc_consensus_aura::slot_duration(&*client)?;

		// Start the Aura consensus mechanism
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
				force_authoring,
				backoff_authoring_blocks,
				keystore: keystore_container.keystore(),
				sync_oracle: sync_service.clone(),
				justification_sync_link: sync_service.clone(),
				block_proposal_slot_portion: SlotProportion::new(2f32 / 3f32),
				max_block_proposal_slot_portion: None,
				telemetry: telemetry.as_ref().map(|x| x.handle()),
				compatibility_mode: Default::default(),
			},
		)?;

		// Spawn the Aura service
		task_manager.spawn_essential_handle().spawn_blocking(
			"aura",
			Some("block-authoring"),
			aura,
		);
	}

	// Initialize Grandpa consensus if enabled
	if enable_grandpa {
		// Configure Grandpa
		let grandpa_config = GrandpaConfig {
			gossip_duration: Duration::from_millis(1000),
			justification_generation_period: 512,
			name: Some(name),
			observer_enabled: false,
			keystore: Some(keystore_container.keystore()),
			local_role: role,
			telemetry: telemetry.as_ref().map(|x| x.handle()),
			protocol_name: sc_consensus_grandpa::protocol_standard_name(
				&client.block_hash(0).ok().flatten().expect("Genesis block exists; qed"),
				&config.chain_spec,
			),
		};

		// Set up Grandpa parameters
		let grandpa_params = GrandpaParams {
			config: grandpa_config,
			link: grandpa_link,
			network: network.clone(),
			notification_service: Box::new(sync_service.clone()),
			telemetry: telemetry.as_ref().map(|x| x.handle()),
			voting_rule: VotingRulesBuilder::default().build(),
			prometheus_registry,
			shared_voter_state: SharedVoterState::empty(),
			offchain_tx_pool_factory: OffchainTransactionPoolFactory::new(transaction_pool.clone()),
			sync: sync_service.clone(),
		};

		// Spawn the Grandpa voter service
		task_manager.spawn_essential_handle().spawn_blocking(
			"grandpa-voter",
			None,
			sc_consensus_grandpa::run_grandpa_voter(grandpa_params)?,
		);
	}

	// Start the network
	network_starter.start();

	// Return the task manager
	Ok(task_manager)
}
