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

#![warn(unused_extern_crates)]

//! Service implementation. Specialized wrapper over substrate service.

use sc_cli::Result;
use kitchensink_runtime::{self, opaque::Block, Runtime};
use sc_client_api::{Backend, BlockBackend, BlockchainEvents};
use sc_consensus::BasicQueue;
use sc_executor::NativeElseWasmExecutor;
use sc_network::NetworkService;
use sc_service::{error::Error as ServiceError, Configuration, TaskManager, RpcHandlers};
use sc_telemetry::{Telemetry, TelemetryWorker};
use sc_transaction_pool::{BasicPool, FullChainApi};
use sp_runtime::traits::Block as BlockT;
use std::sync::Arc;

pub struct ExecutorDispatch;

impl sc_executor::NativeExecutionDispatch for ExecutorDispatch {
	type ExtendHostFunctions = ();

	fn dispatch(method: &str, data: &[u8]) -> Option<Vec<u8>> {
		kitchensink_runtime::api::dispatch(method, data)
	}

	fn native_version() -> sc_executor::NativeVersion {
		kitchensink_runtime::native_version()
	}
}

/// A specialized `WasmExecutor` intended to use across substrate node.
pub type RuntimeExecutor = sc_executor::WasmExecutor<()>;

/// The full client type definition.
pub type FullClient = sc_service::TFullClient<Block, Runtime, NativeElseWasmExecutor<ExecutorDispatch>>;
pub type FullBackend = sc_service::TFullBackend<Block>;
pub type FullSelectChain = sc_consensus::LongestChain<FullBackend, Block>;
pub type TransactionPool = BasicPool<FullChainApi<FullClient, Block>, Block>;
pub type NetworkHandle = Arc<NetworkService<Block, <Block as BlockT>::Hash>>;

/// Creates a new partial node.
pub fn new_partial(
	config: &Configuration,
) -> Result<
	sc_service::PartialComponents<
		FullClient,
		FullBackend,
		FullSelectChain,
		BasicQueue<Block, FullClient>,
		TransactionPool,
		(Option<Telemetry>,),
	>,
	ServiceError,
> {
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

	let executor = NativeElseWasmExecutor::<ExecutorDispatch>::new(
		config.executor.wasm_method,
		config.executor.default_heap_pages,
		config.executor.max_runtime_instances,
		config.executor.runtime_cache_size,
	);

	let (client, backend, keystore_container, task_manager) =
		sc_service::new_full_parts::<Block, Runtime, _>(
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

	let transaction_pool = BasicPool::new_full(
		config.transaction_pool.clone().into(),
		false,
		config.prometheus_registry(),
		task_manager.spawn_essential_handle(),
		client.clone(),
	);

	let import_queue = BasicQueue::new(
		client.clone(),
		None,
		config.prometheus_registry(),
		task_manager.spawn_essential_handle(),
	);

	Ok(sc_service::PartialComponents {
		client,
		backend,
		task_manager,
		keystore_container,
		select_chain,
		import_queue,
		transaction_pool,
		other: (telemetry,),
	})
}

/// Result of [`new_full_base`].
pub struct NewFullBase {
	/// The task manager of the node.
	pub task_manager: TaskManager,
	/// The client instance of the node.
	pub client: Arc<FullClient>,
	/// The networking service of the node.
	pub network: Arc<NetworkService<Block, <Block as BlockT>::Hash>>,
	/// The syncing service of the node.
	pub sync: Arc<sc_network::SyncingService<Block>>,
	/// The transaction pool of the node.
	pub transaction_pool: Arc<TransactionPool>,
	/// The rpc handlers of the node.
	pub rpc_handlers: RpcHandlers,
}

/// Creates a full service from the configuration.
pub fn new_full_base(
	config: Configuration,
) -> Result<NewFullBase> {
	let sc_service::PartialComponents {
		client,
		backend,
		mut task_manager,
		import_queue,
		keystore_container,
		select_chain,
		transaction_pool,
		other: (mut telemetry),
	} = new_partial(&config)?;

	let client = Arc::new(client);
	let backend = Arc::new(backend);

	let (network, system_rpc_tx, network_starter) = sc_service::build_network(sc_service::BuildNetworkParams {
		config: &config,
		client: client.clone(),
		transaction_pool: transaction_pool.clone(),
		spawn_handle: task_manager.spawn_handle(),
		import_queue,
		block_announce_validator_builder: None,
		net_config: Default::default(),
		metrics: sc_network::NotificationMetrics::new(Some(config.prometheus_registry().expect("Prometheus registry must be available"))),
	})?;

	let pool = transaction_pool.clone();

	let rpc_extensions_builder = {
		let client = client.clone();
		let pool = pool.clone();
		let select_chain = select_chain.clone();
		let keystore = keystore_container.keystore();

		Box::new(move |deny_unsafe, _| {
			let deps = crate::rpc::FullDeps {
				client: client.clone(),
				pool: pool.clone(),
				select_chain: select_chain.clone(),
				deny_unsafe,
				keystore: keystore.clone(),
			};

			crate::rpc::create_full(deps)
		})
	};

	sc_service::spawn_tasks(sc_service::SpawnTasksParams {
		config: &config,
		backend: backend.clone(),
		client: client.clone(),
		keystore: keystore_container.keystore(),
		task_manager: &mut task_manager,
		transaction_pool: transaction_pool.clone(),
		rpc_builder: Box::new(|_, _| Ok(())),
		system_rpc_tx,
		telemetry: telemetry.as_mut().map(|x| x.handle()),
	})?;

	network_starter.start_network();

	Ok(NewFullBase {
		task_manager,
		client,
		network,
		sync: network_starter,
		transaction_pool,
		rpc_handlers: rpc_extensions_builder,
	})
}

/// Builds a new service for a full client.
pub fn new_full(config: Configuration) -> Result<TaskManager, ServiceError> {
	let sc_service::PartialComponents {
		client,
		backend,
		mut task_manager,
		import_queue,
		keystore_container,
		select_chain: _,
		transaction_pool,
		other: (mut telemetry),
	} = new_partial(&config)?;

	let (network, system_rpc_tx, tx_handler_controller, network_starter) =
		sc_service::build_network(sc_service::BuildNetworkParams {
			config: &config,
			client: client.clone(),
			transaction_pool: transaction_pool.clone(),
			spawn_handle: task_manager.spawn_handle(),
			import_queue,
			block_announce_validator_builder: None,
			warp_sync_config: None,
			block_relay: None,
			net_config: Default::default(),
			metrics: sc_network::NotificationMetrics::new(config.prometheus_registry()),
		})?;

	let rpc_builder = {
		let client = client.clone();
		let transaction_pool = transaction_pool.clone();

		Box::new(move |deny_unsafe, _| {
			let deps = crate::rpc::FullDeps {
				client: client.clone(),
				pool: transaction_pool.clone(),
				deny_unsafe,
			};

			crate::rpc::create_full(deps)
		})
	};

	sc_service::spawn_tasks(sc_service::SpawnTasksParams {
		network,
		client: client.clone(),
		keystore: keystore_container.keystore(),
		task_manager: &mut task_manager,
		transaction_pool: transaction_pool.clone(),
		rpc_builder,
		backend,
		system_rpc_tx,
		config: config.clone(),
		telemetry: telemetry.as_mut(),
		tx_handler_controller,
		sync_service: network_starter,
	})?;

	Ok(task_manager)
}
