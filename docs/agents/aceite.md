# Testes de aceitação (`tests/aceite/`)

Como um agente escreve os testes congelados de uma fatia da reescrita (spec,
`~/ObsidianVault/projects/workflow-ia/reescrita-do-freitask-spec.md`, §3.3 e
§4). Quem escreve não vê a implementação: lê a spec e `docs/freitask.md`.

## Onde escrever

- `.aceite-rascunho/<fatia>/main.rs` (obrigatório) e os módulos que quiser ao
  lado (`mod util;` em `.aceite-rascunho/<fatia>/util.rs`). `make congelar
  FATIA=<fatia>` leva a pasta para `tests/aceite/<fatia>/` e a registra como
  alvo de teste; `tests/aceite/` em si não se escreve.
- Arquivos da raiz de `tests/aceite/` (`divergencias.toml`, `schema/*.json`,
  `dores.rs`) vão em `.aceite-rascunho/raiz/`; o `congelar` só copia o que
  ainda não existe lá.
- Cada pasta é autocontida: copie o helper da fatia anterior se precisar, não
  o edite lá. As dependências disponíveis são as `[dev-dependencies]`
  que `scripts/gerar-cargo-aceite.sh` escreve (e as mesmas de
  `scripts/rascunho.sh`); `gerador` dá vaults sintéticos (`gerador::minimo()`,
  `gerador::patologicos()`, `gerador::sintetico(semente, &Opcoes::default())`,
  `Vault::gravar(&raiz)`).

## Como conferir o rascunho

`scripts/rascunho.sh <fatia>` compila e roda o rascunho fora do workspace
(cópia em `target/rascunho/`), sem tocar em `tests/aceite/`. Antes de
congelar, o rascunho tem de compilar e tem de ficar VERMELHO contra o
esqueleto: teste que já passa sem implementação não prova nada.

## Como o teste roda o binário

- Caminho do binário: a raiz do repo vem em `FREITASK_REPO_RAIZ` quando o
  rascunho roda; congelado, é `CARGO_MANIFEST_DIR/../..`. Use
  `std::env::var("FREITASK_REPO_RAIZ").unwrap_or_else(|_| format!("{}/../..", env!("CARGO_MANIFEST_DIR")))`
  e o binário em `<raiz>/target/debug/freitask` (o `make aceite` compila
  antes). Não há `CARGO_BIN_EXE_*` fora do pacote do binário. Arquivos da raiz
  de `tests/aceite/` (`divergencias.toml`, `schema/`): o diretório vem em
  `FREITASK_ACEITE_RAIZ` no rascunho e, congelado, é `<raiz>/tests/aceite`:
  `std::env::var("FREITASK_ACEITE_RAIZ").unwrap_or_else(|_| format!("{raiz}/tests/aceite"))`.
- Vault: `<HOME>/ObsidianVault`, com `HOME` num `tempfile::tempdir()`. Monte os
  arquivos à mão ou com o `gerador`; o `.freitask/status.json` default está em
  `gerador::STATUS_JSON`.
- Ambiente: `env_clear()`, depois `HOME`, `PATH`, `LANG=C.UTF-8`,
  `LC_ALL=C.UTF-8` e `FREITASK_TODAY=AAAA-MM-DD` (fixa o dia; o painel e as
  adiadas dependem dele). `current_dir` no HOME.
- TUI: em processo pelo contrato `freitask::tui::Sessao`
  (`crates/freitask/src/tui.rs`, decisão f7.1 em
  `docs/decisoes-da-reescrita.md`), e o binário num PTY (`portable-pty`, `vt100`).
- Afirme explicitamente stdout, stderr, código de saída e o conteúdo dos
  arquivos tocados: o valor esperado está escrito no teste, nunca gerado da
  implementação. Nada de snapshot aqui.
- Um teste por comportamento, nome em português que diz o que prova
  (`set_recusa_done_com_saida_1_e_nao_grava`). Teste que precisa de `#[ignore]`
  não entra.

## O que cada fatia cobre

A tabela da spec (§5) diz o que entra em cada fatia; `docs/freitask.md` e a
§2 da spec dizem o comportamento, byte a byte quando houver texto de hoje a
reproduzir (mensagens de erro em stderr com o prefixo `freitask: `, o usage de
`freitask help`, as saídas de texto de `list`, `projects`, `statuses`,
`doctor`). O JSON se compara decodificado, nunca como texto; as chaves saem em
ordem alfabética.
