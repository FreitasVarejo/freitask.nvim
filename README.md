# freitask

Gestão de tarefas em Markdown sobre um vault do Obsidian. Uma tarefa é **um
arquivo**; o status é **o tipo do callout**; estar arquivada é **o caminho**.
Não há banco de dados, e nenhum estado vive fora dos arquivos que o Obsidian já
sincroniza.

Duas partes, neste repo:

- **o motor** (`lua/freitask/`), com todas as regras, rodado pela CLI
  `freitask` sob `nvim --clean -l` — o Neovim como interpretador Lua, sem
  janela;
- **a TUI** (`tui/`, Rust), uma tela estilo lazygit que fala com o motor só
  pela CLI.

O nome `.nvim` é do tempo em que isto era um plugin com picker no Snacks; o
Neovim não carrega mais nada daqui.

## Instalação

A CLI é um shim que vive no dotfiles (`vault/.local/bin/freitask`), porque é o
dotfiles que sabe onde este repo foi clonado (`FREITASK_REPO`, default
`~/dev/freitask.nvim`). A TUI:

```bash
cargo install --path tui     # instala freitask-tui em ~/.cargo/bin
```

O módulo assume o vault em `~/ObsidianVault/projects/<projeto>/tasks/` (ver
`lua/freitask/config.lua`).

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
- [`docs/freitask-internals.md`](docs/freitask-internals.md) — mapa dos módulos,
  regras de dependência e como testar.

## Testes

```bash
tests/run.sh             # motor: funções puras + a CLI de ponta a ponta
tests/run.sh cli         # filtra por nome de arquivo
luacheck lua tests
(cd tui && cargo test)   # TUI: estado e teclas
(cd tui && cargo fmt --check && cargo clippy --all-targets -- -D warnings)
```

O harness (`tests/runner.lua`) roda sob `nvim --clean -l` de propósito: a
suíte não deve depender de plugin nenhum nem do estado do usuário.
