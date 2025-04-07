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

//! Block construction benchmark.
//!
//! This benchmark is expected to measure block construction.
//! We want to protect against cold-cache attacks, and so this
//! benchmark should not rely on any caching (except those entries that
//! DO NOT depend on user input). Thus transaction generation should be
//! based on randomized data.

use std::{borrow::Cow, collections::HashMap, pin::Pin, sync::Arc};

use async_trait::async_trait;
use node_primitives::Block;
use sc_transaction_pool_api::{
	ImportNotificationStream, PoolStatus, ReadyTransactions, TransactionFor, TransactionSource,
	TransactionStatusStreamFor, TxHash, TxInvalidityReportMap,
};
use sp_runtime::OpaqueExtrinsic;

use crate::{
	common::SizeType,
	core::{self, Mode, Path},
};

pub struct ConstructionBenchmarkDescription {
	pub key_types: String,
	pub block_type: String,
	pub size: SizeType,
	pub database_type: String,
}

pub struct ConstructionBenchmark {
	database: String,
	transactions: String,
}

impl core::BenchmarkDescription for ConstructionBenchmarkDescription {
	fn path(&self) -> Path {
		Path::new(&["node", "proposer"])
	}

	fn setup(self: Box<Self>) -> Box<dyn core::Benchmark> {
		Box::new(ConstructionBenchmark {
			database: String::new(),
			transactions: String::new(),
		})
	}

	fn name(&self) -> Cow<'static, str> {
		"Block construction benchmark".into()
	}
}

impl core::Benchmark for ConstructionBenchmark {
	fn run(&mut self, _mode: Mode) -> std::time::Duration {
		std::time::Duration::from_secs(0)
	}
}

#[derive(Clone, Debug)]
pub struct PoolTransaction {
	data: Arc<OpaqueExtrinsic>,
	hash: node_primitives::Hash,
}

impl From<OpaqueExtrinsic> for PoolTransaction {
	fn from(e: OpaqueExtrinsic) -> Self {
		PoolTransaction { data: Arc::from(e), hash: node_primitives::Hash::zero() }
	}
}

impl sc_transaction_pool_api::InPoolTransaction for PoolTransaction {
	type Transaction = Arc<OpaqueExtrinsic>;
	type Hash = node_primitives::Hash;

	fn data(&self) -> &Self::Transaction {
		&self.data
	}

	fn hash(&self) -> &Self::Hash {
		&self.hash
	}

	fn priority(&self) -> &u64 {
		unimplemented!()
	}

	fn longevity(&self) -> &u64 {
		unimplemented!()
	}

	fn requires(&self) -> &[Vec<u8>] {
		unimplemented!()
	}

	fn provides(&self) -> &[Vec<u8>] {
		unimplemented!()
	}

	fn is_propagable(&self) -> bool {
		unimplemented!()
	}
}

#[derive(Clone, Debug)]
pub struct Transactions(Vec<Arc<PoolTransaction>>);
pub struct TransactionsIterator(std::vec::IntoIter<Arc<PoolTransaction>>);

impl Iterator for TransactionsIterator {
	type Item = Arc<PoolTransaction>;

	fn next(&mut self) -> Option<Self::Item> {
		self.0.next()
	}
}

impl ReadyTransactions for TransactionsIterator {
	fn report_invalid(&mut self, _tx: &Self::Item) {}
}

#[async_trait]
impl sc_transaction_pool_api::TransactionPool for Transactions {
	type Block = Block;
	type Hash = node_primitives::Hash;
	type InPoolTransaction = PoolTransaction;
	type Error = sc_transaction_pool_api::error::Error;

	/// Asynchronously imports a bunch of unverified transactions to the pool.
	async fn submit_at(
		&self,
		_at: Self::Hash,
		_source: TransactionSource,
		_xts: Vec<TransactionFor<Self>>,
	) -> Result<Vec<Result<node_primitives::Hash, Self::Error>>, Self::Error> {
		unimplemented!()
	}

	/// Asynchronously imports one unverified transaction to the pool.
	async fn submit_one(
		&self,
		_at: Self::Hash,
		_source: TransactionSource,
		_xt: TransactionFor<Self>,
	) -> Result<TxHash<Self>, Self::Error> {
		unimplemented!()
	}

	async fn submit_and_watch(
		&self,
		_at: Self::Hash,
		_source: TransactionSource,
		_xt: TransactionFor<Self>,
	) -> Result<Pin<Box<TransactionStatusStreamFor<Self>>>, Self::Error> {
		unimplemented!()
	}

	async fn ready_at(
		&self,
		_at: Self::Hash,
	) -> Box<dyn ReadyTransactions<Item = Arc<Self::InPoolTransaction>> + Send> {
		Box::new(TransactionsIterator(self.0.clone().into_iter()))
	}

	fn ready(&self) -> Box<dyn ReadyTransactions<Item = Arc<Self::InPoolTransaction>> + Send> {
		unimplemented!()
	}

	fn report_invalid(
		&self,
		_at: Option<Self::Hash>,
		_invalid_tx_errors: TxInvalidityReportMap<TxHash<Self>>,
	) -> Vec<Arc<Self::InPoolTransaction>> {
		Default::default()
	}

	fn futures(&self) -> Vec<Self::InPoolTransaction> {
		unimplemented!()
	}

	fn status(&self) -> PoolStatus {
		unimplemented!()
	}

	fn import_notification_stream(&self) -> ImportNotificationStream<TxHash<Self>> {
		unimplemented!()
	}

	fn on_broadcasted(&self, _propagations: HashMap<TxHash<Self>, Vec<String>>) {
		unimplemented!()
	}

	fn hash_of(&self, _xt: &TransactionFor<Self>) -> TxHash<Self> {
		unimplemented!()
	}

	fn ready_transaction(&self, _hash: &TxHash<Self>) -> Option<Arc<Self::InPoolTransaction>> {
		unimplemented!()
	}

	async fn ready_at_with_timeout(
		&self,
		_at: Self::Hash,
		_timeout: std::time::Duration,
	) -> Box<dyn ReadyTransactions<Item = Arc<Self::InPoolTransaction>> + Send> {
		unimplemented!()
	}
}
