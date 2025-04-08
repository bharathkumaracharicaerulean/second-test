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

use std::result::Result;
use kitchensink_runtime::{self, opaque::Block, RuntimeApi};
use sc_executor::NativeElseWasmExecutor;
use sc_service::{error::Error as ServiceError, Configuration, PartialComponents, TFullBackend, TFullClient, TaskManager};
use sc_consensus::import_queue::{ImportQueue, ImportQueueService};
use std::sync::Arc;
use sc_transaction_pool::{BasicPool, Options, PoolLimit, FullChainApi};
use std::time::Duration;
use sc_network::config::NetworkConfiguration;
use sp_core::traits::SpawnNamed;

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

/// The full client type definition.
pub type FullClient = TFullClient<Block, RuntimeApi, NativeElseWasmExecutor<ExecutorDispatch>>;
pub type FullBackend = TFullBackend<Block>;
pub type FullSelectChain = sc_consensus::LongestChain<FullBackend, Block>;
pub type Pool = BasicPool<FullChainApi<FullClient, Block>, Block>;

/// Creates a new partial node.
pub fn new_partial(
	config: &Configuration,
) -> Result<PartialComponents<FullClient, FullBackend, FullSelectChain, Box<dyn ImportQueue<Block>>, Pool, ()>, ServiceError> {
	let executor = NativeElseWasmExecutor::<ExecutorDispatch>::new(
		config.executor.wasm_method,
		config.executor.default_heap_pages,
		config.executor.max_runtime_instances,
		config.executor.runtime_cache_size,
	);

	let (client, backend, keystore_container, task_manager) =
		sc_service::new_full_parts::<Block, RuntimeApi, _>(
			config,
			Default::default(),
			executor,
		)?;
	let client = Arc::new(client);

	let select_chain = sc_consensus::LongestChain::new(backend.clone());

	let import_queue = Box::new(ImportQueueService::new(
		Box::new(client.clone()),
		client.clone(),
		config.prometheus_registry(),
	));

	let transaction_pool = BasicPool::new_full(
		Options {
			ready: PoolLimit {
				count: 8192,
				total_bytes: 20 * 1024 * 1024,
			},
			future: PoolLimit {
				count: 8192,
				total_bytes: 20 * 1024 * 1024,
			},
			reject_future_transactions: false,
			ban_time: Duration::from_secs(60 * 60), // 1 hour
		},
		config.role.is_authority().into(),
		config.prometheus_registry(),
		task_manager.spawn_essential_handle(),
		client.clone(),
	);

	Ok(PartialComponents {
		client,
		backend,
		task_manager,
		import_queue,
		keystore_container,
		select_chain,
		transaction_pool: Arc::new(transaction_pool),
		other: (),
	})
}

/// Creates a full service from the configuration.
pub fn new_full(config: Configuration) -> Result<TaskManager, ServiceError> {
	let sc_service::PartialComponents {
		client,
		backend,
		mut task_manager,
		import_queue,
		mut keystore_container,
		select_chain,
		transaction_pool,
		other: (),
	} = new_partial(&config)?;

	let net_config = NetworkConfiguration::new();
	let (network, _, _, network_starter) =
		sc_service::build_network(sc_service::BuildNetworkParams {
			config: &config,
			client: client.clone(),
			transaction_pool: transaction_pool.clone(),
			spawn_handle: task_manager.spawn_handle(),
			import_queue: Some(import_queue),
			block_announce_validator_builder: None,
			warp_sync_config: None,
			metrics: Default::default(),
			net_config,
			block_relay: Default::default(),
		})?;

	sc_service::spawn_tasks(sc_service::SpawnTasksParams {
		network: network.clone(),
		client: client.clone(),
		keystore: keystore_container.keystore(),
		task_manager: &mut task_manager,
		transaction_pool: transaction_pool.clone(),
		rpc_builder: Box::new(|_| Ok(())),
		sync_service: network.clone(),
		telemetry: None,
		backend,
		config,
		system_rpc_tx: Default::default(),
	})?;

	network_starter.start();

	Ok(task_manager)
}
