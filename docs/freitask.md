# Freitask & Daily Notes (Obsidian)

Como registrar tarefas e o planejamento diário neste setup — pensado tanto para
o usuário quanto para **agentes de IA** que precisem ler/criar notas sem quebrar
as convenções.

Implementação, neste repositório: o **motor** em `lua/freitask/` (dividido por
responsabilidade), que a CLI `freitask` roda sob `nvim -l`, e a **TUI** em
`tui/` (Rust), que fala com o motor só pela CLI. O Neovim não carrega nada
daqui: o dotfiles só abre a TUI num float (`<leader>k`) e roda
`freitask rebuild` ao salvar uma task. Opera **exclusivamente** sob
`~/ObsidianVault/projects/<projeto>/tasks/`. Para o mapa dos módulos e como
testar, ver
[`freitask-internals.md`](freitask-internals.md).

---

## Modelo mental

- **Um arquivo por tarefa**: `~/ObsidianVault/projects/<projeto>/tasks/<id>.md`
  — ou `~/ObsidianVault/projects/<projeto>/tasks/archived/<tipo>/<id>.md` se
  arquivada.
- **Projeto = subdiretório de `projects/` que tem um `tasks/` dentro** (id
  kebab-case, ex.: `bjju-web`). A presença da pasta é o registro: não há lista
  de projetos em lugar nenhum. As tasks moram junto da spec e das `decisoes/`
  do mesmo projeto, e não numa árvore própria longe do porquê que as justifica.
- **`id` da tarefa = nome do arquivo = nome da branch git.** Não existe campo
  "branch" separado — os dois são a mesma coisa (sem prefixo `feat/`).
- **`CURRENT.md`** é o painel diário, **gerado automaticamente** — não edite as
  seções de projeto à mão (só a seção `## Notas Avulsas` é preservada).
- **`status.json`** define os status (número → callout/título/ícone/cor). Mora
  em `.freitask/`, que é configuração e não conteúdo — pasta oculta para o
  Obsidian não a listar junto das notas.

Diretórios **reservados** (não são projetos): `daily/` (arquivo diário) e
`templates/`. Dentro de um projeto, `archived/` também não é um projeto — é
onde ficam as tarefas arquivadas (ver "Arquivar / desarquivar").

---

## Formato do bloco de uma tarefa

O topo de cada arquivo de tarefa é um *callout* do Obsidian de 2 a N linhas:

```markdown
> [!todo] Título da task
> [[projects/bjju-web/tasks/id-da-task|id-da-task]]
> _Aguardando aprovação do Fábio_
> impedimento: falta acesso à VPN
```

| Linha | Conteúdo | Papel |
| ----- | -------- | ----- |
| 1 | `> [!<callout>] <título>` | O **tipo do callout** (`todo`, `example`, …) dá ícone/cor **e é a ÚNICA fonte de verdade do status** — não há número redundante em nenhum outro lugar. O título vai em texto simples aqui (sem negrito — o `[!callout]` já destaca a linha). Se o tipo não existir em `status.json`, a task cai em **status 0** (ver seção própria abaixo) — continua editável, só perde ícone/cor. |
| 2 | `> [[projects/<projeto>/tasks/<id>\|<id>]]` | Link do Obsidian para a própria nota, com caminho relativo ao vault (evita ambiguidade entre projetos com o mesmo id) e o `id` como alias; o `id` também é o nome da branch. |
| 3 (opcional) | `> _descrição do estado_` | **Só existe se envolta em itálico** (`_..._`) — é o que identifica a linha como descrição, não a posição. Texto curto e independente do status em si (ex.: "aguardando revisão de X"), digitado a cada edição. **Omitida por completo se vazia** — nunca grave um `>` vazio no meio do bloco: isso faria a nota seguinte ser lida como descrição no próximo parse. |
| 4+ | `> qualquer texto` | Notas/impedimentos, texto livre. **Preservado na íntegra** ao editar. Uma linha em itálico aqui (depois de já haver alguma nota) continua sendo nota, não descrição — o marcador só conta na primeira linha livre do bloco. |

Depois do bloco vem o corpo livre do arquivo (`## Notas Soltas`, checklists…),
que a ferramenta **nunca** reescreve. A única coisa que ela escreve fora do
bloco é o rodapé `## Histórico`, ao arquivar/desarquivar — e ainda assim só
acrescentando uma linha (ver "Arquivar / desarquivar / deletar").

**Frontmatter YAML** é pulado pela detecção do callout, e hoje carrega a
prioridade (abaixo). Chaves que o freitask não conhece — as que o obsidian.nvim
injeta, como `aliases`/`tags` — são **preservadas verbatim**: ele nunca reescreve
linha que não seja sua.

### Prioridade (`prioridade`)

Um campo **opcional** no frontmatter, ortogonal ao status: o callout diz em que
**fase** o trabalho está, este diz se ele passa na frente.

```yaml
---
prioridade: alta
---
```

Uma marca só: `alta` é o único valor que conta, e sem ela a task é normal.
Três níveis acabam com tudo em "média". As altas vêm **primeiro** na TUI e no
`CURRENT.md`, qualquer que seja a fase (a fase continua visível pelo ícone);
dentro de cada grupo a ordem é a da fase e depois a do id.

Mora no frontmatter, e não no bloco, por dois motivos: é metadado consultável
(o Obsidian indexa propriedade, não texto de blockquote), e o bloco é
reserializado inteiro pelo doctor a partir do **caminho** — prioridade não é
derivável de caminho nenhum e seria regenerada para fora. O celular o edita
como edita o callout.

**A escrita é manual, de propósito.** Não há subcomando para gravá-la: o
freitask só cresce por dor medida, e a de gravar à mão ainda não apareceu. O
que é automático é a vigilância: `prioridade-desconhecida` no `doctor` acusa
qualquer valor diferente de `alta` (`Alta`, `urgente`), que de outro modo
viraria "normal" em silêncio.

**Refino e implementação da mesma task nunca rodam em paralelo.** Task em Em
implementação tem o contrato congelado; refino que chegue no meio vira **task
nova**, nunca edição da que está em voo.

### Status 0 — callout sem tipo reconhecido

Se a linha 1 tem um `[!tipo]` que não existe em `status.json` (typo, tipo
removido, edição manual malfeita), a task recebe `status_num = 0` em vez de
cair silenciosamente em `todo`. O tipo digitado é **preservado verbatim** no
arquivo (não é substituído nem apagado), para que:

- a task continue consertável — `s` na TUI (ou `freitask set <id> --status
  <fase>`) troca o tipo; status 0 não é um beco sem saída;
- o erro fique visível: `freitask list --json` devolve o tipo errado em
  `callout`, e a TUI mostra a task com o ícone de erro;
- no board, a task ordene **antes das normais do seu projeto** (0 é o menor
  `status_num`; só as de `prioridade: alta` passam à frente), com ícone/hl de
  erro — sinal de "conserte isso".

Um callout do Obsidian que não é fase (um `[!note]` ou `[!info]` no corpo da
nota) **não** cai aqui: o status vem só do primeiro bloco, e o resto do arquivo
o freitask nem olha.

### Status (`~/ObsidianVault/.freitask/status.json`)

Lista as fases, ordenadas como uma narrativa de workflow (a ordem/número só
define a **ordenação** das tasks no board — o status em si vem do nome do
callout):

| nº | callout | título | significa |
| -- | ------- | ------ | --------- |
| 1 | `todo` | Não iniciada | ninguém começou |
| 2 | `abstract` | Refinando | o desenho está sendo feito; o contrato ainda se move |
| 3 | `tip` | Refinada | o desenho está fechado e pronto para implementar: a fronteira entre pensar (no celular) e codar (no notebook) |
| 4 | `example` | Em implementação | o contrato está congelado e alguém está escrevendo código |
| 5 | `question` | Em homologação | escrito, em teste/validação |
| 6 | `warning` | Bloqueada | travada por algo externo; **o porquê vai na linha 4+ do bloco** |
| 7 | `check` | Pronta | acabou, falta arquivar |
| 8 | `quote` | Algum dia | backlog que o dono aceita esquecer por anos, mas quer poder lembrar; **escondida por padrão** |
| 9 | `done` | Arquivada | é o que `archive` grava |

São **as fases mais `done`**, e não os 27 callouts que o render-markdown
suporta. O vocabulário largo não estava sendo escolhido — 36 tasks ativas,
todas em `todo` — e status que ninguém escolhe não informa nada: a
diferenciação real terminava na frase em itálico, que é texto livre e não se
consulta. Cada fase existe porque alguém precisa distingui-la de outra para não
atropelá-la.

**Algum dia é escondida por padrão** (`HIDDEN_CALLOUTS` em `config.lua`, e
`hidden` no `list --json`): o `CURRENT.md` não a imprime nem a conta, e a TUI só
a mostra com `S`. Continua em `freitask list`, para quem precisa achá-la. É a
última fase que aparece na ordem; `done` só existe em arquivada.

Os callouts de Refinada (`tip`) e Algum dia (`quote`) saem de famílias que o
Obsidian do celular não confunde com as outras: ele renderiza igual os tipos da
mesma família (`check`/`done`/`success`, `question`/`help`/`faq`,
`warning`/`caution`/`attention`, `abstract`/`summary`/`tldr`).

Os nomes honram o vocabulário **antigo** deste plugin (`LEGACY_STATUS_TITLES`):
`example` já era "In Progress", `question` já era "Review", `warning` já era
"Blocked".

Sair do `status.json` **não** tira um callout do Obsidian: `note`, `info` e os
demais seguem renderizando no corpo das notas. Este arquivo governa só como o
freitask lê o **primeiro bloco** de uma task.

Tasks novas (`M.template`) começam em `1` (`todo`).

O arquivo só é criado quando falta: um vault que já tinha o `status.json` de
antes de Refinada e Algum dia precisa ganhar essas duas linhas à mão (copie de
`DEFAULT_STATUS_JSON` em `config.lua`). Sem elas, a task em `[!tip]` ou
`[!quote]` vira status 0, nada é escondido, e o `doctor` acusa o callout que
não existe.

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
> [[projects/bjju-web/tasks/criar-jiujitsu-unicamp|criar-jiujitsu-unicamp]]
> _Ajustando formulário de inscrição_
```

- **Auto-regeneração**: salvar qualquer arquivo de tarefa (`BufWritePost`)
  regenera o `CURRENT.md` silenciosamente — tarefas novas/mudanças de status
  aparecem sozinhas. Se o `CURRENT.md` estiver com edições não salvas, a regen é
  pulada (sem sobrescrever seu trabalho).
- **Arquivo diário**: ao regenerar num dia novo, o painel do dia anterior é
  snapshotado em `daily/YYYY-MM-DD.md` antes de ser recarimbado com a data
  de hoje.
- **`## Notas Avulsas`** é a única seção editável à mão que sobrevive à regen.
- **Ordem e o que não entra**: dentro de cada projeto, as tasks de prioridade
  alta vêm primeiro e depois a ordem da fase; as de Algum dia não são impressas
  e não entram na contagem do cabeçalho.

---

## Uso interativo (TUI)

`freitask` sem argumentos, num terminal, abre a TUI (`freitask-tui`); no
Neovim, `<leader>k` a abre num float. São três painéis: projetos, as tarefas
do projeto (ícone e cor da fase, e um `!` nas de prioridade alta) e o preview
do arquivo.

| Tecla | Ação |
| ----- | ---- |
| `n` | nova tarefa no projeto: título → id (sugerido pelo motor a partir do título, editável) → descrição; nasce `todo` |
| `s` | trocar a fase |
| `t` / `i` | editar o título / a descrição do estado |
| `r` | renomear o id |
| `a` | arquivar (o tipo sugerido vem pré-selecionado) ou, numa arquivada, desarquivar |
| `A` | mostrar / esconder as arquivadas |
| `S` | mostrar / esconder as de Algum dia |
| `d` | largar: arquiva como `dropped` (pede `y`) |
| `N` | novo projeto |
| `c` | regenerar e abrir o `CURRENT.md` |
| `/` | filtrar por id ou título |
| `Enter` / `e` | abrir a tarefa no editor |
| `?` | todas as teclas |

A TUI **não conhece regra de tarefa**: cada tecla vira um subcomando da CLI
(`set`, `rename`, `archive`…), e quando o motor recusa, a mensagem dele aparece
na barra de baixo e o modal volta preenchido para corrigir. As **notas**
(linhas 4+) se editam no editor (`e`) — texto livre não cabe num campo.

Abrir uma tarefa com a TUI **dentro do Neovim** a abre no Neovim pai (via
`$NVIM`) e fecha o float, como o lazygit; fora dele, roda `$VISUAL`/`$EDITOR`
em primeiro plano e regenera o `CURRENT.md` na volta. Salvar uma tarefa no
Neovim roda `freitask rebuild` (autocmd do dotfiles).

Renomear o id faz **rename real** do arquivo (`vim.fn.rename`, não
escrever-o-novo-e-apagar-o-velho) e **migra as referências do vault**
(`M.retarget_links`) junto com o `id:` do frontmatter. O id novo passa pela
mesma validação de uma tarefa nova — kebab-case e livre, inclusive em
`archived/` —, porque ele é também o nome da branch.

Editar uma tarefa **arquivada** funciona normalmente, inclusive renomear o id
— ela continua na mesma pasta de arquivamento. Trocar a fase de uma arquivada
**não** a desarquiva: mover arquivo como efeito colateral de uma edição seria
fácil de disparar sem querer. Quem move é `a` / `freitask unarchive`. E `done`
não se escolhe no `s`: é o que `archive <id> done` grava junto com o move.

### Arquivar / desarquivar / deletar

**Arquivar move o arquivo** para `projects/<projeto>/tasks/archived/<tipo>/<id>.md`,
e **a fase acompanha a pasta**: arquivar como `done` grava o callout `done`, e
desarquivar uma task `done` a devolve como `check` (Pronta). Só `done` tem essa
regra, porque só ele tem callout — `dropped` e `failed` são destinos sem fase
correspondente. O
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

O tipo de arquivamento já vem **pré-selecionado** a partir do callout
(`M.suggest_archive_type`, exposto como `suggested_archive` no
`list --json`: `done`/`check` → `done`; o resto → `dropped`). `failed` não tem
default desde que as fases encolheram — nenhuma delas significa
"falhou" — e se escolhe à mão. `dropped` é a única
decisão que o status não carrega. Não há um quarto balde genérico ("misc") de
propósito: ele viraria o destino de tudo que se arquiva com pressa, sem
distinguir nada — e criar um diretório depois é trivial, esvaziar um cheio não.

- **Arquivar** (`a` na TUI · `freitask archive <id> <tipo>` ·
  `M.archive_task(path, tipo)`): move o arquivo, reescreve o wikilink da linha 2
  para o novo caminho, registra no rodapé (ver abaixo), migra as referências do
  vault e tira do cache.
- **Desarquivar** (`a` numa arquivada · `freitask unarchive <id>` ·
  `M.unarchive_task(path)`): o caminho inverso.
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
arquivamento escreve ali: renomear o id, editar (`set`) ou regenerar o board
**não** acrescentam linha. O rodapé também não vaza para o `CURRENT.md`, que
reimprime apenas o bloco de callout.

Esta é a **única** exceção à regra de que a ferramenta não reescreve o corpo do
arquivo — e ela só acrescenta, nunca mexe no que já estava lá.
- **Ver as arquivadas**: `A` na TUI alterna a exibição delas na mesma lista
  (prefixadas com `[<tipo>]` e esmaecidas); `freitask list --archived` as
  inclui. São lidas do disco
  sob demanda (`M.archived_entries_for`) e **nunca** entram em `M.cache`, para
  que board, regen e o resto do módulo sigam enxergando só o que está ativo.
- **Largar** (`d` na TUI, que pede `y`): é arquivar como `dropped`. O arquivo
  fica e os links acompanham, como em qualquer arquivamento. Numa task já
  arquivada a tecla recusa: reclassificar é `a`, desarquivando antes.
- **Deletar** (`freitask delete <id>`, só na CLI, que não pergunta): apaga o
  arquivo do disco de vez. As referências a ela ficam penduradas de propósito,
  e o `doctor` as aponta (`link-pendurado`).

### Referências do Obsidian (`M.retarget_links`)

Arquivar, desarquivar e renomear o id são o **mesmo evento** — "esse arquivo
mudou de endereço" — e por isso passam todos por `M.retarget_links(old, new)`,
que varre o vault inteiro reescrevendo os wikilinks. Cobre `[[alvo]]`,
`[[alvo|alias]]`, embeds `![[...]]` e sufixos `#heading` / `^bloco`; preserva o
alias, exceto quando ele **era** o id antigo (aí acompanha o rename).

O Obsidian resolve `[[foo]]` pelo **basename**, em qualquer pasta do vault.
Daí a assimetria — e é por isso que arquivar sai barato:

| operação | `[[foo]]` | `[[projects/p/tasks/foo\|foo]]` | frontmatter `id:` |
| --- | --- | --- | --- |
| arquivar / desarquivar | intacto | reescrito | intacto |
| renomear id | reescrito | reescrito | reescrito |

Ficam **de fora** da varredura, de propósito:

- `daily/*.md` — são snapshots de como o board estava naquele dia;
  reescrevê-los falsificaria o histórico.
- `CURRENT.md` (na raiz do vault) — é regenerado do zero; basta o
  `rebuild_current` seguinte.

A CLI roda **fora** do seu Neovim e não enxerga os buffers dele: uma nota
aberta e com alterações não salvas que referencia a tarefa é reescrita no
disco mesmo assim. O Neovim avisa (W12) quando você for salvá-la — recarregue
(`:e`) antes; se salvar por cima, o `doctor` acusa o link pendurado.

---

## CLI (`freitask`)

A CLI é a **única porta para o motor**: a TUI passa por ela, e também os
outros escritores deste vault — o Obsidian (aqui e no celular, via Syncthing)
e agentes de IA com acesso a shell. Nenhum deles vai chamar uma função Lua; sem
a CLI, chamariam `mv`. Ela existe para que o caminho certo seja o mais fácil:

```bash
freitask new         <projeto> <id> "<título>" [--desc "<estado>"]
freitask set         <id|caminho> [--status <fase>] [--title "…"] [--desc "…"]
freitask archive     <id|caminho> done|dropped|failed
freitask unarchive   <id|caminho>
freitask rename      <id|caminho> <novo-id>
freitask delete      <id|caminho>
freitask new-project <nome>
freitask list        [--json] [--archived]
freitask projects    [--json]
freitask statuses    [--json]
freitask kebab       "<texto>"
freitask rebuild
freitask doctor      [--fix] [--json] [--quiet]
```

Argumentos e o formato exato de cada saída: `freitask help`.

Saídas: `0` ok · `1` inconsistência pendente ou operação falha · `2` uso
inválido. Com `--json`, `list`, `projects`, `statuses` e `doctor` emitem
estrutura parseável — é o contrato para agentes e para a TUI. Os avisos do
motor saem em stderr, uma linha cada; stdout carrega só o dado.

`set` recusa `--status done`: `done` é o que `archive <id> done` grava junto
com o move. `delete` não pergunta nada — quem confirma é quem chama — e deixa
as referências penduradas de propósito, para o `doctor` apontar.

Implementação: `vault/.local/bin/freitask` (shim bash) →
`lua/freitask/cli.lua` (shim Lua) → o módulo `freitask`. **Nenhuma regra
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
| `callout-vs-pasta` — em `archived/done/` com callout ≠ `done` | warn | sim |
| `prioridade-desconhecida` — `prioridade:` diferente de `alta` | warn | não |

`callout-vs-pasta` só vale para `done`: esse status significa literalmente
"Arquivada", e um arquivo em `archived/done/` dizendo "Não iniciada" é
contradição. Em `dropped` e `failed` a fase em que a task estava quando foi
largada é **informação** ("estava bloqueada quando desisti") e fica preservada.

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
vaultgit log --oneline -- projects/<projeto>/tasks/<id>.md
vaultgit restore --source=HEAD~3 projects/<projeto>/tasks/<id>.md
```

Cada máquina tem sua própria história — é um log de undo local, não
sincronização (quem sincroniza é o Syncthing) nem histórico compartilhado.

---

## Para agentes de IA — como criar/editar tarefas sem quebrar nada

> O contrato curto e imperativo vive em `~/ObsidianVault/AGENTS.md`, junto dos
> dados — é a pasta para a qual um agente externo é apontado. Esta seção é a
> versão longa.

1. **Criar tarefa**: `freitask new <projeto> <id> "<título>" [--desc "<estado>"]`.
   É o caminho único onde há shell — ele valida (id kebab-case, projeto não
   reservado, id livre inclusive em `archived/`; projeto novo é criado), serializa o bloco pelo mesmo
   código de todo o resto e **regenera o `CURRENT.md`**. Montar o markdown na
   mão deixa o painel para trás, que era o sintoma antigo: task criada por
   arquivo não aparecia no board até alguém salvar outra task no Neovim.
   Na TUI, `n` chama o mesmo `freitask new`. De dentro do Lua, `require("freitask").create_task(project, model)` —
   `model` é `{ status_num, raw_callout, title, id, desc, extras }`, os mesmos
   campos que `parse_block` devolve.
   Sem shell (o claude.ai pelo conector MCP), escrever o arquivo à mão segue
   sendo a única opção: siga o formato do bloco à risca. A TUI lê os arquivos
   e regenera o `CURRENT.md` ao abrir pelo `c`; `freitask rebuild` é o mesmo
   para quem tem shell.
2. **Status é derivado EXCLUSIVAMENTE do tipo do callout da linha 1** — para
   mudar o status, `freitask set <id> --status <fase>`, ou troque o callout à
   mão (`[!todo]`→`[!example]`). Se o tipo não
   existir em `status.json`, a task vira status 0 (ver seção "Status 0"
   acima) em vez de falhar — mas isso normalmente indica um typo, não é o
   caminho recomendado para setar status de propósito.
3. **`id` == nome da branch** (sem `feat/`). O link da linha 2 é
   `[[projects/<projeto>/tasks/<id>|<id>]]` (caminho vault-relative como alvo, `id` como
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

- O código vive em `lua/freitask/`, um módulo por responsabilidade — ver
  [`freitask-internals.md`](freitask-internals.md) para o mapa, as regras de
  dependência e o roteiro de verificação. `freitask.model`
  (`parse_block`/`serialize_block`) é a fonte única do formato: mudanças de
  formato começam aí. `lua/freitask/cli.lua` e o executável
  `vault/.local/bin/freitask` (no dotfiles) são shims sem lógica: **não**
  acrescente regra neles, senão passam a existir duas versões da mesma regra.
- `links.retarget_links` é chamada por `task.archive_task`,
  `task.unarchive_task` e `task.apply_edit` (rename). Qualquer caminho novo que mova ou renomeie um
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
  cache, doctor e CLI passam por ele. Ao acrescentar um formato de
  caminho, mude-o lá e não com um `match` novo no ponto de uso.
- O cache (`cache.cache`) guarda o **bloco bruto** de cada task (`entry.block`),
  lido uma única vez por `scan_task`; `rebuild_current` reimprime a partir daí
  em vez de reler os arquivos. Ao adicionar campos ao cache, prefira estender
  `scan_task` a abrir o arquivo de novo em outro lugar.
- Validação:
  ```bash
  tests/run.sh        # funções puras + a CLI de ponta a ponta
  (cd tui && cargo test)   # estado e teclas da TUI
  luacheck lua tests
  shellcheck -x tests/run.sh
  freitask doctor          # integridade dos DADOS (o resto checa o CÓDIGO)
  ./healthcheck.sh         # inclui o doctor via vault/hooks/check.sh
  ```
  As operações de **escrita** são cobertas por `tests/cli_spec.lua`, num vault
  descartável (ver [`freitask-internals.md`](freitask-internals.md)); o que
  segue manual é o `doctor --fix` e o desenho da TUI.
- O `doctor` é a rede de segurança contra escritores que não passam por este
  módulo (Obsidian, celular, agentes). Ao acrescentar um invariante ao formato,
  acrescente **junto** o check correspondente — um invariante que só existe na
  documentação é um invariante que vai ser violado em silêncio.
