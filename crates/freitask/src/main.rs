//! freitask — o binário: CLI, TUI e despacho.
//!
//! Esqueleto da fatia 0: ainda não há subcomando. Sai 2 com a mensagem de uso
//! mínima, que as fatias seguintes substituem pelo clap.

use std::process::ExitCode;

fn main() -> ExitCode {
    eprintln!("freitask: esqueleto da reescrita; nenhum subcomando implementado ainda");
    ExitCode::from(2)
}
