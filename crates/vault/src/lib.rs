//! freitask-vault — a fronteira com o disco.
//!
//! Varre e lê o vault, grava com CAS (temporário, fsync, rename), toma o lock
//! entre dois `freitask` na mesma máquina e executa os planos do `core`.
