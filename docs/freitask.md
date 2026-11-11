# Freitask & Daily Notes (Obsidian)

Como registrar tarefas e o planejamento diário neste setup — pensado tanto para
o usuário quanto para **agentes de IA** que precisem ler/criar notas sem quebrar
as convenções.

Implementação, neste repositório: **um binário Rust**, `freitask`, que é a CLI e
a TUI (workspace na raiz, `crates/`). As regras moram num núcleo puro; o binário
só traduz argumentos e teclas em chamadas a ele. O Neovim não carrega nada
daqui: o dotfiles só abre a TUI num float (`<leader>k`) e roda
`freitask rebuild` ao salvar uma task. Opera **exclusivamente** sob
`~/ObsidianVault/projects/<projeto>/tasks/`; só encerrar e reabrir um projeto
movem a pasta `projects/<projeto>/` inteira. Para o mapa do código e como
testar, ver [`freitask-internals.md`](freitask-internals.md).

---

## Modelo mental

- **Um arquivo por tarefa**: `~/ObsidianVault/projects/<projeto>/tasks/<id>.md`
  — ou `~/ObsidianVault/projects/<projeto>/tasks/archived/<tipo>/<id>.md` se
  arquivada.
- **Projeto = subdiretório de `projects/` que tem um `tasks/` dentro** (id
  kebab-case, ex.: `bjju-web`). A presença da pasta é o registro: não há lista
  de projetos em lugar nenhum. As tasks moram junto da spec e das `decisoes/`
  do mesmo projeto, e não numa árvore própria longe do porquê que as justifica.
- **Projeto encerrado = `projects/_encerrados/<projeto>/`**: fora do painel e de
  toda listagem, com o caminho como único estado (ver "Encerrar / reabrir um
  projeto").
- **`id` da tarefa = nome do arquivo = nome da branch git.** Não existe campo
  "branch" separado — os dois são a mesma coisa (sem prefixo `feat/`).
- **`CURRENT.md`** é o painel diário, **gerado automaticamente** — não edite as
  seções de projeto à mão (só a seção `## Notas Avulsas` é preservada).
- **`status.json`** define os status (número → callout/título/ícone/cor). Mora
  em `.freitask/`, que é configuração e não conteúdo — pasta oculta para o
  Obsidian não a listar junto das notas.

Diretórios **reservados** (não são projetos): `daily/` (arquivo diário),
`templates/` e `_encerrados/` (onde moram os projetos encerrados). Dentro de um
projeto, `archived/` também não é um projeto — é
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
bloco é o rodapé `## Histórico`, ao arquivar/desarquivar, ao mover de projeto
e ao renomear o id — e ainda assim só acrescentando uma linha (ver "Arquivar /
desarquivar / deletar", "Mover para outro projeto" e "Rodapé `## Histórico`").

**Frontmatter YAML** é pulado pela detecção do callout, e carrega a
prioridade, a data de volta, o prazo e as dependências (abaixo). Chaves que o freitask não conhece — as
que o obsidian.nvim injeta, como `aliases`/`tags` — são **preservadas
verbatim**: ele nunca reescreve linha que não seja sua.

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

### Volta em (`volta-em`) — a task adiada

Um campo **opcional** no frontmatter, ortogonal ao status e à prioridade: a fase
não muda, e o campo diz até que dia a task fica **fora da tela**. Não é
arquivar — arquivada fecha uma frente e muda de caminho; a adiada segue viva em
`tasks/`, na mesma fase.

```yaml
---
volta-em: 2026-12-01
---
```

- **É o primeiro dia visível.** A task fica escondida até a véspera e, no dia da
  data, já aparece: ela volta à meia-noite local. Só o dia conta, sem hora.
- **Enquanto a data não chega, vale o tratamento de Algum dia:** fora do
  `CURRENT.md` e da contagem do cabeçalho, escondida na TUI até o `S` (que a
  mostra com a data de volta ao lado), e à mostra em `freitask list` (o JSON traz
  `hidden: true` e, em `volta_em`, o dia em que ela volta).
- **Vale em qualquer fase ativa** e vence `prioridade: alta` enquanto a data não
  chega; a prioridade só decide a ordem do que está à mostra.
- **É lida na hora e nunca gravada.** O freitask compara a data com o dia de hoje a
  cada leitura, e quando a data chega nada grava nada na task. Por isso não há
  job que possa falhar e deixá-la escondida (nem timer: duas máquinas gravariam
  o mesmo arquivo sob o Syncthing). O preço é o painel: o `CURRENT.md` só muda
  quando algo o regenera (`freitask rebuild`, a TUI ao abrir, uma escrita da CLI,
  salvar uma task no Neovim), e num dia sem nenhum desses ele fica atrasado —
  isso é aceito.
- **Depois da data** o campo fica no arquivo, sem efeito e sem achado no
  `doctor`. Adiar de novo é trocar a data; uma task com várias datas troca o
  campo a cada passo.
- **Só uma data.** Condição além do dia e ordem entre tasks não moram aqui.
- **A escrita é `freitask set <id> --volta-em AAAA-MM-DD`**, e `""` limpa o
  campo; o comando recusa o que não for uma data, pela regra do item seguinte.
  À mão também vale, para o celular e o claude.ai. A TUI não tem tecla para
  adiar.
- **A data é uma data.** Só `AAAA-MM-DD` de um dia que existe vale. `2026-12-1`,
  `01/12/2026` e `2026-02-30` não são datas e **não escondem a task**: a
  comparação é de data, não de texto (como texto, `2026-12-1` ficaria maior que
  `2026-12-05`), e esconder por erro de digitação faria a task sumir sem aviso.
  Em vez disso a task aparece e o `doctor` acusa `volta-em-invalido`, sem
  `--fix`: o erro tem de ficar visível. Campo vazio conta como sem data, e
  arquivada não é adiada, então não há o que acusar nela.

Mora no frontmatter pelos mesmos motivos da prioridade: é metadado consultável,
o celular o edita, e o bloco é reserializado a partir do caminho, que não
carrega data nenhuma.

### Dependências (`bloqueada-por`)

Um campo **opcional** no frontmatter da task que **espera** outra, ortogonal à
fase: a fase diz em que pé o trabalho está, este diz do que ele depende.

```yaml
---
bloqueada-por: [id-a, id-b]
---
```

- **Só ids, numa linha.** Lista YAML em uma linha, e cada elemento é o `id` de
  uma task (o nome do arquivo, como em todo lugar). É a forma que o parser de
  frontmatter entende e que o celular escreve à mão.
- **Só o lado que espera é gravado.** O lado inverso (`bloqueia`: quem espera
  esta task) nunca vai para o arquivo; é derivado na leitura. Gravar nos dois
  lados dobraria as escritas sob o Syncthing e abriria a porta para os dois
  discordarem.
- **A escrita é `freitask set <id> --bloqueada-por "a,b"`**, que **substitui a
  lista inteira**: a vírgula separa, os espaços são aparados, repetidos saem, e
  `""` limpa (a chave some, e o frontmatter some junto se era só ela). O
  comando recusa — saída `1`, mensagem em stderr, nada gravado — o id que não
  existe entre as tasks ativas nem entre as arquivadas, o id que existe em mais
  de um projeto (ambíguo), a própria task, e a lista que fecha um **ciclo**, com
  o caminho na mensagem (`ciclo de dependência: a → b → a`). Limpar a lista
  sempre se aceita: é como se corta o ciclo que o `doctor` acusa. À mão também
  vale, para o celular e o claude.ai; o `doctor` acusa o que vier errado. A
  TUI tem a tecla `b` (um prompt com a lista atual).
- **Leitura.** Cada linha do `list --json` e o `show --json` trazem
  `bloqueada_por` (a lista, como está escrita), `bloqueada` (verdadeiro quando
  ao menos uma bloqueadora **ativa** existe) e `bloqueia` (os ids ativos que
  listam esta task). `freitask show <id>` em texto imprime o bloco da task e,
  embaixo, `bloqueada por: a, b` e `bloqueia: 3 (x, y, z)` quando houver; com
  ciclo, `CICLO: a → b → a` em stderr e saída `1`. Quem decide qual é a "task
  atual" é quem chama (as skills a descobrem pela branch): o freitask só
  responde sobre o id que recebe.
- **Semântica das bloqueadoras.** Uma bloqueadora **arquivada**, em qualquer
  tipo, está resolvida e não bloqueia. Uma que **não existe** também não
  bloqueia — um erro de digitação não pode fazer uma task sumir da frente — e
  vira achado do `doctor`. Uma **escondida** (Algum dia ou adiada) bloqueia
  normalmente: esconder não resolve. A task bloqueada **não sai** do painel
  nem da TUI: ganha uma marca (`⛔` antes do id na TUI; nada no `CURRENT.md`,
  cujo bloco é o do arquivo). Quem quer só a fronteira do que dá para começar
  filtra o JSON (`jq 'map(select(.bloqueada | not))'`); não há subcomando para
  isso.
- **`rename` arrasta; `delete` não toca.** Renomear o id de uma task reescreve
  o id em todas as linhas `bloqueada-por` do vault (edição só da linha no
  frontmatter, pela mesma gravação conferida), como já reescreve os wikilinks.
  Arquivar e mover de projeto não mudam o id, então não há o que reescrever.
  `delete` não toca ninguém: a dependência fica pendurada e o `doctor` a acusa,
  pelo mesmo motivo do `link-pendurado`.
- **Vigiado pelo `doctor`:** `dependencia-pendurada` (cita um id que não
  existe) e `dependencia-ciclica`, ambos sem `--fix`: não se adivinha o id, e
  escolher que aresta cortar é decisão editorial. É aqui que aparece o ciclo
  escrito à mão pelo celular, ou juntado por um merge do Syncthing de duas
  edições que sozinhas eram acíclicas.

Mora no frontmatter pelos mesmos motivos da prioridade: é metadado
consultável, o celular o edita, e o bloco é reserializado a partir do caminho,
que não carrega dependência nenhuma. Ficam de fora tipos de aresta e
dependência entre projetos como caso especial (o id é global).

### Prazo (`prazo`)

Um campo **opcional** no frontmatter, ortogonal à fase, à prioridade e ao
`volta-em`: uma **data informativa**, que a CLI, o JSON e a TUI mostram e o
`doctor` vigia.

```yaml
---
prazo: 2026-12-24
---
```

- **Não muda a ordem nem a visibilidade.** O prazo não reordena o painel, o
  `list` nem a TUI, e não esconde nem mostra task alguma: quem passa na frente
  continua sendo `prioridade: alta`. Um vault com `prazo:` em todas as tasks
  gera o mesmo `CURRENT.md` que sem, porque o prazo mora no frontmatter, que o
  painel nunca copiou. Não há urgência derivada do prazo, nem pontuação.
- **A escrita é `freitask set <id> --prazo AAAA-MM-DD`**, e `""` limpa o
  campo. A validação é a do `volta-em`: só `AAAA-MM-DD` de um dia que existe;
  data inválida é saída `2`, sem gravar nada do comando. À mão também vale. A
  TUI tem a tecla `p` (prompt com o prazo atual; vazio limpa).
- **Leitura.** O JSON traz `prazo` (o texto como está, ou `null`) e
  `dias_para_o_prazo` (inteiro, negativo quando vencido; `null` sem prazo ou
  com prazo inválido). `freitask show <id>` em texto imprime
  `prazo: AAAA-MM-DD (em N dias)` ou `prazo: AAAA-MM-DD (vencido há N dias)`.
- **Na TUI**, `prazo DD/MM` aparece ao lado do id, no mesmo lugar em que a
  adiada mostra `volta`: em amarelo até o dia do prazo, em vermelho depois dele.
  A task arquivada não mostra prazo (um prazo vencido de uma task feita seria
  ruído), nem a de prazo inválido (não há dia para mostrar).
- **Adiada com prazo** continua escondida até a data de volta. Se o prazo vem
  **antes** da volta, a task voltaria à tela já vencida, e o `doctor` acusa
  `prazo-antes-da-volta` enquanto ela ainda está adiada.
- **Vigiado pelo `doctor`:** `prazo-invalido` (o que não é uma data), sem
  `--fix` — o espelho do `volta-em-invalido`. Uma data inválida não conta
  dias: o JSON a devolve em `prazo`, com `dias_para_o_prazo` nulo.

Fica de fora tudo o que seria decidir pelo prazo: urgência derivada, recorrência
e "revisar depois de".

### Status 0 — callout sem tipo reconhecido

Se a linha 1 tem um `[!tipo]` que não existe em `status.json` (typo, tipo
removido, edição manual malfeita), a task recebe `status_num = 0` em vez de
cair silenciosamente em `todo`. O tipo digitado é **preservado verbatim** no
arquivo (não é substituído nem apagado), para que:

- a task continue consertável — `s` na TUI (ou `freitask set <id> --status
  <fase>`) troca o tipo; status 0 não é um beco sem saída;
- o erro fique visível: `freitask list --json` devolve o tipo errado em
  `callout`, e a TUI mostra a task com o ícone de erro (e o `doctor` acusa
  `status-0`);
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

São **as fases mais `done`**, e não todos os callouts que o render-markdown
suporta. O vocabulário largo não estava sendo escolhido — as tasks ativas
ficavam todas em `todo` — e status que ninguém escolhe não informa nada: a
diferenciação real terminava na frase em itálico, que é texto livre e não se
consulta. Cada fase existe porque alguém precisa distingui-la de outra para não
atropelá-la.

**Algum dia é escondida por padrão** (`HIDDEN_CALLOUTS` em `config`, no
`core`, e `hidden` no `list --json`): o `CURRENT.md` não a imprime nem a conta, e a TUI só
a mostra com `S`. Continua em `freitask list`, para quem precisa achá-la. É a
última fase que aparece na ordem; `done` só existe em arquivada. A task adiada
(`volta-em`, acima) é escondida pela mesma regra e pelo mesmo campo `hidden`,
sem ser dessa fase.

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

Tasks novas começam em `1` (`todo`).

O arquivo só é criado quando falta, e só por um comando que escreve (`new`,
`rebuild`…): quem só lê usa o vocabulário embutido no binário
(`DEFAULT_STATUS_JSON` em `config`) e não cria nada. Um vault que já tinha o
`status.json` de antes de Refinada e Algum dia precisa ganhar essas duas linhas
à mão (copie dali). Sem elas, a task em `[!tip]` ou `[!quote]` vira status 0,
nada é escondido, e o `doctor` acusa o callout que não existe.

Editar `status.json` muda rótulos/ícones/cores/ordem. O parser mapeia
`callout → número` a partir desse arquivo, então mantenha os callouts
distintos. Renumerar/reordenar não quebra tasks existentes: o status de cada
task é lido do `[!callout]` da linha 1, nunca de um número gravado no arquivo.
O `status.json` é lido a cada execução; se não for um JSON válido, o freitask
avisa em stderr e usa o vocabulário embutido.

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

- **Auto-regeneração**: os comandos que escrevem regeneram o `CURRENT.md` no
  fim, e salvar uma tarefa no Neovim roda `freitask rebuild` (autocmd do
  dotfiles) — tarefas novas e mudanças de status aparecem sozinhas. Abrir a
  TUI também o regenera, e é o que põe o painel em dia quando uma task adiada
  volta sem que nada tenha sido gravado. O `rebuild` só grava quando o
  conteúdo muda. Como a CLI roda fora do Neovim e não enxerga os buffers dele,
  um `CURRENT.md` aberto com edição não salva é regravado no disco mesmo
  assim, e o Neovim avisa (W12) quando você for salvá-lo — o mesmo aviso das
  notas que citam uma tarefa renomeada (ver "Referências do Obsidian"). O que
  está em `## Notas Avulsas` sobrevive à regeneração.
- **Arquivo diário**: ao regenerar num dia novo, o painel do dia anterior é
  snapshotado em `daily/YYYY-MM-DD.md` antes de ser recarimbado com a data
  de hoje.
- **`## Notas Avulsas`** é a única seção editável à mão que sobrevive à regen.
- **Ordem e o que não entra**: dentro de cada projeto, as tasks de prioridade
  alta vêm primeiro e depois a ordem da fase; as de Algum dia e as adiadas
  (`volta-em` num dia que vem) não são impressas e não entram na contagem do
  cabeçalho.

---

## Uso interativo (TUI)

`freitask` sem argumentos, num terminal, e `freitask tui` abrem a TUI; no
Neovim, `<leader>k` a abre num float. Fora de um terminal (script, agente,
pipe) os dois imprimem o usage e saem `2`: nada pode travar esperando uma tela
que ninguém vê. São três painéis: projetos, as tarefas do projeto (ícone e cor
da fase, um `!` nas de prioridade alta, um `⛔` antes do id nas que esperam
outra, e `prazo DD/MM` ao lado do id nas que têm prazo, ver "Prazo") e o
preview do arquivo.

| Tecla | Ação |
| ----- | ---- |
| `j` / `k`, `↓` / `↑` | mover |
| `h` / `l`, `←` / `→`, `Tab` | trocar de painel |
| `g` / `G`, `Home` / `End` | topo / fim |
| `Enter` / `e` | abrir a tarefa no editor |
| `C-d` / `C-u` | rolar o preview |
| `/` | filtrar por id ou título |
| `n` | nova tarefa no projeto: título → id (sugerido a partir do título (`freitask kebab`), editável) → descrição; nasce `todo` |
| `s` | trocar a fase |
| `t` / `i` | editar o título / a descrição do estado |
| `p` | definir / limpar o prazo |
| `r` | renomear o id |
| `b` | editar a lista `bloqueada-por` |
| `a` | arquivar (o tipo sugerido vem pré-selecionado) ou, numa arquivada, desarquivar |
| `A` | mostrar / esconder as arquivadas |
| `S` | mostrar / esconder as de Algum dia e as adiadas (com a data de volta ao lado da adiada) |
| `d` | largar: arquiva como `dropped` (pede `y`) |
| `N` | novo projeto |
| `c` | regenerar e abrir o `CURRENT.md` |
| `R` | reler o vault |
| `Esc` | limpar o filtro e o aviso |
| `?` | todas as teclas |
| `q` / `C-c` | sair |

A TUI **não conhece regra de tarefa**, e **roda em processo**: cada tecla monta
um pedido e chama as mesmas funções da CLI (`set`, `rename`, `archive`…) pelo
mesmo caminho — lock, leitura, plano do núcleo, gravação conferida e
regeneração do painel —, sem subprocesso e sem segunda cópia de regra. Quando a
regra recusa, a mesma mensagem que a CLI daria aparece na barra de baixo e o
modal volta preenchido para corrigir. Ao abrir, ela regenera o painel (o mesmo que
`freitask rebuild`) antes de ler as tasks; se isso falhar, a TUI abre mesmo
assim e o motivo vai para a barra. As **notas** (linhas 4+) se editam no editor
(`e`) — texto livre não cabe num campo.

Abrir uma tarefa com a TUI **dentro do Neovim** a abre no Neovim pai (via
`$NVIM`) e fecha o float, como o lazygit; fora dele, roda `$VISUAL`/`$EDITOR`
em primeiro plano e regenera o `CURRENT.md` e relê o vault na volta. Salvar uma
tarefa no Neovim roda `freitask rebuild` (autocmd do dotfiles).

Renomear o id **move** o arquivo (não escreve o novo e apaga o velho) e
**migra as referências do vault** junto com o `id:` do frontmatter e as listas
`bloqueada-por` das outras tasks, e registra o id antigo no `## Histórico`. O
id novo passa pela mesma validação de uma tarefa nova — kebab-case e livre,
inclusive em `archived/` —, porque ele é também o nome da branch.

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
frontmatter, do mesmo jeito que o status vem só do callout: `tasks/<id>.md` é
ativa, `tasks/archived/<tipo>/<id>.md` é arquivada, e `list`, o painel e a TUI
só enxergam as ativas, a não ser que se peça as arquivadas.

Os tipos são **três**:

| tipo | quando |
| ---- | ------ |
| `done` | concluída |
| `dropped` | decidi não fazer / parei de tocar |
| `failed` | tentei e não deu |

O tipo de arquivamento já vem **pré-selecionado** a partir do callout
(exposto como `suggested_archive` no `list --json`: `done`/`check` → `done`;
o resto → `dropped`). `failed` não tem
default desde que as fases encolheram — nenhuma delas significa
"falhou" — e se escolhe à mão. `dropped` é a única
decisão que o status não carrega. Não há um quarto balde genérico ("misc") de
propósito: ele viraria o destino de tudo que se arquiva com pressa, sem
distinguir nada — e criar um diretório depois é trivial, esvaziar um cheio não.

- **Arquivar** (`a` na TUI · `freitask archive <id> <tipo>`): move o arquivo,
  reescreve o wikilink da linha 2 para o novo caminho, registra no rodapé (ver
  abaixo) e migra as referências do vault.
- **Desarquivar** (`a` numa arquivada · `freitask unarchive <id>`): o caminho
  inverso. A tarefa reaparece no `CURRENT.md` na regen seguinte.

#### Rodapé `## Histórico`

Arquivar, desarquivar, mover de projeto e renomear o id acrescentam uma linha
datada ao fim do arquivo, numa seção `## Histórico` criada na primeira vez:

```markdown
## Histórico

- 2026-08-09 — arquivada em `archived/dropped` (abandonado)
- 2026-08-11 — desarquivada, de volta ao board
- 2026-08-30 — arquivada em `archived/done` (feito)
- 2026-09-02 — movida de `dotfiles` para `workflow-ia`
- 2026-09-05 — renomeada de `foo` para `bar`
```

É um **log, não uma linha única**: arquivar → desarquivar → rearquivar é um
ciclo normal, e sobrescrever apagaria justamente o que interessa (quantas vezes,
e quando você mudou de ideia). As glosas em PT-BR são `done` → feito,
`dropped` → abandonado, `failed` → falhou; o nome do diretório vem junto porque
é ele o dado real. Na mudança de projeto, origem e destino são os nomes dos
projetos, entre crases; no rename, o id antigo e o novo. A entrada do rename é
o único registro de que `foo` um dia foi o id da tarefa, e vale também numa
arquivada: a de arquivamento que já estava lá continua valendo.

Entradas novas entram no **fim da seção**, não no fim do arquivo — se você
escrever outra `## seção` depois do histórico, ela continua abaixo. Só o
arquivamento, a mudança de projeto e o rename escrevem ali: editar (`set`),
regenerar o board e encerrar ou reabrir o projeto **não** acrescentam linha. O
rodapé também não vaza para o `CURRENT.md`, que reimprime apenas o bloco de
callout.

Esta é a **única** exceção à regra de que a ferramenta não reescreve o corpo do
arquivo — e ela só acrescenta, nunca mexe no que já estava lá.

- **Ver as arquivadas**: `A` na TUI alterna a exibição delas na mesma lista
  (prefixadas com `[<tipo>]` e esmaecidas); `freitask list --archived` as
  inclui. Elas **nunca** entram no painel nem na contagem dele, que seguem
  enxergando só o que está ativo.
- **Largar** (`d` na TUI, que pede `y`): é arquivar como `dropped`. O arquivo
  fica e os links acompanham, como em qualquer arquivamento. Numa task já
  arquivada a tecla recusa: reclassificar é `a`, desarquivando antes.
- **Deletar** (`freitask delete <id>`, só na CLI, que não pergunta): apaga o
  arquivo do disco de vez. As referências a ela ficam penduradas de propósito,
  e o `doctor` as aponta (`link-pendurado`).

### Mover para outro projeto

`freitask move <id|caminho> <projeto>`. Leva o arquivo para
`projects/<projeto>/tasks/<id>.md` — ou para
`projects/<projeto>/tasks/archived/<tipo>/<id>.md`, se estava arquivada. O tipo
de arquivamento e a fase **não mudam**: mudar de projeto não é (des)arquivar. O
`id` também não, então os links curtos `[[id]]` seguem valendo; quem muda de
endereço são o wikilink da linha 2 e as referências path-qualificadas do vault
(a mesma reescrita dos outros moves, ver "Referências do Obsidian"), e a mudança
é registrada no `## Histórico`.

O `move` recusa — saída `1`, mensagem em stderr, nada escrito — quando:

- o projeto de destino **não existe**. A mensagem sugere `freitask new-project`:
  um projeto nunca nasce em silêncio por causa de um typo no nome;
- o projeto está **encerrado**. A mensagem aponta `freitask reabrir`, e não o
  `new-project`, que também recusaria;
- o projeto é **reservado** (`daily`, `templates`, `_encerrados`), a mesma
  noção do `new`;
- a task **já está** nesse projeto;
- o **id já está ocupado** no destino, ativo ou arquivado em qualquer tipo — o
  id é o nome da branch, e duas tasks com ele no mesmo projeto não se distinguem.

### Encerrar / reabrir um projeto

`freitask encerrar <projeto>` e `freitask reabrir <projeto>`.

Projeto que terminou mora em `projects/_encerrados/<projeto>/`. Encerrar leva
para lá a pasta `projects/<projeto>/` inteira (MOC, spec, `decisoes/`,
`anexos/` e as tasks arquivadas), e reabrir a traz de volta. O **caminho é o
único estado**, como o de uma task arquivada: o MOC não ganha campo. Um nível
abaixo do glob `projects/*/tasks`, o projeto encerrado sai do painel, do
`list`, do `projects`, da TUI e do `doctor` sem um filtro novo em leitor
nenhum.

Os dois reescrevem, no vault inteiro, os wikilinks path-qualificados que
começam por `projects/<projeto>/` (ao reabrir, por
`projects/_encerrados/<projeto>/`): a mesma varredura de wikilinks de
"Referências do Obsidian", com as mesmas exceções (`daily/` e `CURRENT.md`). As
notas da própria pasta entram, e é assim que a linha 2 de cada task acompanha.
Os links curtos ficam como estão, porque o basename de nada muda. E nenhum dos
dois escreve no `## Histórico`: quem mudou de lugar foi o projeto, não a task.

`encerrar` recusa (saída `1`, motivo em stderr, nada movido) quando:

- o projeto tem **task ativa**, em qualquer fase, inclusive Algum dia, que o
  painel esconde; a mensagem as lista. O desfecho de um projeto encerrado está
  nas tasks, e uma task ativa lá dentro sairia de vista sem desfecho: arquive-as
  antes;
- o projeto **não existe** (não há `projects/<projeto>/tasks/`);
- o nome é **reservado**;
- `projects/_encerrados/<projeto>/` **já existe**.

`reabrir` recusa quando o projeto **não está encerrado** e quando
`projects/<projeto>/` **já existe**: reabrir por cima juntaria duas pastas num
projeto só.

Encerrado, o projeto fica fora do alcance da CLI: as tasks dele não se acham
pelo id, e um caminho para dentro de `_encerrados/` não é reconhecido como
arquivo de task. Para mexer nelas, reabra.

Um nome encerrado também não volta por outra porta: `new`, `new-project` e
`move` o recusam e apontam `freitask reabrir <projeto>`. O `new` cria em
silêncio o projeto que não existe, mas não o encerrado: criá-lo ao lado da
pasta encerrada daria dois projetos com um nome só, um deles invisível.

### Referências do Obsidian

Arquivar, desarquivar, mudar de projeto e renomear o id são o **mesmo evento** —
"esse arquivo mudou de endereço" — e por isso passam todos pelo mesmo plano
(`deslocar`: mover o arquivo, regravá-lo no endereço novo, reescrever os
wikilinks que o citam), que varre o vault inteiro reescrevendo os wikilinks.
Cobre `[[alvo]]`, `[[alvo|alias]]`, embeds `![[...]]` e sufixos
`#heading` / `^bloco`; preserva o alias, exceto quando ele **era** o id antigo
(aí acompanha o rename).

Encerrar e reabrir são o mesmo evento para a pasta de um projeto, e passam pelo
plano de `projeto`: a mesma varredura, trocando o prefixo
`projects/<projeto>/` de cada link que começa por ele e deixando o resto do
link (alias, sufixo, extensão de anexo) como estava.

O Obsidian resolve `[[foo]]` pelo **basename**, em qualquer pasta do vault.
Daí a assimetria — e é por isso que arquivar e mudar de projeto saem baratos:

| operação | `[[foo]]` | `[[projects/p/tasks/foo\|foo]]` | frontmatter `id:` |
| --- | --- | --- | --- |
| arquivar / desarquivar / mudar de projeto | intacto | reescrito | intacto |
| encerrar / reabrir o projeto | intacto | reescrito | intacto |
| renomear id | reescrito | reescrito | reescrito (e a lista `bloqueada-por` das outras tasks) |

Ficam **de fora** da varredura, de propósito:

- `daily/*.md` — são snapshots de como o board estava naquele dia;
  reescrevê-los falsificaria o histórico.
- `CURRENT.md` (na raiz do vault) — é regenerado do zero; basta o
  `freitask rebuild` seguinte.

A CLI roda **fora** do seu Neovim e não enxerga os buffers dele: uma nota
aberta e com alterações não salvas que referencia a tarefa é reescrita no
disco mesmo assim. O Neovim avisa (W12) quando você for salvá-la — recarregue
(`:e`) antes; se salvar por cima, o `doctor` acusa o link pendurado.

---

## CLI (`freitask`)

A CLI é a **única porta para as regras**: a TUI chama o mesmo código em
processo, e também os outros escritores deste vault — o Obsidian (aqui e no
celular, via Syncthing) e agentes de IA com acesso a shell. Nenhum deles tem uma
API em processo para chamar; sem a CLI, chamariam `mv`. Ela existe para que o
caminho certo seja o mais fácil:

```bash
freitask                                  # TUI, se stdin e stdout são terminal; senão usage, saída 2
freitask tui
freitask current                          # exec nvim ~/ObsidianVault/CURRENT.md
freitask new         <projeto> <id> "<título>" [--desc "<estado>"] [--json]
freitask set         <id|caminho> [--status <fase>] [--title "…"] [--desc "…"] [--volta-em AAAA-MM-DD]
                                  [--prazo AAAA-MM-DD] [--bloqueada-por "id,id"]
freitask show        <id|caminho> [--json]
freitask archive     <id|caminho> done|dropped|failed
freitask unarchive   <id|caminho>
freitask rename      <id|caminho> <novo-id>
freitask move        <id|caminho> <projeto>
freitask encerrar    <projeto>
freitask reabrir     <projeto>
freitask delete      <id|caminho>
freitask new-project <nome>
freitask list        [--json] [--archived]
freitask projects    [--json]
freitask statuses    [--json]
freitask kebab       "<texto>"
freitask rebuild
freitask doctor      [--fix] [--json] [--quiet]
freitask version     [--json]
freitask help        [--json]
freitask <sub> --help | -h
```

`freitask help` é o usage geral e `freitask <sub> --help` (ou `-h`) o trecho de
um subcomando, com saída `0`; `--help` é resolvido **antes** de abrir o vault,
então nenhum subcomando com `--help` lê ou grava arquivo. Flag desconhecida é
saída `2`, com a mensagem do parser em stderr.

### Códigos de saída

| Código | Significa |
| ------ | --------- |
| `0` | ok; `doctor` sem achado pendente |
| `1` | recusa (task não encontrada, projeto encerrado, `done` por `set`, ciclo de dependência…); `doctor` com achado pendente; `show` de uma task em ciclo |
| `2` | uso inválido (falta argumento, fase que não existe, data inválida, flag desconhecida, `freitask` sem terminal para a TUI) |
| `3` | **conflito de escrita** (o arquivo mudou entre a leitura e a gravação) ou **lock ocupado** por mais de 5 s. Nada foi gravado, e repetir o comando é seguro. |

Os avisos saem em stderr, uma linha cada, com o prefixo `freitask: `;
stdout carrega só o dado.

### JSON: a mesma forma, campos só por adição

Com `--json`, `list`, `show`, `projects`, `statuses`, `doctor`, `new`,
`version` e `help` emitem estrutura parseável — é o contrato para agentes,
scripts e hooks. As regras:

- **A forma não muda.** `list --json`, `projects --json`, `statuses --json` e
  `doctor --json` seguem arrays e objetos crus, sem envelope, e os campos de
  antes seguem com o mesmo nome e tipo. Campo novo só entra por **adição**;
  campo removido ou de tipo mudado é mudança do contrato e **decisão do dono**
  (é o `json` de `version --json`). Quem lê deve ignorar campo que não conhece.
- **`id` é sempre string**, as chaves saem em **ordem alfabética**, o que não
  tem valor é `null`, e lista vazia é `[]`.
- **`list --json`** devolve um array de linhas, cada uma com `project`, `id`,
  `status` (o título da fase), `status_num`, `callout`, `title`, `desc`,
  `archived`, `suggested_archive`, `hidden`, `prioridade`, `volta_em`, `path`
  e os campos que dispensam abrir o arquivo da task: `notes` (as linhas 4+ do
  bloco, sem o `> `), `historico` (as linhas `- AAAA-MM-DD — …` do
  `## Histórico`, na ordem do arquivo), `checklist` (`{feitos, total}` dos
  `- [ ]` e `- [x]` do corpo) e, de dependências e prazo, `bloqueada_por`,
  `bloqueada`, `bloqueia`, `prazo` e `dias_para_o_prazo`.
- **`show --json`** é um objeto com os mesmos campos de uma linha do `list
  --json`, mais `ciclo` (a lista de ids do ciclo em que a task está, ou
  `null`). Em texto, `show` imprime o bloco da task e as linhas de dependência e
  prazo descritas em "Dependências" e "Prazo".
- **`new --json`** devolve `{"id", "path", "project"}`.
- **`projects --json`** é `[{project, active}]`, **`statuses --json`** é
  `[{num, callout, title, icon, hl_group, hidden}]` e **`doctor --json`** é
  `{ok, findings: [{level, kind, path, msg, fixed}]}`.
- **`version --json`** devolve `{"freitask": "<versão>", "json": <n>}`; `json`
  é a versão do contrato acima.
- **`help --json`** é a **gramática** dos subcomandos, gerada das definições
  da CLI: `{"json": <n>, "subcommands": {"set": {"writes": true,
  "positionals": ["ref"], "value_flags": ["status", …], "bool_flags": []}, …}}`.
  É a fonte para quem precisa saber quais subcomandos escrevem e que flags
  levam valor (as travas dos agentes), em vez de copiar a lista à mão. O
  `doctor` consta como `writes: false`: só o `--fix` escreve, e quem decide é a
  flag.
- O **schema de cada comando** mora em `tests/aceite/schema/` e é validado nos
  testes de aceite.

### Leitura pura

`list`, `show`, `projects`, `statuses`, `kebab`, `help`, `version` e
`doctor` sem `--fix` **nunca escrevem nada**: nem `status.json`, nem
`projects/`, nem o `CURRENT.md`, e não tomam o lock. Num vault vazio, nada é
criado. O que cria o que falta (`projects/`, `.freitask/status.json`) são os
comandos que escrevem.

### Como grava

Toda gravação do vault passa por um caminho só, para o Syncthing nunca ver um
arquivo pela metade e uma edição de outro dispositivo nunca ser perdida:

- **Só grava quando o conteúdo muda.** O comando lê o arquivo, calcula o
  resultado e, se ele é igual ao que já está lá, não grava e não toca no mtime
  (vale também para o `CURRENT.md`: `rebuild` repetido não escreve nada).
- **Temporário na mesma pasta e `rename`.** O resultado vai para
  `.freitask.<nome>.<pid>.tmp` ao lado do destino, com `fsync`, e só então
  substitui o arquivo por `rename` (com `fsync` da pasta). O temporário
  recebe antes as permissões do arquivo que substitui: o Syncthing sincroniza
  o modo, e uma troca de `0600` para `0644` seria um evento de sync à toa.
- **Conferência contra o que foi lido.** Antes do `rename`, o destino é relido;
  se mudou desde a leitura (o Obsidian, o celular, outro `freitask`), o
  temporário é apagado, nada é gravado e a saída é `3`. É conflito, nunca
  perda: repetir o comando refaz a conta sobre o que está lá agora.
- **Não normaliza.** CRLF por linha, BOM e a falta de `\n` final do arquivo
  sobrevivem a uma edição.
- **Lock entre dois `freitask`.** Os comandos que escrevem tomam um lock fora do
  vault (`$XDG_RUNTIME_DIR/freitask.lock`, ou `~/.local/state/freitask/lock`) e
  esperam até 5 s por ele; depois disso, saída `3`. Existe porque os hooks dos
  agentes chamam a CLI a cada comando de shell.
- **Operação com vários arquivos** (um `rename` com N backlinks) grava o arquivo
  primeiro e os links depois, sem journal: se for interrompida no meio, o
  estado intermediário é o que o `doctor` acusa (`link-pendurado`,
  `bloco-dessincronizado`) e o `--fix` repara.

### Os comandos

`set` recusa `--status done`: `done` é o que `archive <id> done` grava junto
com o move. `set --volta-em` e `set --prazo` recusam (saída `2`) o que não é uma
data `AAAA-MM-DD` de um dia que existe, sem escrever nada do comando, e `""`
limpa o campo; `set --bloqueada-por` recusa o que "Dependências" lista (saída
`1`). `delete` não pergunta nada — quem confirma é quem chama — e deixa
as referências penduradas de propósito, para o `doctor` apontar.

O dia de hoje é o relógio local; só os testes o fixam, com
`FREITASK_TODAY=AAAA-MM-DD`, o que vale para adiadas, prazo, cabeçalho do
painel e a linha do `## Histórico`. O vault é sempre `$HOME/ObsidianVault`:
nenhuma variável o troca (os testes trocam o `HOME`).

**O binário é a CLI inteira.** O shim do dotfiles (`vault/.local/bin/freitask`)
só faz `exec` dele; **nenhuma regra mora no shim nem na TUI**, e um segundo
freitask em bash ou python divergiria do primeiro em semanas, e aí existiriam duas
regras em vez de uma. O mapa do código está em
[`freitask-internals.md`](freitask-internals.md).

## `freitask doctor` — verificação e reparo

Nenhum escritor externo pode ser *obrigado* a usar a CLI. A estratégia,
portanto, não é impedir o desvio — é torná-lo **barulhento e, quando possível,
reparável**.

O que ajuda é que a maior parte dos invariantes é **derivável do caminho** (o
wikilink da linha 2, o `id:` do frontmatter, o estado de arquivamento) e por
isso não precisa ser obedecida: precisa ser regenerada. Só duas coisas se
perdem de verdade:

1. **backlinks externos após um rename na mão** — o `rename` da CLI migra as
   referências do vault e registra o id antigo no `## Histórico`; um `mv` não faz
   nenhuma das duas coisas, e nada mais registra que `foo` se chamava `bar`;
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
| `volta-em-invalido` — `volta-em:` que não é uma data `AAAA-MM-DD` | warn | não (não se adivinha a data) |
| `prazo-invalido` — `prazo:` que não é uma data `AAAA-MM-DD` | warn | não (idem) |
| `prazo-antes-da-volta` — `prazo` anterior à `volta-em` de uma task ainda adiada: ela voltaria à tela já vencida | warn | não |
| `dependencia-pendurada` — `bloqueada-por` cita um id que não existe (nem ativo nem arquivado) | warn | não (não se adivinha o id) |
| `dependencia-ciclica` — as tasks ativas se esperam em ciclo | error | não (que aresta cortar é decisão editorial) |
| `temporario-orfao` — `.freitask.<nome>.<pid>.tmp` de uma gravação que não chegou ao `rename` | warn | sim (apaga o temporário; o destino está no estado anterior) |

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

`doctor` sem `--fix` é **leitura pura**: não toma o lock, não grava e, num
vault vazio, não cria nada. Com `--fix`, executa o plano de reparo.

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

Não há API em processo para quem está de fora: a interface do freitask é a CLI.
`freitask help --json` diz quais subcomandos escrevem e que flags cada um leva;
o texto do `freitask help` ainda é o de antes da reescrita e não lista `show`,
`version`, `--prazo` nem `--bloqueada-por`, que estão nesta página.

1. **Criar tarefa**: `freitask new <projeto> <id> "<título>" [--desc "<estado>"]`
   (`--json` devolve `{id, path, project}`). É o caminho único onde há shell —
   ele valida (id kebab-case, projeto não reservado, id livre inclusive em
   `archived/`; projeto novo é criado, menos o encerrado, que se reabre),
   serializa o bloco pelo mesmo código de todo o resto e **regenera o
   `CURRENT.md`**. Montar o markdown na mão deixa o painel para trás: task criada
   por arquivo não aparecia no board até alguém salvar outra task no Neovim. Na
   TUI, `n` faz o mesmo. Sem shell (o claude.ai pelo conector MCP), escrever o
   arquivo à mão segue sendo a única opção: siga o formato do bloco à risca. A
   TUI lê os arquivos e regenera o `CURRENT.md` ao abrir pelo `c`;
   `freitask rebuild` é o mesmo para quem tem shell.
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
5. **Para ler, use `freitask list --json` ou `freitask show <id> --json`**, não
   regex nem `awk` sobre o arquivo: é o mesmo parser do bloco que o resto usa, e
   o JSON já traz o que seria extraído à mão — `notes` (as linhas 4+ do
   bloco), `historico` (o log do `## Histórico`), `checklist` (`feitos` e
   `total`), a fase, a prioridade, as dependências e o prazo. `callout` é o tipo
   cru digitado — relevante só quando `status_num` é 0, para ver o typo. Ler o
   `.md` direto só faz sentido onde não há shell.
6. **Não edite as seções `## <projeto>` do `CURRENT.md` à mão** — são geradas.
   Edite os arquivos de tarefa (ou use a CLI); o painel se atualiza ao salvar
   no Neovim e a cada escrita da CLI. Só `## Notas Avulsas` é livre.
7. **Ignore `daily/`, `templates/` e `projects/_encerrados/`** ao varrer
   projetos, e pule `tasks/*/archived/**` — estão arquivadas, não fazem parte
   do board ativo. O estado de arquivamento é o **caminho**, não uma flag de
   frontmatter; `list --json` já traz `archived` e `path`, e `list --archived`
   inclui as arquivadas.
8. **Nunca mova nem renomeie um arquivo de tarefa na mão** — use
   `freitask archive` / `unarchive` / `move` / `rename`. Os quatro reescrevem os
   wikilinks do vault, sem o que eles ficam pendurados; o wikilink da linha 2
   é path-qualificado e precisa ser reescrito junto; e eles registram a linha
   datada em `## Histórico`. A pasta de um projeto também não se move na
   mão para dentro ou para fora de `projects/_encerrados/`: é
   `freitask encerrar` / `freitask reabrir`, que levam os links junto. Apagar é
   `freitask delete`.
9. **Não escreva em `## Histórico` à mão** para marcar arquivamento, mudança de
   projeto ou rename — quem escreve ali é a CLI (`archive`, `unarchive`, `move`,
   `rename`), e a fonte de verdade do estado continua sendo o **caminho**. Uma
   linha de histórico sem o move correspondente é só um texto mentindo sobre
   onde o arquivo está.
10. **Adiar uma task é `volta-em`, não fase nem arquivo.** Com shell,
    `freitask set <id> --volta-em AAAA-MM-DD` (`""` limpa), que recusa data
    inválida. Sem shell, a linha `volta-em:` no frontmatter, com `AAAA-MM-DD` de
    um dia que existe. A task segue em `tasks/`, na mesma fase.
11. **Prazo e dependência são frontmatter, não bloco.** Com shell,
    `freitask set <id> --prazo AAAA-MM-DD` e
    `freitask set <id> --bloqueada-por "id-a,id-b"` (`""` limpa os dois): a CLI
    recusa data inválida, id que não existe e ciclo, sem gravar nada. Sem shell,
    a linha `prazo: AAAA-MM-DD` e a linha `bloqueada-por: [id-a, id-b]` no
    frontmatter (lista numa linha, só ids); o `doctor` acusa o que vier errado
    (`prazo-invalido`, `dependencia-pendurada`, `dependencia-ciclica`). Nenhum
    dos dois muda a ordem do painel nem esconde task: o que passa na frente é
    `prioridade: alta`.
12. **Saída `3` é "repita".** A CLI saiu com conflito de escrita ou lock
    ocupado: nada foi gravado, e rodar o mesmo comando de novo é seguro. Saída
    `1` é recusa, com o motivo em stderr; não insista com o mesmo
    pedido.

---

## Manutenção / verificação

- O código e o mapa de cada módulo estão em
  [`freitask-internals.md`](freitask-internals.md); os portões antes de dar por
  pronto, em `docs/agents/verificacao.md` e em `make help`.
- O formato do bloco tem **fonte única**: `crates/core/src/model.rs` (parser e
  serializer). Mudança de formato começa ali, e nada fora do `core` reimplementa
  o bloco. O shim do dotfiles e a TUI não têm regra: **não** acrescente regra
  neles, senão passam a existir duas versões da mesma regra.
- Todo caminho que **move ou renomeia** um arquivo de task, ou uma pasta que as
  contenha, passa pelo plano de `deslocar` (task) ou de `projeto` (pasta de um
  projeto): são o único ponto que conserta as referências do vault e escreve o
  `## Histórico`. Comando novo que mova arquivo por fora deles deixa wikilinks
  pendurados.
- O `doctor` é a rede de segurança contra escritores que não passam por este
  código (Obsidian, celular, agentes). Ao acrescentar um invariante ao formato,
  acrescente **junto** o check correspondente — um invariante que só existe na
  documentação é um invariante que vai ser violado em silêncio.
