//! A TUI do freitask em processo (spec §1.3, §1.7 e §3.5).
//!
//! - `estado`: `State`, `Evento`, `Comando` e o `update` puro; as teclas
//!   do modo normal vêm da tabela de `atalhos`.
//! - `tela`: `view`, o desenho, que só lê o `State`.
//! - `dados`: o que a tela mostra, lido do vault pelos mesmos tipos do `core`
//!   que alimentam o `list --json`.
//! - `sessao`: o `State` mais o executor dos `Comando`s contra o vault, sem
//!   terminal. É o contrato que o aceite congela (decisão f7.1 em
//!   `docs/decisoes-da-reescrita.md`): `Sessao::abrir`, `tecla` e `tela`; o
//!   que sai do processo volta como `Pedido`.
//! - `laco`: o terminal de verdade em volta da `Sessao` (ratatui, editor,
//!   `$NVIM`).
//!
//! Nada aqui tem regra de task: a TUI monta o pedido e chama as mesmas
//! funções que a CLI (`escrita`, `deslocar`), pelo mesmo caminho: lock,
//! leitura, plano do `core` ou recusa, escrita com CAS, `rebuild`.

pub mod atalhos;
pub mod dados;
pub mod estado;
pub mod laco;
pub mod sessao;
pub mod tela;

#[cfg(test)]
mod propriedades;

pub use dados::{Dados, Projeto};
pub use estado::{Comando, Depois, Escrita, Evento, State, update};
pub use sessao::{Pedido, Sessao};
pub use tela::view;
