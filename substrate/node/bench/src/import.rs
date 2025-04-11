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

//! Block import benchmark.
//!
//! This benchmark is expected to measure block import operation of
//! some more or less full block.
//!
//! As we also want to protect against cold-cache attacks, this
//! benchmark should not rely on any caching (except those that
//! DO NOT depend on user input). Thus block generation should be
//! based on randomized operation.
//!
//! This is supposed to be very simple benchmark and is not subject
//! to much configuring - just block full of randomized transactions.
//! It is not supposed to measure runtime modules weight correctness

use std::borrow::Cow;

use node_primitives::Block;
use sp_runtime::{
	generic::Header,
	traits::{Block as BlockT, Header as HeaderT},
};

use crate::{
	common::SizeType,
	core::{self, Mode, Path},
};

pub struct ImportBenchmarkDescription {
	pub key_types: String,
	pub block_type: String,
	pub size: SizeType,
	pub database_type: String,
}

pub struct ImportBenchmark {
	database: String,
	block: Block,
	block_type: String,
}

impl core::BenchmarkDescription for ImportBenchmarkDescription {
	fn path(&self) -> Path {
		Path::new(&["node", "import"])
	}

	fn setup(self: Box<Self>) -> Box<dyn core::Benchmark> {
		Box::new(ImportBenchmark {
			database: String::new(),
			block: Block::new(
				Header::<u32, sp_runtime::traits::BlakeTwo256>::new(0, Default::default(), Default::default(), Default::default(), Default::default()),
				Vec::new(),
			),
			block_type: String::new(),
		})
	}

	fn name(&self) -> Cow<'static, str> {
		"Block import benchmark".into()
	}
}

impl core::Benchmark for ImportBenchmark {
	fn run(&mut self, _mode: Mode) -> std::time::Duration {
		std::time::Duration::from_secs(0)
	}
}
