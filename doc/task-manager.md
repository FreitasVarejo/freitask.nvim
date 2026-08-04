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
  **`tasks/STATUS.md`** é uma folha de consulta gerada à mão (não é uma task)
  com todos os status na ordem do board, cada um como um callout renderizado —
  útil pra lembrar rápido a aparência/ordem sem abrir `status.json`.

Diretórios **reservados** (não são projetos): `daily/` (arquivo diário) e
`templates/`.

---

## Formato do bloco de uma tarefa

O topo de cada arquivo de tarefa é um *callout* do Obsidian de 3 linhas:

```markdown
> [!todo] Título da task
> [[tasks/bjju-web/id-da-task|id-da-task]]
> Ajustando formulário de inscrição
```

| Linha | Conteúdo | Papel |
| ----- | -------- | ----- |
| 1 | `> [!<callout>] <título>` | O **tipo do callout** (`todo`, `example`, …) dá ícone/cor **e é a ÚNICA fonte de verdade do status** — não há número redundante em nenhum outro lugar. O título vai em texto simples aqui (sem negrito — o `[!callout]` já destaca a linha). |
| 2 | `> [[tasks/<projeto>/<id>\|<id>]]` | Link do Obsidian para a própria nota, com caminho relativo ao vault (evita ambiguidade entre projetos com o mesmo id) e o `id` como alias; o `id` também é o nome da branch. |
| 3 | `> <nome>` | **Texto livre, independente do status** — você informa/edita esse "nome" a cada criação/edição do callout (ex.: em que ponto a task está, um resumo curto). Omitida se vazia. |
| 4+ | `> qualquer texto` | Livre: impedimentos, notas, etc. **Preservado na íntegra** ao editar. |

Depois do bloco vem o corpo livre do arquivo (`## Notas Soltas`, checklists…),
que a ferramenta **nunca** reescreve.

**Frontmatter YAML** (`--- id/aliases/tags ---`) que o obsidian.nvim injeta ao
salvar é tolerado: a detecção do callout pula o frontmatter automaticamente.

### Status (`~/ObsidianVault/tasks/status.json`)

Cobre todos os tipos de callout suportados pelo
[render-markdown.nvim](https://github.com/MeanderingProgrammer/render-markdown.nvim/wiki/Callouts),
ordenados como uma narrativa de workflow (a ordem/número só define a
**ordenação** das tasks no board — o status em si vem do nome do callout):

| nº | callout | título |
| -- | ------- | ------ |
| 1 | `todo` | Todo |
| 2 | `note` | Note |
| 3 | `info` | Info |
| 4 | `abstract` | Abstract |
| 5 | `summary` | Summary |
| 6 | `tldr` | TL;DR |
| 7 | `example` | Example |
| 8 | `important` | Important |
| 9 | `question` | Question |
| 10 | `help` | Help |
| 11 | `faq` | FAQ |
| 12 | `attention` | Attention |
| 13 | `warning` | Warning |
| 14 | `caution` | Caution |
| 15 | `bug` | Bug |
| 16 | `failure` | Failure |
| 17 | `fail` | Fail |
| 18 | `missing` | Missing |
| 19 | `danger` | Danger |
| 20 | `error` | Error |
| 21 | `tip` | Tip |
| 22 | `hint` | Hint |
| 23 | `success` | Success |
| 24 | `check` | Check |
| 25 | `done` | Done |
| 26 | `quote` | Quote |
| 27 | `cite` | Cite |

Tasks novas (`M.template`) começam em `1` (`todo`).

Editar `status.json` muda rótulos/ícones/cores/ordem. O parser mapeia
`callout → número` a partir desse arquivo, então mantenha os callouts
distintos. Renumerar/reordenar não quebra tasks existentes: o status de cada
task é lido do `[!callout]` da linha 1, nunca de um número gravado no arquivo.

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

> [!todo] Criar jiujitsu unicamp
> [[tasks/bjju-web/criar-jiujitsu-unicamp|criar-jiujitsu-unicamp]]
> Ajustando formulário de inscrição
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

Dentro do picker de tarefas: `<C-t>` nova tarefa · `<C-x>` deletar · `<C-r>`
arquivar · `<C-e>` editar (form) · `<C-o>` voltar. No picker de projetos:
`<C-c>` novo projeto.

O **form** (janela flutuante) edita de uma vez, na ordem: `Título`,
`Id / Branch`, `Status`, `Nome` (o texto livre da linha 3, independente do
status), e depois notas livres. `⏎` salva, `q`/`<Esc>` cancela, `<C-s>` escolhe
o status. Editar a partir do `CURRENT.md` grava no arquivo-fonte **e** atualiza
o bloco no painel in place. Renomear o `Id` faz rename da tarefa via delete +
recreate (o arquivo e a branch passam a se chamar pelo novo id; backlinks
externos **não** são migrados automaticamente).

### Arquivar vs. deletar

Ambos tiram a tarefa do `CURRENT.md` imediatamente (a regen roda na hora),
mas diferem no que sobra em disco:

- **Arquivar** (`<C-r>` no picker, ou `M.archive_task(path)`): marca
  `archived: true` no frontmatter YAML do arquivo — o `.md` continua existindo
  e é essa marca que indica que foi arquivada. `M.build_cache`/`update_cache_entry`
  ignoram arquivos arquivados, por isso somem do cache e do painel. Não há
  comando de "desarquivar" ainda — para reverter, apague a linha `archived: true`
  do frontmatter manualmente.
- **Deletar** (`<C-x>` no picker): pede confirmação e apaga o arquivo do disco
  de vez.

---

## Para agentes de IA — como criar/editar tarefas sem quebrar nada

1. **Criar tarefa**: escreva `tasks/<projeto>/<id>.md` com o bloco de 3 linhas
   acima. `id` deve ser kebab-case (use `M.template` se estiver em Lua). O
   projeto (diretório) precisa existir.
2. **Status é derivado EXCLUSIVAMENTE do tipo do callout da linha 1** — para
   mudar o status, troque o callout (`[!todo]`→`[!success]`). Não escreva
   número/status em nenhuma outra linha; a linha 3 é um campo livre à parte
   (ver item 4 a seguir), não o status.
3. **`id` == nome da branch** (sem `feat/`). O link da linha 2 é
   `[[tasks/<projeto>/<id>|<id>]]` (caminho vault-relative como alvo, `id` como
   alias) — não invente uma linha `Branch:` separada, e não use `[[id]]` sem
   caminho para tarefas novas (evita colisão entre projetos com o mesmo id).
4. **Linha 3 é um "nome" livre, independente do status** — texto curto que
   você (ou o agente, a pedido do usuário) informa a cada criação/edição do
   callout. Omita a linha inteira se não houver nome. Preserve as linhas 4+
   (impedimentos/notas) ao editar — são texto livre do usuário.
5. **Reaproveite o parser canônico** em vez de regex ad-hoc:
   `require("util.tasks").parse_block(linhas)` →
   `{ status_num, title, id, name, extras }`, e `serialize_block(model)` de
   volta (`model.project` precisa estar setado para o link sair com caminho).
   O par é idempotente (round-trip seguro) e já lida com frontmatter e
   formatos legados.
6. **Não edite as seções `## <projeto>` do `CURRENT.md` à mão** — são geradas.
   Edite os arquivos de tarefa; o painel se atualiza ao salvar. Só
   `## Notas Avulsas` é livre.
7. **Ignore `daily/` e `templates/`** ao varrer projetos, e pule arquivos com
   `archived: true` no frontmatter (`M.is_archived(path)`) — estão arquivados,
   não fazem parte do board ativo.

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
