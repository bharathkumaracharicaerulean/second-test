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

use sc_cli::{
	ChainSpec, CliConfiguration, DefaultConfigurationValues, ImportParams, KeystoreParams,
	NetworkParams, Result as CliResult, RuntimeVersion, SharedParams, SubstrateCli,
};
use sc_service::{
	config::{BasePath, PrometheusConfig},
	ChainSpec as ChainSpecT,
	PartialComponents,
	ImportQueue,
	ChainSpecExtension,
	Configuration,
	ChainType,
	GenericChainSpec,
};
use std::path::PathBuf;
use std::any::{Any, TypeId};
use serde::{Serialize, Deserialize};
use kitchensink_runtime::RuntimeGenesisConfig;
use sc_chain_spec::NoExtension;
use futures::TryFutureExt;
use serde_json::Value;
use sc_telemetry::TelemetryEndpoints;

use crate::chain_spec;
use crate::service;
use kitchensink_runtime::Block;

/// Sub-commands supported by the main executor.
#[derive(Debug, clap::Subcommand)]
pub enum Subcommand {
	/// Build a chain specification.
	BuildSpec(sc_cli::BuildSpecCmd),

	/// Validate blocks.
	CheckBlock(sc_cli::CheckBlockCmd),

	/// Export blocks.
	ExportBlocks(sc_cli::ExportBlocksCmd),

	/// Export the state of a given block into a chain spec.
	ExportState(sc_cli::ExportStateCmd),

	/// Import blocks.
	ImportBlocks(sc_cli::ImportBlocksCmd),

	/// Remove the whole chain.
	PurgeChain(sc_cli::PurgeChainCmd),

	/// Revert the chain to a previous state.
	Revert(sc_cli::RevertCmd),
}

#[derive(Debug, clap::Parser)]
pub struct Cli {
	#[clap(subcommand)]
	pub subcommand: Option<Subcommand>,

	#[clap(flatten)]
	pub run: sc_cli::RunCmd,

	#[clap(flatten)]
	pub shared_params: SharedParams,
}

impl SubstrateCli for Cli {
	fn impl_name() -> String {
		"Substrate Node".into()
	}

	fn impl_version() -> String {
		env!("SUBSTRATE_CLI_IMPL_VERSION").into()
	}

	fn description() -> String {
		env!("CARGO_PKG_DESCRIPTION").into()
	}

	fn author() -> String {
		env!("CARGO_PKG_AUTHORS").into()
	}

	fn support_url() -> String {
		"https://github.com/paritytech/substrate/issues/new".into()
	}

	fn copyright_start_year() -> i32 {
		2017
	}

	fn executable_name() -> String {
		"substrate".into()
	}

	fn load_spec(&self, id: &str) -> std::result::Result<Box<dyn sc_cli::ChainSpec>, String> {
		let spec = match id {
			"dev" => chain_spec::development_config(),
			"" | "local" => chain_spec::local_testnet_config(),
			path => {
				let chain_spec = chain_spec::ChainSpec::from_json_file(PathBuf::from(path))?;
				let genesis = chain_spec.as_storage_builder().build_storage().map_err(|e| format!("Error getting genesis: {}", e))?;
				let wrapped_config = serde_json::to_value(&genesis).map_err(|e| format!("Error converting genesis: {}", e))?;

				let telemetry_endpoints = chain_spec.telemetry_endpoints().clone()
					.unwrap_or_else(|| sc_telemetry::TelemetryEndpoints::new(vec![]).expect("Failed to create telemetry endpoints"));

				let chain_spec = sc_service::GenericChainSpec::builder(&[], NoExtension::default())
					.with_name(chain_spec.name())
					.with_id(chain_spec.id())
					.with_chain_type(ChainType::Local)
					.with_genesis_config(wrapped_config)
					.with_boot_nodes(chain_spec.boot_nodes().to_vec())
					.with_telemetry_endpoints(telemetry_endpoints)
					.with_protocol_id(chain_spec.protocol_id().unwrap_or(""))
					.with_properties(chain_spec.properties().clone())
					.with_extensions(NoExtension::default())
					.build()
					.map_err(|e| format!("Error building chain spec: {}", e))?;

				Ok(Box::new(chain_spec))
			}
		}?;
		Ok(spec)
	}
}

static mut EMPTY: () = ();

#[derive(Serialize, Deserialize, Clone)]
struct GenesisConfigWrapper(#[serde(skip)] RuntimeGenesisConfig);

impl ChainSpecExtension for GenesisConfigWrapper {
	type Forks = NoExtension;

	fn get<T: 'static>(&self) -> Option<&T> {
		None
	}

	fn get_any(&self, _type_id: TypeId) -> &(dyn Any + 'static) {
		unsafe { &EMPTY }
	}

	fn get_any_mut(&mut self, _type_id: TypeId) -> &mut (dyn Any + 'static) {
		unsafe { &mut EMPTY }
	}
}

impl Clone for RuntimeGenesisConfig {
	fn clone(&self) -> Self {
		Self::default()
	}
}

/// Parse and run command line arguments
pub fn run() -> sc_cli::Result<()> {
	let cli = Cli::from_args();

	match &cli.subcommand {
		Some(Subcommand::BuildSpec(cmd)) => {
			let runner = cli.create_runner(cmd)?;
			runner.sync_run(|config| cmd.run(config.chain_spec, config.network))
		}
		Some(Subcommand::CheckBlock(cmd)) => {
			let runner = cli.create_runner(cmd)?;
			runner.async_run(|config| {
				let PartialComponents { client, task_manager, import_queue, .. } =
					service::new_partial(&config)?;
				Ok((cmd.run(client, import_queue), task_manager))
			})
		}
		Some(Subcommand::ExportBlocks(cmd)) => {
			let runner = cli.create_runner(cmd)?;
			runner.async_run(|config| {
				let PartialComponents { client, task_manager, .. } = service::new_partial(&config)?;
				Ok((cmd.run(client, config.database), task_manager))
			})
		}
		Some(Subcommand::ExportState(cmd)) => {
			let runner = cli.create_runner(cmd)?;
			runner.async_run(|config| {
				let PartialComponents { client, task_manager, .. } = service::new_partial(&config)?;
				Ok((cmd.run(client, config.chain_spec), task_manager))
			})
		}
		Some(Subcommand::ImportBlocks(cmd)) => {
			let runner = cli.create_runner(cmd)?;
			runner.async_run(|config| {
				let PartialComponents { client, task_manager, import_queue, .. } =
					service::new_partial(&config)?;
				Ok((cmd.run(client, import_queue), task_manager))
			})
		}
		Some(Subcommand::PurgeChain(cmd)) => {
			let runner = cli.create_runner(cmd)?;
			runner.sync_run(|config| cmd.run(config.database))
		}
		Some(Subcommand::Revert(cmd)) => {
			let runner = cli.create_runner(cmd)?;
			runner.async_run(|config| {
				let PartialComponents { client, task_manager, backend, .. } =
					service::new_partial(&config)?;
				Ok((cmd.run(client, backend, None), task_manager))
			})
		}
		None => {
			let runner = cli.create_runner(&cli.run)?;
			runner.run_node_until_exit(|config| async move {
				service::new_full(config).await.map_err(sc_cli::Error::Service)
			})
		}
	}
}
