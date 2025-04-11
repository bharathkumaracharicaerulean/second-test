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

mod common;
mod construct;
#[macro_use]
mod core;
mod generator;
mod import;
mod simple_trie;
mod state_sizes;
mod tempdb;
mod trie;
mod txpool;

use clap::Parser;

use crate::{
	common::SizeType,
	construct::ConstructionBenchmarkDescription,
	core::{run_benchmark, Mode as BenchmarkMode},
	import::ImportBenchmarkDescription,
	txpool::PoolBenchmarkDescription,
};

#[derive(Debug, Parser)]
#[command(name = "node-bench", about = "Node integration benchmarks")]
struct Opt {
	/// Show list of all available benchmarks.
	///
	/// Will output ("name", "path"). Benchmarks can then be filtered by path.
	#[arg(short, long)]
	list: bool,

	/// Machine readable json output.
	///
	/// This also suppresses all regular output (except to stderr)
	#[arg(short, long)]
	json: bool,

	/// Filter benchmarks.
	///
	/// Run with `--list` for the hint of what to filter.
	filter: Option<String>,

	/// Number of transactions for block import with `custom` size.
	#[arg(long)]
	transactions: Option<usize>,

	/// Mode
	///
	/// "regular" for regular benchmark
	///
	/// "profile" mode adds pauses between measurable runs,
	/// so that actual interval can be selected in the profiler of choice.
	#[arg(short, long, default_value = "regular")]
	mode: BenchmarkMode,
}

fn main() {
	let opt = Opt::parse();

	if !opt.json {
		sp_tracing::try_init_simple();
	}

	let benchmarks = vec![
		Box::new(ImportBenchmarkDescription {
			key_types: "sr25519".to_string(),
			block_type: "transfer".to_string(),
			size: SizeType::Medium,
			database_type: "rocksdb".to_string(),
		}) as Box<dyn core::BenchmarkDescription>,
		Box::new(ConstructionBenchmarkDescription {
			key_types: "sr25519".to_string(),
			block_type: "transfer".to_string(),
			size: SizeType::Medium,
			database_type: "rocksdb".to_string(),
		}),
		Box::new(PoolBenchmarkDescription {
			database_type: "rocksdb".to_string(),
		}),
	];

	if opt.list {
		println!("Available benchmarks:");
		if let Some(filter) = opt.filter.as_ref() {
			println!("\t(filtered by \"{}\")", filter);
		}
		for benchmark in benchmarks.iter() {
			if opt.filter.as_ref().map(|f| benchmark.path().has(f)).unwrap_or(true) {
				println!("{}: {}", benchmark.name(), benchmark.path().full())
			}
		}
		return;
	}

	let mut results = Vec::new();
	for benchmark in benchmarks {
		if opt.filter.as_ref().map(|f| benchmark.path().has(f)).unwrap_or(true) {
			log::info!("Starting {}", benchmark.name());
			let result = run_benchmark(benchmark, opt.mode);
			log::info!("{}", result);

			results.push(result);
		}
	}

	if results.is_empty() {
		eprintln!("No benchmark was found for query");
		std::process::exit(1);
	}

	if opt.json {
		let json_result: String =
			serde_json::to_string(&results).expect("Failed to construct json");
		println!("{}", json_result);
	}
}
