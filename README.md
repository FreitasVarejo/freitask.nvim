# freitask.nvim

Gestão de tarefas em Markdown sobre um vault do Obsidian, dirigida por
[Snacks.picker](https://github.com/folke/snacks.nvim). Uma tarefa é **um
arquivo**; o status é **o tipo do callout**; estar arquivada é **o caminho**.
Não há banco de dados, e nenhum estado vive fora dos arquivos que o Obsidian já
sincroniza.

Foi extraído do meu dotfiles quando passou de 3.000 linhas — ver
`docs/freitask-internals.md` para o mapa dos módulos.

## Instalação

Com [lazy.nvim](https://github.com/folke/lazy.nvim), a partir de um clone local:

```lua
{
  dir = vim.fn.expand("~/projects/freitask.nvim"),
  dependencies = { "folke/snacks.nvim" },
  keys = {
    { "<leader>ob", function() require("freitask").open_projects() end, desc = "Freitask" },
  },
  init = function()
    require("freitask").setup_autocmd()
  end,
}
```

O módulo assume o vault em `~/ObsidianVault/tasks/` (ver `lua/freitask/config.lua`).

## CLI

`lua/freitask/cli.lua` roda sob `nvim -l` e expõe as mesmas operações fora do
editor — é o caminho que o Obsidian, scripts e agentes de IA devem usar em vez
de `mv`/`rm`, porque só ele mantém os invariantes acoplados em sincronia:

```bash
freitask list
freitask archive <id> done|dropped|failed
freitask unarchive <id>
freitask rename <antigo> <novo>
freitask doctor [--fix]
```

O executável `freitask` que embrulha essa chamada vive no dotfiles
(`vault/.local/bin/freitask`), porque é o dotfiles que sabe onde este repo foi
clonado.

## Documentação

- [`docs/freitask.md`](docs/freitask.md) — formato das tarefas, fluxo de uso e o
  contrato para agentes de IA.
- [`docs/freitask-internals.md`](docs/freitask-internals.md) — mapa dos módulos,
  regras de dependência e como testar.

## Testes

```bash
tests/run.sh          # suíte inteira
tests/run.sh links    # filtra por nome de arquivo
luacheck lua tests
```

O harness (`tests/runner.lua`, ~80 linhas) roda sob `nvim --clean -l` de
propósito: a suíte não deve depender de plugin nenhum nem do estado do usuário.
