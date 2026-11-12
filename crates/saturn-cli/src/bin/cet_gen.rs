// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
//! Standalone synthetic Capture Event Track generator binary (`cet-gen`).

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if let Err(error) = saturn_cli::cet_cmd::run_cet_gen_cli(args) {
        eprintln!("cet-gen: {error}");
        std::process::exit(1);
    }
}
