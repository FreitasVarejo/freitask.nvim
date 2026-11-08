//! freitask — o binário: CLI, TUI e despacho.
//!
//! `tui` e `freitask` sem argumentos num terminal abrem a TUI; `current`
//! executa o Neovim no `CURRENT.md`; o resto é a CLI (`cli.rs`).

mod cli;
mod deslocar;
mod escrita;
mod linhas;
mod usage;

use std::process::ExitCode;

fn main() -> ExitCode {
    cli::run(std::env::args_os().skip(1).collect())
}
