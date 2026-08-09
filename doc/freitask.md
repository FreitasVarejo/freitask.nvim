# Freitask & Daily Notes (Obsidian)

Como registrar tarefas e o planejamento diário neste setup — pensado tanto para
o usuário quanto para **agentes de IA** que precisem ler/criar notas sem quebrar
as convenções.

Implementação: `nvim/lua/freitask/` (o módulo, dividido por responsabilidade) +
`nvim/lua/plugins/freitask.lua` (spec do plugin). Opera **exclusivamente** sob
`~/ObsidianVault/tasks/`. Para o mapa dos módulos e como testar, ver
[`freitask-internals.md`](freitask-internals.md).

---

## Modelo mental

- **Um arquivo por tarefa**: `~/ObsidianVault/tasks/<projeto>/<id>.md` — ou
  `~/ObsidianVault/tasks/<projeto>/archived/<tipo>/<id>.md` se arquivada.
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
`templates/`. Dentro de um projeto, `archived/` também não é um projeto — é
onde ficam as tarefas arquivadas (ver "Arquivar / desarquivar").

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
que a ferramenta **nunca** reescreve. A única coisa que ela escreve fora do
bloco é o rodapé `## Histórico`, ao arquivar/desarquivar — e ainda assim só
acrescentando uma linha (ver "Arquivar / desarquivar / deletar").

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
| `<leader>oa` | dentro de um arquivo de tarefa | Arquiva (perguntando o tipo) ou desarquiva a tarefa do buffer |
| `<C-a>` | picker de projetos | Regenera e abre o `CURRENT.md` |

Dentro do picker de tarefas: `<C-t>` nova tarefa (abre o form vazio) · `<C-x>`
deletar · `<C-r>` arquivar **ou desarquivar** (a mesma tecla nos dois sentidos,
conforme a tarefa selecionada) · `<C-u>` mostrar/esconder as arquivadas na
lista · `<C-e>` editar (form) · `<C-o>` voltar. No picker de projetos: `<C-c>`
novo projeto.

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
no painel in place. Renomear o `Id` faz **rename real** do arquivo
(`vim.fn.rename`, não escrever-o-novo-e-apagar-o-velho) — o arquivo e a branch
passam a se chamar pelo novo id — e **migra as referências do vault**
(`M.retarget_links`) junto com o `id:` do frontmatter.

Editar uma tarefa **arquivada** funciona normalmente (o form abre com o título
`Editar task (archived/<tipo>)`), inclusive renomear o id — ela continua na
mesma pasta de arquivamento. Trocar o callout de uma tarefa arquivada para um
status "ativo" **não** a desarquiva: mover arquivo como efeito colateral de um
save seria fácil de disparar sem querer. Use `<leader>oa` / `<C-r>`.

### Arquivar / desarquivar / deletar

**Arquivar move o arquivo** para `tasks/<projeto>/archived/<tipo>/<id>.md`. O
**caminho é a única fonte de verdade** do arquivamento — não existe flag no
frontmatter, do mesmo jeito que o status vem só do callout. Como `archived/`
acrescenta dois níveis, o glob do cache (`tasks/*/*.md`) já as ignora sem
código extra.

Os tipos são **três**:

| tipo | quando |
| ---- | ------ |
| `done` | concluída |
| `dropped` | decidi não fazer / parei de tocar |
| `failed` | tentei e não deu |

`done` e `failed` são deriváveis do callout, e por isso o prompt de
arquivamento já vem **pré-selecionado** a partir dele (`M.suggest_archive_type`:
`done`/`success`/`check`/`tip`/`hint` → `done`; `failure`/`fail`/`error`/
`danger`/`missing`/`bug` → `failed`; o resto → `dropped`). `dropped` é a única
decisão que o status não carrega. Não há um quarto balde genérico ("misc") de
propósito: ele viraria o destino de tudo que se arquiva com pressa, sem
distinguir nada — e criar um diretório depois é trivial, esvaziar um cheio não.

- **Arquivar** (`<C-r>` no picker · `<leader>oa` no arquivo ·
  `M.archive_task(path, tipo)`): move o arquivo, reescreve o wikilink da linha 2
  para o novo caminho, registra no rodapé (ver abaixo), migra as referências do
  vault e tira do cache.
- **Desarquivar** (mesmas teclas · `M.unarchive_task(path)`): o caminho inverso.
  A tarefa volta ao cache e reaparece no `CURRENT.md` na regen seguinte.

#### Rodapé `## Histórico`

Arquivar e desarquivar acrescentam uma linha datada ao fim do arquivo, numa
seção `## Histórico` criada na primeira vez:

```markdown
## Histórico

- 2026-08-09 — arquivada em `archived/dropped` (abandonado)
- 2026-08-11 — desarquivada, de volta ao board
- 2026-08-30 — arquivada em `archived/done` (feito)
```

É um **log, não uma linha única**: arquivar → desarquivar → rearquivar é um
ciclo normal, e sobrescrever apagaria justamente o que interessa (quantas vezes,
e quando você mudou de ideia). As glosas em PT-BR são `done` → feito,
`dropped` → abandonado, `failed` → falhou; o nome do diretório vem junto porque
é ele o dado real.

Entradas novas entram no **fim da seção**, não no fim do arquivo — se você
escrever outra `## seção` depois do histórico, ela continua abaixo. Só o
arquivamento escreve ali: renomear o id, editar pelo form ou regenerar o board
**não** acrescentam linha. O rodapé também não vaza para o `CURRENT.md`, que
reimprime apenas o bloco de callout.

Esta é a **única** exceção à regra de que a ferramenta não reescreve o corpo do
arquivo — e ela só acrescenta, nunca mexe no que já estava lá.
- **Ver as arquivadas**: `<C-u>` no picker de tarefas alterna a exibição delas
  na mesma lista (prefixadas com `[<tipo>]` e esmaecidas). São lidas do disco
  sob demanda (`M.archived_entries_for`) e **nunca** entram em `M.cache`, para
  que board, regen e o resto do módulo sigam enxergando só o que está ativo.
- **Deletar** (`<C-x>` no picker): pede confirmação e apaga o arquivo do disco
  de vez.

### Referências do Obsidian (`M.retarget_links`)

Arquivar, desarquivar e renomear o id são o **mesmo evento** — "esse arquivo
mudou de endereço" — e por isso passam todos por `M.retarget_links(old, new)`,
que varre o vault inteiro reescrevendo os wikilinks. Cobre `[[alvo]]`,
`[[alvo|alias]]`, embeds `![[...]]` e sufixos `#heading` / `^bloco`; preserva o
alias, exceto quando ele **era** o id antigo (aí acompanha o rename).

O Obsidian resolve `[[foo]]` pelo **basename**, em qualquer pasta do vault.
Daí a assimetria — e é por isso que arquivar sai barato:

| operação | `[[foo]]` | `[[tasks/p/foo\|foo]]` | frontmatter `id:` |
| --- | --- | --- | --- |
| arquivar / desarquivar | intacto | reescrito | intacto |
| renomear id | reescrito | reescrito | reescrito |

Ficam **de fora** da varredura, de propósito:

- `tasks/daily/*.md` — são snapshots de como o board estava naquele dia;
  reescrevê-los falsificaria o histórico.
- `tasks/CURRENT.md` — é regenerado do zero; basta o `rebuild_current` seguinte.

Arquivos abertos num buffer **com alterações não salvas** são pulados e
reportados por nome (mesma postura do `rebuild_current` diante de um
`CURRENT.md` sujo) — atualize-os à mão ou salve e refaça a operação.

---

## CLI (`freitask`) — para quem não é o Neovim

O Neovim **não é o único escritor** deste vault: há o Obsidian (aqui e no
celular, via Syncthing) e agentes de IA com acesso a shell. Nenhum deles vai
chamar uma função Lua dentro do Neovim — vão chamar `mv`. A CLI existe para
que o caminho certo seja o mais fácil:

```bash
freitask archive   <id|caminho> done|dropped|failed
freitask unarchive <id|caminho>
freitask rename    <id|caminho> <novo-id>
freitask list      [--json] [--archived]
freitask doctor    [--fix] [--json] [--quiet]
```

Saídas: `0` ok · `1` inconsistência pendente ou operação falha · `2` uso
inválido. Com `--json`, tanto `list` quanto `doctor` emitem estrutura
parseável — é o contrato para agentes.

Implementação: `vault/.local/bin/freitask` (shim bash) →
`nvim/lua/util/freitask_cli.lua` (shim Lua) → o módulo `freitask`. **Nenhuma regra
é reimplementada em nenhuma das camadas**; um segundo motor em bash ou python
divergiria do Lua em semanas, e aí existiriam duas regras em vez de uma.

## `freitask doctor` — verificação e reparo

Nenhum escritor externo pode ser *obrigado* a usar a CLI. A estratégia,
portanto, não é impedir o desvio — é torná-lo **barulhento e, quando possível,
reparável**.

O que ajuda é que a maior parte dos invariantes é **derivável do caminho** (o
wikilink da linha 2, o `id:` do frontmatter, o estado de arquivamento) e por
isso não precisa ser obedecida: precisa ser regenerada. Só duas coisas se
perdem de verdade:

1. **backlinks externos após um rename** — nada registra que `foo` se chamava
   `bar`;
2. **as entradas de histórico** — são fatos sobre o passado.

Repare que **arquivar não está nessa lista**: o move preserva o basename, o
Obsidian resolve `[[foo]]` por basename e o link da linha 2 é derivável. Um
agente que só faz `mv` para `archived/` causa dano **inteiramente reparável**.

| check | nível | `--fix` |
| --- | --- | --- |
| `sync-conflict` — `*.sync-conflict-*.md` do Syncthing | error | **nunca** (decisão editorial) |
| `link-pendurado` — `[[tasks/…]]` que não resolve | error | não (não é adivinhável) |
| `sem-callout` — arquivo de task sem bloco | error | não |
| `bloco-dessincronizado` — bloco ≠ caminho | warn | sim |
| `frontmatter-id` — `id:` ≠ nome do arquivo | warn | sim |
| `historico-ausente` — em `archived/` sem log | warn | sim, marcado como reconstruído |
| `fora-do-padrao` — invisível ao freitask | warn | não |
| `id-duplicado` — `[[id]]` fica ambíguo | warn | não |
| `status-0` — callout fora do `status.json` | warn | não |

Duas decisões deliberadas no detector de links pendurados, ambas contra alarme
falso — **um checker em que não se confia é um checker que não se lê**:

- Só links **path-qualified** (`[[tasks/…]]`) são cobrados. Um `[[nota-futura]]`
  curto que não resolve é comportamento **normal** do Obsidian (vira um
  placeholder que cria a nota ao ser clicado).
- Wikilinks dentro de **código** (cercado por ``` ou entre crases) são
  ignorados, porque o Obsidian também não os renderiza — sem isso, qualquer
  documentação que *cite* um link viraria achado.

O `--fix` de histórico ausente **não inventa data**: escreve a linha marcada
com `[reconstruído pelo doctor; data real desconhecida]`. Mentir sobre a data
seria pior que não ter a linha.

`freitask doctor --quiet` é consumido por `vault/hooks/check.sh`, então
`./healthcheck.sh` já reporta a integridade do vault junto com o resto.

## Rede de proteção: checkpoints git

O vault é sincronizado por Syncthing e **não tinha undo** (o versionamento do
Syncthing está desligado). O pacote `vault/` adiciona um repositório git cujo
`GIT_DIR` fica **fora** da pasta sincronizada:

```
~/.local/state/obsidian-vault.git/   metadata (objects, refs, index)
~/ObsidianVault/                     worktree — nenhum arquivo de git dentro
```

Sincronizar `.git/` entre dispositivos corromperia o repo (refs e index mudam
sem transação e o Syncthing os replica arquivo a arquivo). Mantendo a metadata
fora, as duas ferramentas **nunca tocam o mesmo byte** e o conflito deixa de
ser possível por construção — não por configuração que é preciso lembrar de
manter.

`vault-checkpoint.timer` commita a cada 15 min e sai em silêncio quando nada
mudou. Você nunca commita à mão; commits são checkpoints de máquina. Para
inspecionar ou desfazer, use o wrapper `vaultgit`:

```bash
vaultgit log --oneline -- tasks/jellyfin/stack-up.md
vaultgit restore --source=HEAD~3 tasks/jellyfin/stack-up.md
```

Cada máquina tem sua própria história — é um log de undo local, não
sincronização (quem sincroniza é o Syncthing) nem histórico compartilhado.

---

## Para agentes de IA — como criar/editar tarefas sem quebrar nada

> O contrato curto e imperativo vive em `~/ObsidianVault/AGENTS.md`, junto dos
> dados — é a pasta para a qual um agente externo é apontado. Esta seção é a
> versão longa.

1. **Criar tarefa**: prefira `require("freitask").template(model)` a montar
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
   `require("freitask").parse_block(linhas)` →
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
7. **Ignore `daily/` e `templates/`** ao varrer projetos, e pule
   `tasks/*/archived/**` — estão arquivadas, não fazem parte do board ativo.
   O estado de arquivamento é o **caminho**, não uma flag de frontmatter; use
   `M.split_task_path(path)` → `project, id, archived` como guarda único (ele
   devolve `nil` para qualquer coisa fora de padrão) ou `M.is_archived(path)`.
8. **Nunca mova nem renomeie um arquivo de tarefa na mão** — use
   `M.archive_task` / `M.unarchive_task` / `M.apply_edit`. Os três chamam
   `M.retarget_links`, sem a qual os wikilinks do vault ficam pendurados; o
   wikilink da linha 2 é path-qualified e precisa ser reescrito junto; e
   arquivar/desarquivar ainda registram a linha datada em `## Histórico`.
9. **Não escreva em `## Histórico` à mão** para marcar arquivamento — quem
   escreve ali é `M.archive_task`/`M.unarchive_task`, e a fonte de verdade do
   estado continua sendo o **caminho**. Uma linha de histórico sem o move
   correspondente é só um texto mentindo sobre onde o arquivo está.

---

## Manutenção / verificação

- O código vive em `nvim/lua/freitask/`, um módulo por responsabilidade — ver
  [`freitask-internals.md`](freitask-internals.md) para o mapa, as regras de
  dependência e o roteiro de verificação. `freitask.model`
  (`parse_block`/`serialize_block`) é a fonte única do formato: mudanças de
  formato começam aí. `nvim/lua/util/freitask.lua` (fachada),
  `nvim/lua/util/freitask_cli.lua` e `vault/.local/bin/freitask` são shims sem
  lógica: **não** acrescente regra neles, senão passam a existir duas versões
  da mesma regra.
- `links.retarget_links` é chamada por `task.archive_task`,
  `task.unarchive_task` e `edit.apply_edit` (rename). Qualquer caminho novo que mova ou renomeie um
  arquivo de task precisa chamá-la também — é o único ponto que conserta as
  referências do vault.
- `doctor.migrate_format()` converte arquivos de formatos antigos para o atual
  (idempotente); rode-o após qualquer mudança de formato, seguido de
  `board.rebuild_current()`. Também migra o **formato legado de arquivamento**
  (`archived: true` no frontmatter): move essas tarefas para
  `archived/dropped/` — o balde honesto, já que o flag antigo não registrava
  o porquê — e remove a linha. Reconhece tanto o vocabulário atual de `status.json`
  quanto títulos legados que já saíram dele (`LEGACY_STATUS_TITLES`, ex.:
  "Backlog", "In Progress") — se `status.json` for reestruturado de novo,
  cheque se essa tabela precisa crescer, senão a migração vira texto-livre
  permanente em vez de status.
- `path.split_task_path` é o **guarda único** de "isto é um arquivo de task?" —
  cache, autocmds, form e picker passam por ele. Ao acrescentar um formato de
  caminho, mude-o lá e não com um `match` novo no ponto de uso.
- O cache (`cache.cache`) guarda o **bloco bruto** de cada task (`entry.block`),
  lido uma única vez por `scan_task`; `rebuild_current` reimprime a partir daí
  em vez de reler os arquivos. Ao adicionar campos ao cache, prefira estender
  `scan_task` a abrir o arquivo de novo em outro lugar.
- Validação:
  ```bash
  nvim/tests/run.sh        # a suíte (funções puras: parser, links, kebab…)
  cd nvim && luacheck lua/freitask lua/util lua/plugins/freitask.lua tests
  shellcheck -x -P SCRIPTDIR vault/hooks/*.sh vault/.local/bin/*
  freitask doctor          # integridade dos DADOS (o resto checa o CÓDIGO)
  ./healthcheck.sh         # inclui o doctor via vault/hooks/check.sh
  ```
  As operações de **escrita** e a UI não têm teste automatizado; o roteiro
  manual (vault descartável via `$HOME`) está em
  [`freitask-internals.md`](freitask-internals.md).
- O `doctor` é a rede de segurança contra escritores que não passam por este
  módulo (Obsidian, celular, agentes). Ao acrescentar um invariante ao formato,
  acrescente **junto** o check correspondente — um invariante que só existe na
  documentação é um invariante que vai ser violado em silêncio.
