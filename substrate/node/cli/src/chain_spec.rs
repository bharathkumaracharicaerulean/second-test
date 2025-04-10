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

//! Substrate chain configurations.

use std::result::Result;
use std::borrow::Cow;
use sc_service::{ChainType, ChainSpecExtension};
use sc_chain_spec::{Group, Fork};
use sc_executor::HostFunctions;
use serde::{Serialize, Deserialize};
use sp_core::{sr25519, Pair, Public};
use sp_runtime::traits::{IdentifyAccount, Verify};
use kitchensink_runtime::{
	AccountId, AuraConfig, BalancesConfig, RuntimeGenesisConfig, GrandpaConfig,
	SudoConfig, SystemConfig, WASM_BINARY, Signature,
};
use std::any::{Any, TypeId};
use serde_json::json;
use hex;
use sp_consensus_aura::sr25519::AuthorityId as AuraId;
use sp_consensus_grandpa::AuthorityId as GrandpaId;
use sc_executor::sp_wasm_interface::{HostFunctionRegistry, Function as WasmFunction};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Extensions;

impl ChainSpecExtension for Extensions {
	type Forks = Option<()>;

	fn get<T: 'static>(&self) -> Option<&T> {
		(self as &dyn Any).downcast_ref()
	}

	fn get_any(&self, _: TypeId) -> &dyn Any {
		self as &dyn Any
	}

	fn get_any_mut(&mut self, _: TypeId) -> &mut dyn Any {
		self as &mut dyn Any
	}
}

impl Group for Extensions {
	type Fork = Extensions;

	fn to_fork(self) -> Self::Fork {
		self
	}
}

impl Fork for Extensions {
	type Base = Self;

	fn combine_with(&mut self, other: Self) {
		// No-op, as Extensions is empty
	}

	fn to_base(self) -> Option<Self::Base> {
		Some(self)
	}
}

impl HostFunctions for Extensions {
	fn host_functions() -> Vec<&'static (dyn WasmFunction + 'static)> {
		Vec::new()
	}

	fn register_static<T>(_: &mut T) -> Result<(), <T as HostFunctionRegistry>::Error> 
	where 
		T: HostFunctionRegistry 
	{
		Ok(())
	}
}

/// Specialized `ChainSpec` for the normal parachain runtime.
pub type ChainSpec = sc_service::GenericChainSpec<RuntimeGenesisConfig, Extensions>;

type AccountPublic = <Signature as Verify>::Signer;

/// The chain specification option. This is expected to come in from the CLI and
/// is little more than one of a number of alternatives which can easily be converted
/// from a string (`--chain=...`) into a `ChainSpec`.
#[derive(Clone, Debug)]
pub enum Alternative {
	/// Whatever the current runtime is, with just Alice as an auth.
	Development,
	/// Whatever the current runtime is, with simple Alice/Bob auths.
	LocalTestnet,
}

impl From<&str> for Alternative {
	fn from(s: &str) -> Self {
		match s {
			"dev" | "development" => Alternative::Development,
			"local" | "local_testnet" => Alternative::LocalTestnet,
			_ => panic!("Invalid chain spec name"),
		}
	}
}

/// Generate a crypto pair from seed.
pub fn get_from_seed<TPublic: Public>(seed: &str) -> <TPublic::Pair as Pair>::Public {
	TPublic::Pair::from_string(&format!("//{}", seed), None)
		.expect("static values are valid; qed")
		.public()
}

/// Generate an account ID from seed.
pub fn get_account_id_from_seed<TPublic: Public>(seed: &str) -> AccountId
where
	AccountPublic: From<<TPublic::Pair as Pair>::Public>,
{
	AccountPublic::from(get_from_seed::<TPublic>(seed)).into_account()
}

/// Helper function to generate authority keys
pub fn authority_keys_from_seed(s: &str) -> (AuraId, GrandpaId) {
	(
		get_from_seed::<AuraId>(s),
		get_from_seed::<GrandpaId>(s),
	)
}

/// Helper function to create a GenesisConfig for testing
pub fn testnet_genesis(
	wasm_binary: &[u8],
	initial_authorities: Vec<(AuraId, GrandpaId)>,
	root_key: AccountId,
	endowed_accounts: Vec<AccountId>,
	_enable_println: bool,
) -> RuntimeGenesisConfig {
	RuntimeGenesisConfig {
		system: SystemConfig {
			_config: Default::default(),
		},
		balances: BalancesConfig {
			balances: endowed_accounts.iter().cloned().map(|k| (k, 1 << 60)).collect(),
			dev_accounts: Some((0, 1 << 60, None)),
		},
		aura: AuraConfig {
			authorities: initial_authorities.iter().map(|x| (x.0.clone())).collect(),
		},
		grandpa: GrandpaConfig {
			_config: Default::default(),
			authorities: initial_authorities.iter().map(|x| (x.1.clone(), 1)).collect(),
		},
		sudo: SudoConfig {
			key: Some(root_key),
		},
	}
}

/// Development config (single validator Alice)
pub fn development_config() -> Result<ChainSpec, String> {
	let wasm_binary = WASM_BINARY.ok_or_else(|| "Development wasm binary not available".to_string())?;

	let alice = get_account_id_from_seed::<sr25519::Public>("Alice");
	let alice_authority = authority_keys_from_seed("Alice");

	let genesis = json!({
		"name": "Development",
		"id": "dev",
		"chainType": "Development",
		"genesis": {
			"runtime": {
				"system": {
					"code": hex::encode(wasm_binary)
				},
				"balances": {
					"balances": [
						[alice.to_string(), 1u64 << 60]
					]
				},
				"aura": {
					"authorities": [alice_authority.0.to_string()]
				},
				"grandpa": {
					"authorities": [[alice_authority.1.to_string(), 1u64]]
				},
				"sudo": {
					"key": alice.to_string()
				}
			}
		},
		"bootNodes": [],
		"telemetryEndpoints": null,
		"protocolId": null,
		"properties": null,
		"consensusEngine": null,
		"codeSubstitutes": {}
	});

	let json_bytes = serde_json::to_vec(&genesis).map_err(|e| e.to_string())?;
	ChainSpec::from_json_bytes(Cow::Owned(json_bytes))
}

/// Helper function to create a GenesisConfig for local testnet
pub fn local_testnet_config() -> Result<ChainSpec, String> {
	let wasm_binary = WASM_BINARY.ok_or_else(|| "Development wasm binary not available".to_string())?;

	let alice = get_account_id_from_seed::<sr25519::Public>("Alice");
	let bob = get_account_id_from_seed::<sr25519::Public>("Bob");
	let charlie = get_account_id_from_seed::<sr25519::Public>("Charlie");

	let alice_authority = authority_keys_from_seed("Alice");
	let bob_authority = authority_keys_from_seed("Bob");

	let genesis = json!({
		"name": "Local Testnet",
		"id": "local_testnet",
		"chainType": "Local",
		"genesis": {
			"runtime": {
				"system": {
					"code": hex::encode(wasm_binary)
				},
				"balances": {
					"balances": [
						[alice.to_string(), 1u64 << 60],
						[bob.to_string(), 1u64 << 60],
						[charlie.to_string(), 1u64 << 60]
					]
				},
				"aura": {
					"authorities": [
						alice_authority.0.to_string(),
						bob_authority.0.to_string()
					]
				},
				"grandpa": {
					"authorities": [
						[alice_authority.1.to_string(), 1u64],
						[bob_authority.1.to_string(), 1u64]
					]
				},
				"sudo": {
					"key": alice.to_string()
				}
			}
		},
		"bootNodes": [],
		"telemetryEndpoints": null,
		"protocolId": null,
		"properties": null,
		"consensusEngine": null,
		"codeSubstitutes": {}
	});

	let json_bytes = serde_json::to_vec(&genesis).map_err(|e| e.to_string())?;
	ChainSpec::from_json_bytes(Cow::Owned(json_bytes))
}

/// Helper function to load chain spec from the environment variable
pub fn load_spec(id: &str) -> Result<ChainSpec, String> {
	match Alternative::from(id) {
		Alternative::Development => development_config(),
		Alternative::LocalTestnet => local_testnet_config(),
	}
}

/// Get a chain config from a spec setting.
pub fn get_chain_spec(spec: Alternative) -> Result<ChainSpec, String> {
	match spec {
		Alternative::Development => development_config(),
		Alternative::LocalTestnet => local_testnet_config(),
	}
}
