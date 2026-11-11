# freitask

Gestão de tarefas em Markdown sobre um vault do Obsidian. Uma tarefa é **um
arquivo**; o status é **o tipo do callout**; estar arquivada é **o caminho**.
Não há banco de dados, e nenhum estado vive fora dos arquivos que o Obsidian já
sincroniza.

É **um binário Rust**, `freitask`, que é a CLI e a TUI, sobre o vault
`~/ObsidianVault` (workspace na raiz, `crates/`). O nome `.nvim` é do tempo em
que isto era um plugin com picker no Snacks; o Neovim não carrega mais nada
daqui.

## Instalação

```bash
cargo install --locked --path crates/freitask   # instala freitask em ~/.cargo/bin
```

Precisa de Rust stable via rustup, na versão do `rust-version` do
[`Cargo.toml`](Cargo.toml). No dotfiles, o `setup.sh` faz isso, e o shim
`~/.local/bin/freitask` só faz `exec` do binário.

## Uso

```bash
freitask                  # num terminal: abre a TUI (`?` lista as teclas)
freitask list [--json]    # e o resto dos subcomandos: `freitask help`
```

A CLI é o caminho que a TUI, o Obsidian, scripts e agentes de IA devem usar em
vez de `mv`/`rm`, porque só ela mantém os invariantes acoplados em sincronia
(arquivo, link da linha 2, referências do vault, `## Histórico`).

## Documentação

- [`docs/freitask.md`](docs/freitask.md) — formato das tarefas, fluxo de uso e o
  contrato para agentes de IA.
- [`docs/freitask-internals.md`](docs/freitask-internals.md) — mapa do código,
  regras de dependência e como testar.

## Desenvolvimento

Os alvos estão no `Makefile` (`make help`); não se digita `cargo test` à mão.

```bash
make check     # fmt + clippy
make test      # unitários, propriedades e TUI
make aceite    # os testes congelados de tests/aceite/
make portoes   # tudo isso, mais mutação e catracas; grava o PORTOES.md
```

`tests/aceite/` e os snapshots não se editam nem se apagam (regra no
[`AGENTS.md`](AGENTS.md)). `make diff` compara o binário com o oráculo Lua (a
tag `oraculo-lua`) e precisa de `nvim` no PATH.

Até o cutover, `lua/` e `tui/` seguem no repo como oráculo (ver "Até o cutover"
em [`docs/freitask-internals.md`](docs/freitask-internals.md)).
