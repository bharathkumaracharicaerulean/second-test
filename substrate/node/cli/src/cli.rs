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

//! Command-line interface implementation for the Substrate node.
//! This module defines the structure of the CLI and its subcommands.

use sc_cli::{RunCmd, KeySubcommand, VerifyCmd, VanityCmd, SignCmd, BuildSpecCmd, CheckBlockCmd, ExportBlocksCmd, ExportStateCmd, ImportBlocksCmd, PurgeChainCmd, RevertCmd, ChainInfoCmd};

/// Main CLI structure that holds all command-line arguments.
/// This includes both the subcommand and the run command parameters.
#[derive(Debug, clap::Parser)]
pub struct Cli {
	/// The subcommand to execute.
	/// This can be any of the commands defined in the Subcommand enum.
	#[command(subcommand)]
	pub subcommand: Option<Subcommand>,

	/// The run command parameters.
	/// These parameters are used when running the node without a subcommand.
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

/// Enum defining all possible subcommands that can be executed.
/// Each variant represents a different CLI command with its own parameters.
#[derive(Debug, clap::Subcommand)]
pub enum Subcommand {
	/// Key management CLI utilities.
	/// This includes commands for generating, importing, and managing cryptographic keys.
	#[command(subcommand)]
	Key(KeySubcommand),

	/// Verify a signature for a message.
	/// This command verifies a signature provided on STDIN using a given public or secret key.
	Verify(VerifyCmd),

	/// Generate a seed that provides a vanity address.
	/// This command generates a cryptographic seed that produces an address with a specific pattern.
	Vanity(VanityCmd),

	/// Sign a message with a given secret key.
	/// This command signs a message provided on STDIN using a specified secret key.
	Sign(SignCmd),

	/// Build a chain specification.
	/// This command generates a chain specification file that can be used to initialize a new chain.
	BuildSpec(BuildSpecCmd),

	/// Validate blocks in the chain.
	/// This command checks if blocks in the chain are valid according to the consensus rules.
	CheckBlock(CheckBlockCmd),

	/// Export blocks from the chain.
	/// This command exports blocks from the chain to a file.
	ExportBlocks(ExportBlocksCmd),

	/// Export the state of a given block into a chain spec.
	/// This command exports the state of a specific block to create a new chain specification.
	ExportState(ExportStateCmd),

	/// Import blocks into the chain.
	/// This command imports blocks from a file into the chain.
	ImportBlocks(ImportBlocksCmd),

	/// Remove the whole chain.
	/// This command removes all chain data from the database.
	PurgeChain(PurgeChainCmd),

	/// Revert the chain to a previous state.
	/// This command reverts the chain to a specified block.
	Revert(RevertCmd),

	/// Display database meta columns information.
	/// This command shows information about the database structure.
	ChainInfo(ChainInfoCmd),
}
