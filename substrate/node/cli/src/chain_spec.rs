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
//! This module provides the chain specification implementation for the Substrate node.
//! It includes predefined configurations for development and local testnet environments.

use std::result::Result;
use sc_service::{
	ChainSpecExtension,
};
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
use sp_consensus_aura::sr25519::AuthorityId as AuraId;
use sp_consensus_grandpa::AuthorityId as GrandpaId;
use sc_executor::sp_wasm_interface::{HostFunctionRegistry, Function as WasmFunction};

/// Empty extensions structure for the chain specification.
/// This is used when no additional extensions are needed.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Extensions;

impl Default for Extensions {
	fn default() -> Self {
		Self
	}
}

/// Implementation of ChainSpecExtension for the empty Extensions structure.
/// This provides type-erased access to the extensions.
impl ChainSpecExtension for Extensions {
	type Forks = Option<()>;

	/// Get a value of type T from the extension.
	fn get<T: 'static>(&self) -> Option<&T> {
		(self as &dyn Any).downcast_ref()
	}

	/// Get any value from the extension by type ID.
	fn get_any(&self, _: TypeId) -> &dyn Any {
		self as &dyn Any
	}

	/// Get a mutable reference to any value from the extension by type ID.
	fn get_any_mut(&mut self, _: TypeId) -> &mut dyn Any {
		self as &mut dyn Any
	}
}

/// Implementation of Group trait for Extensions.
/// This allows the extensions to be used in a group context.
impl Group for Extensions {
	type Fork = Extensions;

	/// Convert the extension to a fork.
	fn to_fork(self) -> Self::Fork {
		self
	}
}

/// Implementation of Fork trait for Extensions.
/// This allows the extensions to be used in a fork context.
impl Fork for Extensions {
	type Base = Self;

	/// Combine this extension with another.
	fn combine_with(&mut self, _other: Self) {
		// No-op, as Extensions is empty
	}

	/// Convert the extension to its base type.
	fn to_base(self) -> Option<Self::Base> {
		Some(self)
	}
}

/// Implementation of HostFunctions for Extensions.
/// This provides WASM host functions for the chain specification.
impl HostFunctions for Extensions {
	/// Get the list of host functions.
	fn host_functions() -> Vec<&'static (dyn WasmFunction + 'static)> {
		Vec::new()
	}

	/// Register static host functions.
	fn register_static<T>(_: &mut T) -> Result<(), <T as HostFunctionRegistry>::Error> 
	where 
		T: HostFunctionRegistry 
	{
		Ok(())
	}
}

/// Specialized `ChainSpec` for the normal parachain runtime.
pub type ChainSpec = sc_service::GenericChainSpec<RuntimeGenesisConfig, Extensions>;

/// Type alias for the account public key.
type AccountPublic = <Signature as Verify>::Signer;

/// The chain specification option.
/// This enum represents different chain configurations that can be used.
#[derive(Clone, Debug)]
pub enum Alternative {
	/// Development configuration with a single validator (Alice).
	Development,
	/// Local testnet configuration with multiple validators (Alice/Bob).
	LocalTestnet,
}

/// Convert a string to an Alternative chain spec.
impl From<&str> for Alternative {
	fn from(s: &str) -> Self {
		match s {
			"dev" | "development" => Alternative::Development,
			"local" | "local_testnet" => Alternative::LocalTestnet,
			_ => panic!("Invalid chain spec name"),
		}
	}
}

/// Generate a crypto pair from a seed string.
/// This is used to create consistent keys for testing.
pub fn get_from_seed<TPublic: Public>(seed: &str) -> <TPublic::Pair as Pair>::Public {
	TPublic::Pair::from_string(&format!("//{}", seed), None)
		.expect("static values are valid; qed")
		.public()
}

/// Generate an account ID from a seed string.
/// This is used to create consistent account IDs for testing.
pub fn get_account_id_from_seed<TPublic: Public>(seed: &str) -> AccountId
where
	AccountPublic: From<<TPublic::Pair as Pair>::Public>,
{
	AccountPublic::from(get_from_seed::<TPublic>(seed)).into_account()
}

/// Generate authority keys (Aura and Grandpa) from a seed string.
/// This is used to create consistent authority keys for testing.
pub fn authority_keys_from_seed(s: &str) -> (AuraId, GrandpaId) {
	(
		get_from_seed::<AuraId>(s),
		get_from_seed::<GrandpaId>(s),
	)
}

/// Create a genesis configuration for a testnet.
/// This sets up the initial state of the chain with the given parameters.
pub fn testnet_genesis(
	_wasm_binary: &[u8],
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

/// Create a development chain specification.
pub fn development_config() -> Result<ChainSpec, String> {
	let wasm_binary = WASM_BINARY.ok_or_else(|| "Development wasm binary not available".to_string())?;

	let genesis = testnet_genesis(
		wasm_binary,
		// Initial authorities
		vec![authority_keys_from_seed("Alice")],
		// Root key
		get_account_id_from_seed::<sr25519::Public>("Alice"),
		// Endowed accounts
		vec![
			get_account_id_from_seed::<sr25519::Public>("Alice"),
			get_account_id_from_seed::<sr25519::Public>("Bob"),
			get_account_id_from_seed::<sr25519::Public>("Alice//stash"),
			get_account_id_from_seed::<sr25519::Public>("Bob//stash"),
		],
		true,
	);

	let json = serde_json::to_string(&genesis)
		.map_err(|e| format!("Error serializing genesis config: {}", e))?;
	
	let chain_spec = ChainSpec::from_json_bytes(json.into_bytes())?;
	Ok(chain_spec)
}

/// Create a local testnet chain specification.
pub fn local_testnet_config() -> Result<ChainSpec, String> {
	let wasm_binary = WASM_BINARY.ok_or_else(|| "Development wasm binary not available".to_string())?;

	let genesis = testnet_genesis(
		wasm_binary,
		// Initial authorities
		vec![
			authority_keys_from_seed("Alice"),
			authority_keys_from_seed("Bob"),
		],
		// Root key
		get_account_id_from_seed::<sr25519::Public>("Alice"),
		// Endowed accounts
		vec![
			get_account_id_from_seed::<sr25519::Public>("Alice"),
			get_account_id_from_seed::<sr25519::Public>("Bob"),
			get_account_id_from_seed::<sr25519::Public>("Charlie"),
			get_account_id_from_seed::<sr25519::Public>("Dave"),
			get_account_id_from_seed::<sr25519::Public>("Eve"),
			get_account_id_from_seed::<sr25519::Public>("Ferdie"),
			get_account_id_from_seed::<sr25519::Public>("Alice//stash"),
			get_account_id_from_seed::<sr25519::Public>("Bob//stash"),
			get_account_id_from_seed::<sr25519::Public>("Charlie//stash"),
			get_account_id_from_seed::<sr25519::Public>("Dave//stash"),
			get_account_id_from_seed::<sr25519::Public>("Eve//stash"),
			get_account_id_from_seed::<sr25519::Public>("Ferdie//stash"),
		],
		true,
	);

	let json = serde_json::to_string(&genesis)
		.map_err(|e| format!("Error serializing genesis config: {}", e))?;
	
	let chain_spec = ChainSpec::from_json_bytes(json.into_bytes())?;
	Ok(chain_spec)
}

/// Load a chain specification from an identifier.
/// This can be a built-in spec name or a path to a spec file.
pub fn load_spec(id: &str) -> Result<ChainSpec, String> {
	match Alternative::from(id) {
		Alternative::Development => development_config(),
		Alternative::LocalTestnet => local_testnet_config(),
	}
}

/// Get a chain specification from an Alternative.
/// This is a helper function to convert an Alternative to a ChainSpec.
pub fn get_chain_spec(spec: Alternative) -> Result<ChainSpec, String> {
	match spec {
		Alternative::Development => development_config(),
		Alternative::LocalTestnet => local_testnet_config(),
	}
}
