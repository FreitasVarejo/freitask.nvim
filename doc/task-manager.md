# Task Manager & Daily Notes (Obsidian)

Como registrar tarefas e o planejamento diário neste setup — pensado tanto para
o usuário quanto para **agentes de IA** que precisem ler/criar notas sem quebrar
as convenções.

Implementação: `nvim/lua/util/tasks.lua` (núcleo) + `nvim/lua/plugins/task-manager.lua`
(spec do plugin). Opera **exclusivamente** sob `~/ObsidianVault/tasks/`.

---

## Modelo mental

- **Um arquivo por tarefa**: `~/ObsidianVault/tasks/<projeto>/<id>.md`.
- **Projeto = subdiretório** de `tasks/` (id kebab-case, ex.: `bjju-web`).
- **`id` da tarefa = nome do arquivo = nome da branch git.** Não existe campo
  "branch" separado — os dois são a mesma coisa (sem prefixo `feat/`).
- **`CURRENT.md`** é o painel diário, **gerado automaticamente** — não edite as
  seções de projeto à mão (só a seção `## Notas Avulsas` é preservada).
- **`status.json`** define os status (número → callout/título/ícone/cor).

Diretórios **reservados** (não são projetos): `daily/` (arquivo diário) e
`templates/`.

---

## Formato do bloco de uma tarefa

O topo de cada arquivo de tarefa é um *callout* do Obsidian de 3 linhas:

```markdown
> [!todo] **Título da task**
> [[id-da-task]]
> 1 - Backlog
```

| Linha | Conteúdo | Papel |
| ----- | -------- | ----- |
| 1 | `> [!<callout>] **<título>**` | O **tipo do callout** (`todo`, `example`, …) dá ícone/cor **e é a fonte de verdade do status**. O título vai em negrito aqui. |
| 2 | `> [[<id>]]` | Link do Obsidian para a própria nota; o `id` também é o nome da branch. |
| 3 | `> <n> - <nome do status>` | Texto legível do status (derivado do número). |
| 4+ | `> qualquer texto` | Livre: impedimentos, notas, etc. **Preservado na íntegra** ao editar. |

Depois do bloco vem o corpo livre do arquivo (`## Notas Soltas`, checklists…),
que a ferramenta **nunca** reescreve.

**Frontmatter YAML** (`--- id/aliases/tags ---`) que o obsidian.nvim injeta ao
salvar é tolerado: a detecção do callout pula o frontmatter automaticamente.

### Status (`~/ObsidianVault/tasks/status.json`)

| nº | callout | título |
| -- | ------- | ------ |
| 1 | `todo` | Backlog |
| 2 | `example` | In Progress |
| 3 | `warning` | Blocked |
| 4 | `question` | Review |
| 5 | `success` | Done |

Editar `status.json` muda rótulos/ícones/cores. O parser mapeia
`callout → número` a partir desse arquivo, então mantenha os callouts distintos.

---

## `CURRENT.md` — o painel diário

Gerado a partir de todos os arquivos de tarefa:

```markdown
---
date: 2026-08-03
weekday: Monday
---

# Planning diário 03/08/26

## Notas Avulsas

- notas do dia, escritas à mão (preservadas entre regenerações)

## bjju-web

> [!todo] **Criar jiujitsu unicamp**
> [[criar-jiujitsu-unicamp]]
> 1 - Backlog
```

- **Auto-regeneração**: salvar qualquer arquivo de tarefa (`BufWritePost`)
  regenera o `CURRENT.md` silenciosamente — tarefas novas/mudanças de status
  aparecem sozinhas. Se o `CURRENT.md` estiver com edições não salvas, a regen é
  pulada (sem sobrescrever seu trabalho).
- **Arquivo diário**: ao regenerar num dia novo, o painel do dia anterior é
  snapshotado em `tasks/daily/YYYY-MM-DD.md` antes de ser recarimbado com a data
  de hoje.
- **`## Notas Avulsas`** é a única seção editável à mão que sobrevive à regen.

---

## Uso interativo (Neovim)

| Tecla | Onde | Ação |
| ----- | ---- | ---- |
| `<leader>ob` | qualquer lugar | Abre o picker: projetos → tarefas |
| `<leader>oe` | cursor sobre um callout (em `CURRENT.md` ou no arquivo da tarefa) | Abre o **form** de edição multi-campo |
| `<C-a>` | picker de projetos | Regenera e abre o `CURRENT.md` |

Dentro do picker de tarefas: `<C-t>` nova tarefa · `<C-x>` deletar · `<C-e>`
editar (form) · `<C-o>` voltar. No picker de projetos: `<C-c>` novo projeto.

O **form** (janela flutuante) edita de uma vez, na ordem: `Título`,
`Id / Branch`, `Status`, e depois notas livres. `⏎` salva, `q`/`<Esc>` cancela,
`<C-s>` escolhe o status. Editar a partir do `CURRENT.md` grava no arquivo-fonte
**e** atualiza o bloco no painel in place. Renomear o `Id` faz rename da tarefa
via delete + recreate (o arquivo e a branch passam a se chamar pelo novo id;
backlinks externos **não** são migrados automaticamente).

---

## Para agentes de IA — como criar/editar tarefas sem quebrar nada

1. **Criar tarefa**: escreva `tasks/<projeto>/<id>.md` com o bloco de 3 linhas
   acima. `id` deve ser kebab-case (use `M.template` se estiver em Lua). O
   projeto (diretório) precisa existir.
2. **Status é derivado do tipo do callout da linha 1** — para mudar o status,
   troque o callout (`[!todo]`→`[!success]`) e mantenha a linha 3 coerente. Não
   confie em número na linha 1 (o formato atual não tem número lá).
3. **`id` == nome da branch** (sem `feat/`). Não invente uma linha `Branch:`; o
   `[[id]]` já representa ambos.
4. **Preserve as linhas 4+** (impedimentos/notas) ao editar — elas são texto
   livre do usuário.
5. **Reaproveite o parser canônico** em vez de regex ad-hoc:
   `require("util.tasks").parse_block(linhas)` →
   `{ status_num, title, id, extras }`, e `serialize_block(model)` de volta. O
   par é idempotente (round-trip seguro) e já lida com frontmatter e formatos
   legados.
6. **Não edite as seções `## <projeto>` do `CURRENT.md` à mão** — são geradas.
   Edite os arquivos de tarefa; o painel se atualiza ao salvar. Só
   `## Notas Avulsas` é livre.
7. **Ignore `daily/` e `templates/`** ao varrer projetos.

---

## Manutenção / verificação

- `nvim/lua/util/tasks.lua` concentra parser, serializer, resolver de cursor,
  form e geração do painel. `parse_block`/`serialize_block` são a fonte única do
  formato — mudanças de formato começam aí.
- `M.migrate_format()` converte arquivos de formatos antigos para o atual
  (idempotente); rode-o após qualquer mudança de formato, seguido de
  `M.rebuild_current()`.
- Validação headless (sem framework de teste; o repo é config):
  ```bash
  luacheck nvim/lua/util/tasks.lua
  nvim --headless -c 'lua assert(require("util.tasks").parse_block)' -c 'qa!'
  ```
