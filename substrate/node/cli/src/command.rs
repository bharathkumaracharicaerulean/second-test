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

//! Command line interface implementation for the Substrate node.
//! This module handles parsing and execution of CLI commands.

use sc_cli::{
	ChainSpec as ChainSpecTrait, SharedParams, SubstrateCli,
};
use sc_service::{
	ChainSpecExtension,
	PartialComponents,
};
use std::any::{Any, TypeId};
use serde::{Serialize, Deserialize};
use kitchensink_runtime::RuntimeGenesisConfig;
use sc_chain_spec::NoExtension;
use sc_cli::RunCmd;

use crate::chain_spec;
use crate::service;

/// Sub-commands supported by the main executor.
/// Each variant represents a different CLI command that can be executed.
#[derive(Debug, clap::Subcommand)]
pub enum Subcommand {
	/// Build a chain specification.
	/// This command generates a chain specification file that can be used to initialize a new chain.
	BuildSpec(sc_cli::BuildSpecCmd),

	/// Validate blocks.
	/// This command checks if blocks in the chain are valid according to the consensus rules.
	CheckBlock(sc_cli::CheckBlockCmd),

	/// Export blocks.
	/// This command exports blocks from the chain to a file.
	ExportBlocks(sc_cli::ExportBlocksCmd),

	/// Export the state of a given block into a chain spec.
	/// This command exports the state of a specific block to create a new chain specification.
	ExportState(sc_cli::ExportStateCmd),

	/// Import blocks.
	/// This command imports blocks from a file into the chain.
	ImportBlocks(sc_cli::ImportBlocksCmd),

	/// Remove the whole chain.
	/// This command removes all chain data from the database.
	PurgeChain(sc_cli::PurgeChainCmd),

	/// Revert the chain to a previous state.
	/// This command reverts the chain to a specified block.
	Revert(sc_cli::RevertCmd),
}

/// Main CLI structure that holds all command-line arguments.
/// This includes both the subcommand and the shared parameters.
#[derive(Debug, clap::Parser)]
pub struct Cli {
	/// The subcommand to execute.
	#[clap(subcommand)]
	pub subcommand: Option<Subcommand>,

	/// The run command parameters.
	#[clap(flatten)]
	pub run: RunCmd,

	/// Shared parameters used by all commands.
	#[clap(flatten)]
	pub shared_params: SharedParams,
}

/// Implementation of the Substrate CLI trait for our CLI structure.
/// This provides basic information about the node implementation.
impl SubstrateCli for Cli {
	/// Returns the name of the implementation.
	fn impl_name() -> String {
		"Substrate Node".into()
	}

	/// Returns the version of the implementation.
	fn impl_version() -> String {
		env!("CARGO_PKG_VERSION").into()
	}

	/// Returns the description of the implementation.
	fn description() -> String {
		"Substrate Node Implementation".into()
	}

	/// Returns the author of the implementation.
	fn author() -> String {
		env!("CARGO_PKG_AUTHORS").into()
	}

	/// Returns the support URL for the implementation.
	fn support_url() -> String {
		"https://github.com/paritytech/substrate/issues/new".into()
	}

	/// Returns the copyright start year.
	fn copyright_start_year() -> i32 {
		2017
	}

	/// Returns the executable name.
	fn executable_name() -> String {
		env!("CARGO_PKG_NAME").into()
	}

	/// Loads a chain specification from the given identifier.
	/// This can be a built-in spec name or a path to a spec file.
	fn load_spec(&self, id: &str) -> Result<Box<dyn ChainSpecTrait>, String> {
		let spec = match id {
			"dev" => chain_spec::development_config()?,
			"local_testnet" => chain_spec::local_testnet_config()?,
			path => chain_spec::ChainSpec::from_json_file(path.into())?,
		};
		Ok(Box::new(spec))
	}
}

/// Static empty value used as a placeholder for type erasure.
static mut EMPTY: () = ();

/// Wrapper for genesis configuration that implements ChainSpecExtension.
#[derive(Serialize, Deserialize)]
struct GenesisConfigWrapper(serde_json::Value);

/// Implementation of ChainSpecExtension for GenesisConfigWrapper.
/// This provides type-erased access to the genesis configuration.
impl ChainSpecExtension for GenesisConfigWrapper {
	type Forks = NoExtension;

	/// Get a value of type T from the extension.
	fn get<T: 'static>(&self) -> Option<&T> {
		None
	}

	/// Get any value from the extension by type ID.
	fn get_any(&self, _type_id: TypeId) -> &(dyn Any + 'static) {
		unsafe { &EMPTY }
	}

	/// Get a mutable reference to any value from the extension by type ID.
	fn get_any_mut(&mut self, _type_id: TypeId) -> &mut (dyn Any + 'static) {
		unsafe { &mut EMPTY }
	}
}

/// Implementation of Clone for GenesisConfigWrapper.
impl Clone for GenesisConfigWrapper {
	fn clone(&self) -> Self {
		Self(self.0.clone())
	}
}

/// Wrapper for runtime genesis configuration that can be serialized.
#[derive(Serialize, Deserialize)]
struct RuntimeGenesisConfigWrapper(#[serde(skip)] RuntimeGenesisConfig);

/// Implementation of Clone for RuntimeGenesisConfigWrapper.
impl Clone for RuntimeGenesisConfigWrapper {
    fn clone(&self) -> Self {
        // Create a new instance with default values
        // In a production environment, you would want to properly clone all fields
        Self(RuntimeGenesisConfig::default())
    }
}

/// Main entry point for the CLI.
/// This function parses command line arguments and executes the appropriate command.
pub fn run() -> sc_cli::Result<()> {
	let cli = Cli::from_args();

	match &cli.subcommand {
		Some(Subcommand::BuildSpec(cmd)) => {
			let runner = cli.create_runner(cmd)?;
			runner.sync_run(|config| {
				let chain_spec = config.chain_spec.cloned_box();
				cmd.run(chain_spec, config.network)
			})
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
			runner.sync_run(|config| {
				// Ensure we don't purge custom chain specs
				let chain_spec = config.chain_spec.cloned_box();
				if chain_spec.id().starts_with("dev") || chain_spec.id().starts_with("local") {
					cmd.run(config.database)
				} else {
					Err("Only development and local testnet chains can be purged.".into())
				}
			})
		}
		Some(Subcommand::Revert(cmd)) => {
			let runner = cli.create_runner(cmd)?;
			runner.async_run(|config| {
				let PartialComponents { client, task_manager, backend, .. } =
					service::new_partial(&config)?;
				let aux_revert = Box::new(|client, _backend, blocks| {
					sc_consensus_grandpa::revert(client, blocks)?;
					Ok(())
				});
				Ok((cmd.run(client, backend, Some(aux_revert)), task_manager))
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
