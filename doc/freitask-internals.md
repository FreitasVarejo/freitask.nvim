# Freitask — mapa dos módulos

Documento para quem vai **mexer no código**. Para o formato das tarefas, o
fluxo de uso e o contrato com agentes de IA, ver [`freitask.md`](freitask.md) —
este aqui não repete nada de lá.

Tudo vive em `nvim/lua/freitask/`. `nvim/lua/util/freitask.lua` é uma fachada de
uma linha que existe só para não quebrar quem já apontava para o caminho antigo
(principalmente a CLI, chamada de fora do Neovim).

---

## A pilha

Ela é **acíclica por construção**, e a regra para mantê-la assim é: um módulo só
pode requerer módulos à sua esquerda.

```
config ──┬── fs ────┬── path ──┬── model ──┬── cache ──┬── task ── board ──┬── doctor
         └── status ┘   md ────┘           └── links ──┘                   │
                                                                           ├── ui.form ── edit ──┬── ui.picker
                                                                           └────────────────────┴── autocmd
```

| Módulo | Linhas | Responsabilidade | Não é responsabilidade dele |
| --- | ---: | --- | --- |
| `config` | 99 | Caminhos, tipos de arquivamento, `status.json` default, tabelas de callout. Sem lógica, sem estado. | Decidir qualquer coisa. |
| `types` | 64 | Só `---@class`: `Model`, `Entry`, `Ctx`, `StatusMeta`, `Finding`, `TaskRef`. | Ter código. |
| `fs` | 64 | Ler arquivo, recarregar/reapontar buffer, varrer o vault. | Saber o que é uma task. |
| `status` | 84 | O vocabulário de status lido do `status.json`, e o mapa reverso callout → número. | Saber o status de uma *task* — isso é o callout dela. |
| `path` | 146 | Tudo que se deriva de um **caminho**: projeto, id, arquivamento, `kebab`. | Ler arquivo. |
| `md` | 196 | Linhas de markdown: blockquote, frontmatter, seção `##`, splice. | Saber o que é uma task. |
| `model` | 162 | **Parser e serializer do bloco.** Puro: linhas entram, `freitask.Model` sai. | Tocar disco, notificar, conhecer buffer. |
| `cache` | 185 | O índice em memória das tasks **ativas**, com o bloco de cada uma. | Enxergar arquivadas (elas são lidas do disco sob demanda). |
| `links` | 145 | Reescrever os wikilinks do vault quando um arquivo muda de endereço. | Decidir que ele mudou. |
| `task` | 218 | Operações sobre o **arquivo** de uma task: criar, mover, localizar por id. | Regenerar o board — quem chama decide quando. |
| `board` | 151 | O `CURRENT.md`. | Ser fonte de verdade de coisa alguma. |
| `doctor` | 280 | Verificação, reparo e migração de formato. | Adivinhar (não inventa data nem resolve conflito). |
| `ui/form` | 244 | O buffer flutuante: desenhar, completar, parsear de volta. | Saber o que fazer com o modelo. |
| `edit` | 257 | Resolver **o que** editar e persistir o resultado. | Desenhar janela. |
| `ui/picker` | 227 | O picker de dois níveis, no Snacks. | Ter regra de task nenhuma. |
| `autocmd` | 90 | O que dispara sozinho: regen ao salvar, keymaps buffer-local. | — |
| `init` | 86 | A API pública: os submódulos + a superfície plana de 38 nomes. | Ter lógica. |

---

## Por que estas costuras

- **`model` é a única parte 100% pura**, e é onde está quase todo o teste. É a
  parte que erra em silêncio: um bloco serializado errado só aparece quando o
  arquivo já foi gravado.
- **`md` não sabe o que é uma task.** Se soubesse, o parser do bloco e as
  operações de arquivo teriam que concordar sobre o que é "o bloco" em dois
  lugares.
- **`path` existe porque o caminho é fonte de verdade** de duas coisas — projeto
  e arquivamento —, exatamente como o callout é a do status. Não há flag de
  frontmatter para nenhuma das duas.
- **`ui/form` e `edit` são separados** porque `apply_edit` trata mudança de id
  como *rename*, e um rename arrasta o arquivo, o `id:` do frontmatter, os
  wikilinks do vault inteiro e a branch git de mesmo nome. Isso não é desenho de
  janela, e o form deve poder ser trocado sem encostar nisso.
- **`task` não chama `board.rebuild_current`.** Quem chama é que sabe se vale a
  pena: o picker arquiva e regenera uma vez só, no fim.

## Convenções

- **`path_`, `model_`** com underscore final nos requires: `path` e `model` são
  nomes de variável usados em quase toda função desses arquivos, e sombrear o
  módulo daria um erro mudo.
- **Comentário responde "por quê", não "o quê".** O código já diz o que faz; o
  que se perde com o tempo é a razão de ele fazer *assim* — e é essa razão que
  impede alguém (você, daqui a seis meses, ou um agente) de "simplificar" um
  guarda que existe por um motivo. Onde a razão é longa demais para o código,
  ela está em `freitask.md`, não duplicada aqui.
- **Especificação de formato mora em `freitask.md`.** Duas cópias de uma spec
  divergem; a do código era a que ficava desatualizada.

---

## Testes

```bash
nvim/tests/run.sh          # a suíte inteira
nvim/tests/run.sh freitask # filtra por nome de arquivo
```

`nvim/tests/runner.lua` é um harness de ~80 linhas rodando sob `nvim --clean -l`
— sem busted, sem plenary. A suíte testa um módulo que só toca arquivos e
`vim.fn`; uma dependência externa aqui seria um passo de setup que ninguém
lembra de fazer antes de mexer no parser.

Cobertura em `freitask_spec.lua`: as funções **puras** — `path.kebab`,
`path.split_task_path`, `path.suggest_archive_type`, `status.status_meta`,
`model.is_status_text`, `model.parse_block` (formato atual, legado e status 0),
`model.serialize_block`, o round-trip entre os dois, `md.first_block_range`,
`md.block_around`, `md.splice`, `md.append_history`,
`md.update_frontmatter_key` e `links.rewrite_link`.

O que **não** está coberto por teste automatizado, e portanto precisa de
verificação manual quando você mexer: as operações de escrita (`task.move_task`,
`edit.apply_edit`, `board.rebuild_current`, `doctor --fix`) e a UI. Para as
primeiras, o caminho mais rápido é um vault descartável — o módulo deriva a raiz
de `~`, então basta trocar `$HOME`:

```bash
SB=$(mktemp -d); mkdir -p "$SB/ObsidianVault/tasks/p"
printf -- '---\nid: t1\n---\n\n> [!todo] Um\n> [[tasks/p/t1|t1]]\n' > "$SB/ObsidianVault/tasks/p/t1.md"
echo 'ref [[t1]] e [[tasks/p/t1|t1]]' > "$SB/ObsidianVault/n.md"

run() { HOME="$SB" nvim --clean --cmd "set runtimepath+=$HOME/dotfiles/nvim" \
  -l ~/dotfiles/nvim/lua/util/freitask_cli.lua "$@"; }
run archive t1 done && run unarchive t1 && run rename t1 t2 && run doctor
```

Confira, depois: os dois formatos de wikilink em `n.md`, o `id:` do
frontmatter, o log em `## Histórico` e o `CURRENT.md`.

## Ao mexer

1. `nvim/tests/run.sh`
2. `cd nvim && luacheck lua/freitask lua/util lua/plugins/freitask.lua tests`
3. `freitask doctor` no vault real (read-only; deve sair verde)

O luacheck **não** pega `mod.foo` apontando para uma função que não existe —
`mod` é uma tabela válida e o erro só aparece em runtime, possivelmente meses
depois, num caminho que você não exercita. Se você mover funções entre módulos,
vale rodar um checador de referências cruzadas (foi assim que dois erros de
substituição apareceram durante a divisão do monolito).
