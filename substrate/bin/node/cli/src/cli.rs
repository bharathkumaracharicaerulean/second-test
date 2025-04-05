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

use sc_cli::{RunCmd, KeySubcommand, VerifyCmd, VanityCmd, SignCmd, BuildSpecCmd, CheckBlockCmd, ExportBlocksCmd, ExportStateCmd, ImportBlocksCmd, PurgeChainCmd, RevertCmd, ChainInfoCmd};

/// An overarching CLI command definition.
#[derive(Debug, clap::Parser)]
pub struct Cli {
	/// Possible subcommand with parameters.
	#[command(subcommand)]
	pub subcommand: Option<Subcommand>,

	#[allow(missing_docs)]
	#[clap(flatten)]
	pub run: RunCmd,

	/// Disable automatic hardware benchmarks.
	///
	/// By default these benchmarks are automatically ran at startup and measure
	/// the CPU speed, the memory bandwidth and the disk speed.
	///
	/// The results are then printed out in the logs, and also sent as part of
	/// telemetry, if telemetry is enabled.
	#[arg(long)]
	pub no_hardware_benchmarks: bool,
}

/// Possible subcommands of the main binary.
#[derive(Debug, clap::Subcommand)]
pub enum Subcommand {
	/// Key management cli utilities
	#[command(subcommand)]
	Key(KeySubcommand),

	/// Verify a signature for a message, provided on STDIN, with a given (public or secret) key.
	Verify(VerifyCmd),

	/// Generate a seed that provides a vanity address.
	Vanity(VanityCmd),

	/// Sign a message, with a given (secret) key.
	Sign(SignCmd),

	/// Build a chain specification.
	BuildSpec(BuildSpecCmd),

	/// Validate blocks.
	CheckBlock(CheckBlockCmd),

	/// Export blocks.
	ExportBlocks(ExportBlocksCmd),

	/// Export the state of a given block into a chain spec.
	ExportState(ExportStateCmd),

	/// Import blocks.
	ImportBlocks(ImportBlocksCmd),

	/// Remove the whole chain.
	PurgeChain(PurgeChainCmd),

	/// Revert the chain to a previous state.
	Revert(RevertCmd),

	/// Db meta columns information.
	ChainInfo(ChainInfoCmd),
}
