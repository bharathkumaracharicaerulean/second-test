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

//! Transaction pool integrated benchmarks.
//!
//! The goal of this benchmark is to figure out time needed to fill
//! the transaction pool for the next block.

use std::borrow::Cow;

use crate::core::{self, Mode, Path};

pub struct PoolBenchmarkDescription {
	pub database_type: String,
}

pub struct PoolBenchmark {
	database: String,
}

impl core::BenchmarkDescription for PoolBenchmarkDescription {
	fn path(&self) -> Path {
		Path::new(&["node", "txpool"])
	}

	fn setup(self: Box<Self>) -> Box<dyn core::Benchmark> {
		Box::new(PoolBenchmark {
			database: String::new(),
		})
	}

	fn name(&self) -> Cow<'static, str> {
		"Transaction pool benchmark".into()
	}
}

impl core::Benchmark for PoolBenchmark {
	fn run(&mut self, _mode: Mode) -> std::time::Duration {
		std::time::Duration::from_secs(0)
	}
}
