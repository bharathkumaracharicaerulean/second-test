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

use substrate_build_script_utils::{generate_cargo_keys, rerun_if_git_head_changed};
use std::env;

fn main() {
	// Generate the cargo keys for version information
	generate_cargo_keys();

	// Rerun the build script if the git HEAD changes
	rerun_if_git_head_changed();

	// Set environment variables for the build
	println!("cargo:rerun-if-changed=../../runtime/src/lib.rs");
	println!("cargo:rerun-if-changed=../../runtime/Cargo.toml");
	
	// Set the implementation name and version
	println!("cargo:rustc-env=SUBSTRATE_CLI_IMPL_VERSION={}", get_version());
	println!("cargo:rustc-env=SUBSTRATE_CLI_IMPL_NAME=substrate-node");
}

fn get_version() -> String {
	let commit_hash = if let Ok(hash) = std::process::Command::new("git")
		.args(&["rev-parse", "--short", "HEAD"])
		.output() {
		if hash.status.success() {
			String::from_utf8_lossy(&hash.stdout).trim().to_string()
		} else {
			"unknown".into()
		}
	} else {
		"unknown".into()
	};

	format!("{}-{}", env::var("CARGO_PKG_VERSION").unwrap_or_default(), commit_hash)
}

#[cfg(feature = "cli")]
mod cli {
	use substrate_build_script_utils::{generate_cargo_keys, rerun_if_git_head_changed};

	pub fn main() {
		generate_cargo_keys();
		rerun_if_git_head_changed();
	}
}
