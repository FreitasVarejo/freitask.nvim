//! freitask — o binário: CLI, TUI e despacho.
//!
//! `tui` e `freitask` sem argumentos abrem a TUI num terminal (a TUI ainda não
//! está portada) e, fora dele, imprimem o usage com saída 2; `current` executa
//! o Neovim no `CURRENT.md`; o resto é a CLI (`cli.rs`).

mod cli;
mod deslocar;
mod doctor;
mod escrita;
mod linhas;
mod usage;

use std::process::ExitCode;

fn main() -> ExitCode {
    cli::run(std::env::args_os().skip(1).collect())
}
