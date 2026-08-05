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

O topo de cada arquivo de tarefa é um *callout* do Obsidian de 2 a N linhas:

```markdown
> [!todo] Título da task
> [[tasks/bjju-web/id-da-task|id-da-task]]
> _Aguardando aprovação do Fábio_
> impedimento: falta acesso à VPN
```

| Linha | Conteúdo | Papel |
| ----- | -------- | ----- |
| 1 | `> [!<callout>] <título>` | O **tipo do callout** (`todo`, `example`, …) dá ícone/cor **e é a ÚNICA fonte de verdade do status** — não há número redundante em nenhum outro lugar. O título vai em texto simples aqui (sem negrito — o `[!callout]` já destaca a linha). Se o tipo não existir em `status.json`, a task cai em **status 0** (ver seção própria abaixo) — continua editável, só perde ícone/cor. |
| 2 | `> [[tasks/<projeto>/<id>\|<id>]]` | Link do Obsidian para a própria nota, com caminho relativo ao vault (evita ambiguidade entre projetos com o mesmo id) e o `id` como alias; o `id` também é o nome da branch. |
| 3 (opcional) | `> _descrição do estado_` | **Só existe se envolta em itálico** (`_..._`) — é o que identifica a linha como descrição, não a posição. Texto curto e independente do status em si (ex.: "aguardando revisão de X"), digitado a cada edição. **Omitida por completo se vazia** — nunca grave um `>` vazio no meio do bloco: isso faria a nota seguinte ser lida como descrição no próximo parse. |
| 4+ | `> qualquer texto` | Notas/impedimentos, texto livre. **Preservado na íntegra** ao editar. Uma linha em itálico aqui (depois de já haver alguma nota) continua sendo nota, não descrição — o marcador só conta na primeira linha livre do bloco. |

Depois do bloco vem o corpo livre do arquivo (`## Notas Soltas`, checklists…),
que a ferramenta **nunca** reescreve.

**Frontmatter YAML** (`--- id/aliases/tags ---`) que o obsidian.nvim injeta ao
salvar é tolerado: a detecção do callout pula o frontmatter automaticamente.

### Status 0 — callout sem tipo reconhecido

Se a linha 1 tem um `[!tipo]` que não existe em `status.json` (typo, tipo
removido, edição manual malfeita), a task recebe `status_num = 0` em vez de
cair silenciosamente em `todo`. O tipo digitado é **preservado verbatim** no
arquivo (não é substituído nem apagado), para que:

- a task continue abrindo no form via `<leader>oe`/`<C-e>` — status 0 não é um
  beco sem saída;
- o erro fique visível ao reabrir (o form mostra o tipo errado na linha 3,
  pronto pra corrigir);
- no board, a task ordene **antes de tudo no seu projeto** (0 é o menor
  `status_num`), com ícone/hl de erro — sinal de "conserte isso".

Não confundir com `quote`/`cite` (26/27): esses são status **válidos e
deliberados**, para material de referência, e ordenam por último normalmente.

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
> _Ajustando formulário de inscrição_
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

Dentro do picker de tarefas: `<C-t>` nova tarefa (abre o form vazio) · `<C-x>`
deletar · `<C-r>` arquivar · `<C-e>` editar (form) · `<C-o>` voltar. No picker
de projetos: `<C-c>` novo projeto.

O **form** (janela flutuante) é **posicional, sem rótulos no texto** — os
rótulos aparecem como virtual text à direita de cada linha, mas não fazem
parte do conteúdo editável:

```
Título da task                          título
id-da-task                               id / branch
todo _descrição opcional do estado_      tipo de status + descrição
nota livre 1
nota livre 2
```

Linha 1 = título · linha 2 = id (kebab-case; se deixada em branco, é derivada
do título) · linha 3 = `<tipo-de-callout>` seguido, opcionalmente, de
`_descrição_` em itálico · linha 4+ = notas livres.

`⏎` salva, `q`/`<Esc>` cancela, `<C-s>` abre um `vim.ui.select` com os status
disponíveis (troca só o tipo da linha 3, preservando a descrição); a linha 3
também tem completion (`<C-x><C-o>`) para os tipos de callout. **A validação é
estrita**: tipo de callout desconhecido na linha 3, ou menos de 3 linhas, faz o
`⏎` recusar e notificar em vez de fechar — o texto digitado continua no buffer
para corrigir. (O parser do *arquivo* é tolerante — vira status 0 — mas o form
não deixa você criar um status 0 sem querer.)

O mesmo form serve para **criar** (`<C-t>`, buffer vazio com status inicial
`todo`) e **editar** (`<C-e>`/`<leader>oe`, pré-preenchido) — um único formato,
um único parser (`parse_form`), sem dois caminhos para o mesmo dado divergirem.

Editar a partir do `CURRENT.md` grava no arquivo-fonte **e** atualiza o bloco
no painel in place. Renomear o `Id` faz **rename real** do arquivo (`vim.fn.rename`,
não escrever-o-novo-e-apagar-o-velho) — o arquivo e a branch passam a se chamar
pelo novo id; backlinks externos **não** são migrados automaticamente.

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

1. **Criar tarefa**: prefira `require("util.tasks").template(model)` a montar
   o markdown na mão — `model` é `{ status_num, raw_callout, title, id, desc,
   extras, project }`, os mesmos campos que `parse_block` devolve. `id` deve
   ser kebab-case (`M.template` não faz isso por você; use `kebab()` interno ou
   deixe o form derivar do título). O projeto (diretório) precisa existir.
2. **Status é derivado EXCLUSIVAMENTE do tipo do callout da linha 1** — para
   mudar o status, troque o callout (`[!todo]`→`[!success]`). Se o tipo não
   existir em `status.json`, a task vira status 0 (ver seção "Status 0"
   acima) em vez de falhar — mas isso normalmente indica um typo, não é o
   caminho recomendado para setar status de propósito.
3. **`id` == nome da branch** (sem `feat/`). O link da linha 2 é
   `[[tasks/<projeto>/<id>|<id>]]` (caminho vault-relative como alvo, `id` como
   alias) — não invente uma linha `Branch:` separada, e não use `[[id]]` sem
   caminho para tarefas novas (evita colisão entre projetos com o mesmo id).
4. **A descrição do estado (linha 3) só existe se estiver em itálico**
   (`_texto_`) — é o marcador, não a posição, que a distingue de uma nota.
   Omita a linha inteira se não houver descrição (nunca grave `>` vazio no
   meio do bloco — isso deslocaria a leitura da primeira nota real). Preserve
   as linhas seguintes (impedimentos/notas) ao editar — são texto livre do
   usuário, e sobrevivem tal como estão.
5. **Reaproveite o parser canônico** em vez de regex ad-hoc:
   `require("util.tasks").parse_block(linhas)` →
   `{ status_num, raw_callout, title, id, desc, extras }`, e
   `serialize_block(model)` de volta (`model.project` precisa estar setado
   para o link sair com caminho). `raw_callout` é o tipo cru digitado —
   relevante só quando `status_num == 0`, para preservar o typo em vez de
   apagá-lo. O par é idempotente (round-trip seguro) e já lida com
   frontmatter e formatos legados (inclusive o "nome" livre pré-migração, que
   vira nota).
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
  `M.rebuild_current()`. Reconhece tanto o vocabulário atual de `status.json`
  quanto títulos legados que já saíram dele (`LEGACY_STATUS_TITLES`, ex.:
  "Backlog", "In Progress") — se `status.json` for reestruturado de novo,
  cheque se essa tabela precisa crescer, senão a migração vira texto-livre
  permanente em vez de status.
- O cache (`M.cache`) guarda o **bloco bruto** de cada task (`entry.block`),
  lido uma única vez por `scan_task`; `rebuild_current` reimprime a partir daí
  em vez de reler os arquivos. Ao adicionar campos ao cache, prefira estender
  `scan_task` a abrir o arquivo de novo em outro lugar.
- Validação headless (sem framework de teste; o repo é config):
  ```bash
  luacheck nvim/lua/util/tasks.lua
  nvim --headless -c 'lua assert(require("util.tasks").parse_block)' -c 'qa!'
  ```
