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
use kitchensink_runtime::{self, opaque::Block, RuntimeApi};
use sc_client_api::BlockBackend;
use sc_executor::NativeElseWasmExecutor;
use sc_service::{error::Error as ServiceError, Configuration, PartialComponents, TFullBackend, TFullClient, TaskManager};
use sp_consensus::BlockImport;
use sp_runtime::traits::BlakeTwo256;
use std::sync::Arc;

pub struct ExecutorDispatch;

impl sc_executor::NativeExecutionDispatch for ExecutorDispatch {
	type ExtendHostFunctions = frame_benchmarking::benchmarking::HostFunctions;

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
pub type FullPool = sc_transaction_pool::FullPool<Block, FullClient>;

/// Creates a new partial node.
pub fn new_partial(
	config: &Configuration,
) -> Result<PartialComponents<FullClient, FullBackend, FullSelectChain, sp_consensus::import_queue::BasicQueue<Block, FullBackend>, FullPool>, ServiceError> {
	let inherent_data_providers = sp_inherents::InherentDataProviders::new();

	let (client, backend, keystore_container, task_manager) =
		sc_service::new_full_parts::<Block, RuntimeApi, _>(
			config,
			inherent_data_providers.clone(),
		)?;
	let client = Arc::new(client);

	let select_chain = sc_consensus::LongestChain::new(backend.clone());

	let transaction_pool = sc_transaction_pool::BasicPool::new_full(
		config.transaction_pool.clone(),
		config.role.is_authority().into(),
		config.prometheus_registry(),
		task_manager.spawn_handle(),
		client.clone(),
	);

	let import_queue = sc_consensus_babe::import_queue(
		inherent_data_providers.clone(),
		&task_manager.spawn_handle(),
		client.clone(),
		select_chain.clone(),
		move |_, _| async move { Ok(()) },
		&task_manager.spawn_handle(),
		config.prometheus_registry(),
		sp_consensus::NeverCanAuthor,
	)?;

	Ok(PartialComponents {
		client,
		backend,
		task_manager,
		import_queue,
		keystore_container,
		select_chain,
		transaction_pool,
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
		keystore_container,
		select_chain,
		transaction_pool,
		..
	} = new_partial(&config)?;

	let (network, system_rpc_tx, tx_handler_controller, sync_service) = sc_service::build_network(
		sc_service::BuildNetworkParams {
			config: &config,
			client: client.clone(),
			transaction_pool: transaction_pool.clone(),
			spawn_handle: task_manager.spawn_handle(),
			import_queue,
			block_announce_validator_builder: None,
			warp_sync: None,
		},
	)?;

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
		config,
		backend,
		client,
		keystore_container,
		task_manager: &mut task_manager,
		transaction_pool,
		network,
		rpc_builder,
		system_rpc_tx,
		tx_handler_controller,
		sync_service,
	})?;

	Ok(task_manager)
}
