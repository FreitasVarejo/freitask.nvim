//! freitask-core — o núcleo puro do freitask.
//!
//! Sem I/O, sem relógio, sem variável de ambiente: tudo que entra vem por
//! parâmetro e tudo que sai é valor. Operações que mudam o vault devolvem um
//! plano de efeitos sobre arquivos, que a crate `freitask-vault` executa.
