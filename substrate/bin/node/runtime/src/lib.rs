// This file is part of Substrate.

// Copyright (C) Parity Technologies (UK) Ltd.
// SPDX-License-Identifier: Apache-2.0

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// 	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! The Substrate runtime. This can be compiled with `#[no_std]`, ready for Wasm.

#![cfg_attr(not(feature = "std"), no_std)]
// `construct_runtime!` does a lot of recursion and requires us to increase the limit to 256.
#![recursion_limit = "256"]

// Make the WASM binary available.
#[cfg(feature = "std")]
include!(concat!(env!("OUT_DIR"), "/wasm_binary.rs"));

pub mod constants;
pub mod impls;

use frame_support::{
	construct_runtime, parameter_types,
	traits::{
		ConstU128, ConstU32, ConstU64, Everything,
	},
	weights::{
		constants::WEIGHT_REF_TIME_PER_SECOND, Weight,
	},
};
use sp_runtime::{
	generic, impl_opaque_keys,
	traits::{BlakeTwo256, IdentifyAccount, Verify, IdentityLookup},
	MultiSignature,
};
use sp_std::prelude::*;
use sp_std::borrow::Cow;
#[cfg(feature = "std")]
use sp_version::NativeVersion;
use sp_version::RuntimeVersion;

pub use frame_support::weights::constants::RocksDbWeight;
pub use sp_runtime::Permill;

/// An index to a block.
pub type BlockNumber = u32;

/// Alias to 512-bit hash when used in the context of a transaction signature on the chain.
pub type Signature = MultiSignature;

/// Some way of identifying an account on the chain. We intentionally make it equivalent
/// to the public key of our transaction signing scheme.
pub type AccountId = <<Signature as Verify>::Signer as IdentifyAccount>::AccountId;

/// Balance of an account.
pub type Balance = u128;

/// Index of a transaction in the chain.
pub type Nonce = u32;

/// A hash of some data used by the chain.
pub type Hash = sp_core::H256;

/// The block type used by the runtime.
pub type Block = generic::Block<Header, UncheckedExtrinsic>;

/// The header type used by the runtime.
pub type Header = generic::Header<BlockNumber, BlakeTwo256>;

/// The unchecked extrinsic type used by the runtime.
pub type UncheckedExtrinsic = generic::UncheckedExtrinsic<AccountId, RuntimeCall, Signature, ()>;

/// Import the template pallet.


/// The type for looking up accounts. We don't expect more than 4 billion of them, but you
/// never know...
pub type AccountIndex = u32;

/// Opaque types. These are used by the CLI to instantiate machinery that don't need to know
/// the specifics of the runtime. They can then be made to be agnostic over specific formats
/// of data like extrinsics, allowing for them to continue syncing the network through upgrades
/// to even the core data structures.
pub mod opaque {
	use super::*;

	pub use sp_runtime::OpaqueExtrinsic as UncheckedExtrinsic;

	/// Opaque block header type.
	pub type Header = generic::Header<BlockNumber, BlakeTwo256>;
	/// Opaque block type.
	pub type Block = generic::Block<Header, UncheckedExtrinsic>;
	/// Opaque block identifier type.
	pub type BlockId = generic::BlockId<Block>;
}

pub const VERSION: RuntimeVersion = RuntimeVersion {
	spec_name: Cow::Borrowed("minimal"),
	impl_name: Cow::Borrowed("minimal"),
	authoring_version: 1,
	spec_version: 1,
	impl_version: 1,
	apis: Cow::Borrowed(&[]),
	transaction_version: 1,
	system_version: 1,
};

pub const MILLISECS_PER_BLOCK: u64 = 6000;

pub const SLOT_DURATION: u64 = MILLISECS_PER_BLOCK;

// Time is measured by number of blocks.
pub const MINUTES: BlockNumber = 60_000 / (MILLISECS_PER_BLOCK as BlockNumber);
pub const HOURS: BlockNumber = MINUTES * 60;
pub const DAYS: BlockNumber = HOURS * 24;

/// The version information used to identify this runtime when compiled natively.
#[cfg(feature = "std")]
pub fn native_version() -> NativeVersion {
	NativeVersion {
		runtime_version: VERSION,
		can_author_with: Default::default(),
	}
}

parameter_types! {
	pub const BlockHashCount: BlockNumber = 2400;
	/// We allow for 2 seconds of compute with a 6 second average block time.
	pub const MaximumBlockWeight: Weight = Weight::from_parts(2u64 * WEIGHT_REF_TIME_PER_SECOND, u64::MAX);
	pub const MaximumBlockLength: u32 = 5 * 1024 * 1024;
	pub const Version: RuntimeVersion = VERSION;
	pub const SS58Prefix: u8 = 42;
}

impl frame_system::Config for Runtime {
	/// The basic call filter to use in dispatchable.
	type BaseCallFilter = Everything;
	/// Block & extrinsics weights: base values and limits.
	type BlockWeights = ();
	type BlockLength = ();
	/// The identifier used to distinguish between accounts.
	type AccountId = AccountId;
	/// The aggregated dispatch type that is available for extrinsics.
	type RuntimeCall = RuntimeCall;
	/// The lookup mechanism to get account ID from whatever is passed in dispatchers.
	type Lookup = IdentityLookup<Self::AccountId>;
	/// The index type for storing how many extrinsics an account has signed.
	type Nonce = Nonce;
	/// The index type for blocks.
	type Block = Block;
	/// The type for hashing blocks and tries.
	type Hash = Hash;
	/// The hashing algorithm used.
	type Hashing = BlakeTwo256;
	/// The ubiquitous event type.
	type RuntimeEvent = RuntimeEvent;
	/// The ubiquitous origin type.
	type RuntimeOrigin = RuntimeOrigin;
	/// Maximum number of block number to block hash mappings to keep (oldest pruned first).
	type BlockHashCount = ConstU32<250>;
	/// The weight of database operations that the runtime can invoke.
	type DbWeight = RocksDbWeight;
	/// Version of the runtime.
	type Version = ();
	/// Converts a module to the index of the module in `construct_runtime!`.
	///
	/// This type is being generated by `construct_runtime!`.
	type PalletInfo = PalletInfo;
	/// What to do if a new account is created.
	type OnNewAccount = ();
	/// What to do if an account is fully reaped from the system.
	type OnKilledAccount = ();
	/// The data to be stored in an account.
	type AccountData = pallet_balances::AccountData<Balance>;
	/// Weight information for the extrinsics of this pallet.
	type SystemWeightInfo = ();
	/// This is used as an identifier of the chain. 42 is the generic substrate prefix.
	type SS58Prefix = ();
	/// The set code logic used for this runtime.
	type OnSetCode = ();
	/// The maximum number of consumers allowed on a single account.
	type MaxConsumers = ConstU32<16>;
	/// The type for recording an account's balance.
	type RuntimeTask = ();
	/// Weight information for the extrinsics of this pallet.
	type ExtensionsWeightInfo = ();
	/// This is used as an identifier of the chain. 42 is the generic substrate prefix.
	type SingleBlockMigrations = ();
	/// This is used as an identifier of the chain. 42 is the generic substrate prefix.
	type MultiBlockMigrator = ();
	/// This is used as an identifier of the chain. 42 is the generic substrate prefix.
	type PreInherents = ();
	/// This is used as an identifier of the chain. 42 is the generic substrate prefix.
	type PostInherents = ();
	/// This is used as an identifier of the chain. 42 is the generic substrate prefix.
	type PostTransactions = ();
}

impl pallet_balances::Config for Runtime {
	type MaxLocks = ConstU32<50>;
	type MaxReserves = ();
	type ReserveIdentifier = [u8; 8];
	/// The type for recording an account's balance.
	type Balance = Balance;
	/// The ubiquitous event type.
	type RuntimeEvent = RuntimeEvent;
	type DustRemoval = ();
	type ExistentialDeposit = ConstU128<500>;
	type AccountStore = System;
	type WeightInfo = ();
	type FreezeIdentifier = ();
	type MaxFreezes = ();
	type RuntimeHoldReason = ();
	type RuntimeFreezeReason = ();
	type DoneSlashHandler = ();
}

impl pallet_timestamp::Config for Runtime {
	/// A timestamp: milliseconds since the unix epoch.
	type Moment = u64;
	type OnTimestampSet = ();
	type MinimumPeriod = ConstU64<{ SLOT_DURATION / 2 }>;
	type WeightInfo = ();
}

impl pallet_sudo::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type RuntimeCall = RuntimeCall;
	type WeightInfo = ();
}

// Create the runtime by composing the FRAME pallets that were previously configured.
construct_runtime!(
	pub struct Runtime {
		System: frame_system = 0,
		Timestamp: pallet_timestamp = 1,
		Balances: pallet_balances = 2,
		Sudo: pallet_sudo = 3,
	}
);

impl_opaque_keys! {
	pub struct SessionKeys {}
}





