//! freitask — a parte do binário que os testes de fora e a TUI alcançam em
//! processo: os comandos que escrevem (`escrita`, `deslocar`), o que eles
//! devolvem (`saida`), a linha de uma task (`linhas`) e a TUI (`tui`).
//!
//! A CLI (`cli.rs`) e o `doctor` ficam só no binário. O contrato da TUI para
//! o aceite é a decisão f7.1 em `docs/decisoes-da-reescrita.md`.

pub mod deslocar;
pub mod escrita;
pub mod linhas;
pub mod saida;
pub mod tui;
