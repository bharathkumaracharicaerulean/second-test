// This file is part of CBC-Chain.

// Copyright (C) Caerulean Bytechains Private Limited Technologies (UK) Ltd.
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
    SubstrateCli, ChainSpec as ChainSpecTrait,
};
use sc_service::{
    PartialComponents,
};
use sc_chain_spec::{ChainType, GetExtension};
use std::any::{Any, TypeId};
use serde::{Serialize, Deserialize};
use kitchensink_runtime::RuntimeGenesisConfig;
use sc_network::config::MultiaddrWithPeerId;
use sc_telemetry::TelemetryEndpoints;
use std::collections::BTreeMap;
use sp_runtime::{Storage, BuildStorage};


use crate::service;
use crate::chain_spec;
use crate::cli::*; // Import Cli and Subcommand

/// Static empty value used as a placeholder for type erasure.
static mut EMPTY: () = ();

/// Wrapper for genesis configuration that implements ChainSpecExtension.
#[derive(Serialize, Deserialize, Clone)]
struct GenesisConfigWrapper(serde_json::Value);

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
		"https://github.com/paritytech/polkadot-sdk/issues/new".into()
	}

	fn copyright_start_year() -> i32 {
		2017
	}

	fn load_spec(&self, id: &str) -> std::result::Result<Box<dyn sc_service::ChainSpec>, String> {
		let spec = match id {
			"" =>
				return Err(
					"Please specify which chain you want to run, e.g. --dev or --chain=local"
						.into(),
				),
			"dev" => Box::new(chain_spec::development_config()),
			"local" => Box::new(chain_spec::local_testnet_config()),
			"fir" | "flaming-fir" => Box::new(chain_spec::flaming_fir_config()?),
			"staging" => Box::new(chain_spec::local_testnet_config()),
			path =>
				Box::new(chain_spec::ChainSpec::from_json_file(std::path::PathBuf::from(path))?),
		};
		Ok(spec)
	}
}


impl sc_cli::ChainSpec for GenesisConfigWrapper {
    fn id(&self) -> &str {
        "genesis"
    }

    fn name(&self) -> &str {
        "Genesis Chain"
    }

    fn chain_type(&self) -> ChainType {
        ChainType::Development
    }

    fn boot_nodes(&self) -> &[MultiaddrWithPeerId] {
        &[]
    }

    fn telemetry_endpoints(&self) -> &Option<TelemetryEndpoints> {
        &None
    }

    fn properties(&self) -> serde_json::Map<String, serde_json::Value> {
        serde_json::Map::new()
    }

    fn protocol_id(&self) -> Option<&str> {
        None
    }

    fn fork_id(&self) -> Option<&str> {
        None
    }

    fn extensions(&self) -> &dyn GetExtension {
        self
    }

    fn extensions_mut(&mut self) -> &mut dyn GetExtension {
        self
    }

    fn add_boot_node(&mut self, _addr: MultiaddrWithPeerId) {}

    fn as_json(&self, _pretty: bool) -> Result<String, String> {
        serde_json::to_string(&self.0).map_err(|e| e.to_string())
    }

    fn as_storage_builder(&self) -> &dyn BuildStorage {
        self
    }

    fn cloned_box(&self) -> Box<dyn sc_cli::ChainSpec + 'static> {
        Box::new(self.clone())
    }

    fn set_storage(&mut self, _storage: Storage) {}

    fn code_substitutes(&self) -> BTreeMap<String, Vec<u8>> {
        BTreeMap::new()
    }
}

impl BuildStorage for GenesisConfigWrapper {
    fn build_storage(&self) -> Result<Storage, String> {
        Ok(Storage::default())
    }

    fn assimilate_storage(&self, _storage: &mut Storage) -> Result<(), String> {
        Ok(())
    }
}

impl GetExtension for GenesisConfigWrapper {
    fn get_any(&self, _type_id: TypeId) -> &(dyn Any + 'static) {
        unsafe { &EMPTY }
    }

    fn get_any_mut(&mut self, _type_id: TypeId) -> &mut (dyn Any + 'static) {
        unsafe { &mut EMPTY }
    }
}

/// Wrapper for runtime genesis configuration that can be serialized.
#[derive(Serialize, Deserialize, Clone)]
struct RuntimeGenesisConfigWrapper(#[serde(skip)] RuntimeGenesisConfig);

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