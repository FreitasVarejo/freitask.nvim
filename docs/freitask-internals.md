# Freitask — mapa dos módulos

Documento para quem vai **mexer no código**. Para o formato das tarefas, o
fluxo de uso e o contrato com agentes de IA, ver [`freitask.md`](freitask.md) —
este aqui não repete nada de lá. O porquê da stack e da arquitetura está na spec
da reescrita (`~/ObsidianVault/projects/workflow-ia/reescrita-do-freitask-spec.md`,
§1); as decisões tomadas durante a reescrita, em
[`decisoes-da-reescrita.md`](decisoes-da-reescrita.md).

O código principal é um **workspace Rust** na raiz (`Cargo.toml`, `crates/`):
três crates, um binário só (`freitask`) que é a CLI e a TUI. O workspace também
tem as crates de teste de `tests/` (o gerador de vaults sintéticos, o aceite e o
diferencial), que não são código de produção.

```
agentes, scripts, hooks ──▶ freitask (cli) ──┐
terminal ───────────────▶ freitask (tui) ──┴──▶ freitask-vault ──▶ freitask-core
```

(O binário depende também de `freitask-core` diretamente, para os tipos.)

---

## As crates

Regra de dependência, **acíclica por construção**: `freitask-core` não depende
de nada do workspace; `freitask-vault` depende de `core`; o binário depende dos
dois. O `core` é puro — sem I/O, sem relógio, sem variável de ambiente: tudo que
entra é parâmetro e tudo que sai é valor. Quando uma operação muda o vault, ele
devolve um **plano** (`Efeito`: `Escrever`, `Mover`, `Apagar`…; os que mexem em arquivo
carregam os bytes que a operação leu) e quem o executa é o `vault`.

Tamanho de módulo não está aqui de propósito: contagem em contrato apodrece
(ADR 0009 e 0014 no vault). Quando precisar, `wc -l crates/*/src/*.rs`.

### `freitask-core` (`crates/core/src/`)

A tabela está na ordem da pilha dentro da crate: um módulo só usa os que vêm
antes dele (`use crate::…`). A raiz (`lib.rs`) tem as funções soltas que todos
usam: `trim`, `eh_espaco`, `pad_bytes`, `lua_q`, que existem para reproduzir
byte a byte o que o Lua fazia.

| Módulo | Responsabilidade | Não é responsabilidade dele |
| --- | --- | --- |
| `config` | Constantes do formato: pastas reservadas, tipos de arquivamento, o `status.json` default, tabelas de callout, versão. Sem lógica, sem estado. | Decidir qualquer coisa. |
| `date` | `Date`: `AAAA-MM-DD` que existe no calendário, e a conta de dias. | Saber que dia é hoje — o dia entra por parâmetro. |
| `texto` | `Doc`: um arquivo como linhas, sem perder BOM, CRLF por linha nem a ausência de `\n` final. | Saber o que o texto significa. |
| `status` | O vocabulário de fases lido do `status.json` (`Vocab`) e o mapa reverso callout → número. | Saber a fase de uma *task* — isso é o callout dela. Ler o arquivo. |
| `path` | Tudo que se deriva de um **caminho** (relativo ao vault): projeto, id, arquivamento, `kebab`. | Ler arquivo. |
| `md` | Linhas de markdown: blockquote, frontmatter, seção `##`, splice, linhas do `## Histórico`. | Saber o que é uma task. |
| `model` | **Parser e serializer do bloco.** Puro: linhas entram, `Model` sai, e vice-versa. O parser é tolerante (formatos legados, callout desconhecido vira status 0). | Validar entrada — quem é estrito é `task` e `operacao`. Tocar disco. |
| `meta` | O que se lê e escreve no frontmatter, ortogonal à fase: prioridade, data de volta, prazo, `bloqueada-por`; e o que decide se uma task está adiada ou quantos dias faltam ao prazo. | Saber a FASE — isso é o callout. Resolver a dependência (é `deps`). |
| `links` | O scanner de wikilinks (spans em bytes) e a reescrita do segmento do alvo quando um arquivo, ou a pasta de um projeto, muda de endereço. | Decidir que ele mudou. |
| `index` | Lê **uma vez** cada task (`scan_task`) e monta o `Indice`: tasks ativas e arquivadas, a ordem em que o painel, o `list` e a TUI as mostram, e a regra de qual delas fica escondida (`is_hidden`: a fase Algum dia ou a task adiada). | Abrir arquivo: recebe bytes e caminhos. |
| `deps` | O grafo de `bloqueada-por`: o lado reverso, o "bloqueada", ciclo, validação da lista e a troca de id num rename. | Gravar a lista — é `meta`, e o plano é de `operacao`. |
| `task` | As regras puras de uma task: validar projeto e id, a fase pedida ao `set`, a data de volta, o prazo, a fase depois de (des)arquivar, o template de uma task nova. | Tocar disco. |
| `board` | Renderiza o `CURRENT.md` (e, antes de sobrescrever um painel de outro dia, o snapshot de `daily/`). Preserva `## Notas Avulsas`. | Ser fonte de verdade de coisa alguma. |
| `plano` | O tipo do plano: a lista de `Efeito`s sobre arquivos. | Executá-lo. |
| `operacao` | As operações de escrita (`new`, `new-project`, `set`, `delete`) como função pura: recebem os bytes lidos e uma `Visao` do que existe no disco, devolvem um plano ou uma `Recusa`. A ordem das validações e o texto das recusas moram aqui. | Tocar disco — a `Visao` é só pergunta. |
| `doctor` | Verificação e reparo como função de uma `Fotografia` do vault em memória: devolve os achados e, com `--fix`, o plano que repara o que é derivável do caminho. | Adivinhar (não inventa data nem resolve conflito). Ler o vault. |
| `deslocar` | `archive`, `unarchive`, `rename` e `move`: o mesmo evento ("o arquivo mudou de endereço") e o mesmo plano — mover, regravar o arquivo no endereço novo, reescrever os wikilinks que o citam. | Executar o plano. |
| `projeto` | `encerrar` e `reabrir`: a pasta inteira do projeto muda de endereço, e os links que apontavam para dentro dela acompanham. | Escrever no `## Histórico` das tasks — quem mudou de lugar foi o projeto. |

### `freitask-vault` (`crates/vault/src/`)

| Módulo | Responsabilidade | Não é responsabilidade dele |
| --- | --- | --- |
| `lib` | `Vault`: a raiz (`$HOME/ObsidianVault`), a varredura (projetos, tasks, notas), a leitura em bytes, a `Fotografia` do `doctor`, o índice, e o `aplicar` de um plano. Implementa a `Visao` do `core`. | Decidir regra: monta a entrada para o `core` e executa o que ele devolve. |
| `disco` | O sistema de arquivos atrás de um trait (`Disco`), com a implementação real e uma que falha em cada passo, para a gravação ser testada com falha injetada. | Ordem das operações — é `escrita`. |
| `escrita` | A **única** porta de gravação: temporário na mesma pasta, `fsync`, conferência por hash contra o que foi lido (CAS), `rename`, `fsync` do diretório; e `aplicar`, que executa um plano na ordem. Não grava quando nada mudou. | Decidir o que gravar. |
| `trava` | O lock entre dois `freitask` na mesma máquina, fora do vault. | Ser tomado por leitura: só os comandos que escrevem o tomam. |

### O binário (`crates/freitask/src/`)

O pacote tem uma biblioteca (`lib.rs`: o que os testes de fora e a TUI
alcançam em processo) e o binário (`main.rs`, que despacha para `cli`).

| Módulo | Responsabilidade | Não é responsabilidade dele |
| --- | --- | --- |
| `main` | Despacho: entrega os argumentos a `cli::run`. | Ter lógica. |
| `cli` | Os subcomandos com clap: argumentos, saída (texto ou `--json`), código de saída, o relógio (`FREITASK_TODAY` ou o local, lido uma vez por comando; a TUI o recebe como função e o pergunta a cada comando), o `current` (que executa o Neovim no `CURRENT.md`), e a gramática de `help --json`. | Ter regra — é tradução entre argumentos, chamadas e texto. |
| `usage` | O texto de `freitask help` e o trecho de cada subcomando. | Dizer o que um comando faz: aponta para `freitask <sub> --help` e `freitask.md`. |
| `escrita` | Os comandos que escrevem (`new`, `new-project`, `set`, `delete`, `rebuild`): toma o lock, lê, pede o plano ao `core`, executa com CAS, regenera o painel. Devolve um `Resultado`. | Decidir uma validação ou a mensagem de uma recusa — são do `core`. |
| `deslocar` | O mesmo fluxo para `archive`, `unarchive`, `rename`, `move`, `encerrar`, `reabrir`, mais o aviso de quantas referências foram reescritas. | Calcular o plano — é do `core`. |
| `doctor` | `doctor [--fix] [--json] [--quiet]`: sem `--fix` é leitura pura (não toma o lock, não grava); com `--fix`, executa o plano do `core`. | Achar o que está errado — é `core::doctor`. |
| `saida` | O `Resultado` de um comando que escreve, sem terminal e sem `HOME`: a CLI o imprime, a TUI o mostra na barra. Tem o `Contexto` (vault, lock, dia) e a `Corrida` que acumula o stderr. | Imprimir. |
| `linhas` | A linha de uma task como o `list --json`, o `show --json` e a TUI a veem, e o texto do `list`. Uma função só: JSON e tela saem do mesmo lugar. | Ordenar ou esconder — é o `Indice`. |
| `tui` | A TUI em processo; tabela abaixo. | Ter regra de task. |

### A TUI (`crates/freitask/src/tui/`)

| Arquivo | Responsabilidade |
| --- | --- |
| `estado.rs` | O estado da tela e o que cada tecla faz. **Puro**: `update(State, Evento)` devolve o estado novo e os `Comando`s que o laço executa; nada aqui toca disco nem processo. É o que os testes de tecla cobrem. |
| `atalhos.rs` | As teclas do modo normal numa tabela só: a ajuda (`?`) e o despacho saem dela, e o teste `toda_tecla_da_ajuda_tem_handler` vigia as duas pontas. |
| `dados.rs` | O que a tela mostra, lido do vault de uma vez, pelos mesmos tipos do `core` e as mesmas funções de `linhas.rs` que a CLI usa. |
| `tela.rs` | Desenho (`view`, ratatui). Só lê o `State`. Os snapshots ficam em `snapshots/`. |
| `sessao.rs` | O `State` mais o executor dos `Comando`s contra o vault, sem terminal: abre (`rebuild` e depois a leitura), recebe tecla, devolve tela, e devolve como `Pedido` o que precisa do terminal. É o contrato que o aceite usa (decisão f7.1 em `decisoes-da-reescrita.md`). |
| `laco.rs` | O terminal de verdade em volta da `Sessao`: lê tecla, redesenha, executa o `Pedido`. Abrir arquivo dentro de um terminal do Neovim (`$NVIM`) vai para o Neovim pai por `--remote-expr`. |
| `propriedades.rs` | Só teste: sequências aleatórias de teclas e redimensionamentos sobre dados sintéticos. |

---

## Por que estas costuras

- **O `core` devolve plano em vez de executar.** `archive`, `rename` e
  `doctor --fix` se testam sem disco, e a mutação (`make mutantes`) mede onde a
  regra mora. O que falta é só a fronteira, que é pequena e tem os testes dela.
- **`model` é a parte que erra em silêncio**: um bloco serializado errado só
  aparece quando o arquivo já foi gravado. Por isso é puro e é onde está quase
  todo o teste.
- **`md` não sabe o que é uma task.** Se soubesse, o parser do bloco e as
  operações de arquivo teriam que concordar sobre o que é "o bloco" em dois
  lugares.
- **`path` existe porque o caminho é fonte de verdade** de duas coisas — projeto
  e arquivamento —, exatamente como o callout é a do status. Não há flag de
  frontmatter para nenhuma das duas.
- **Mudança de id é *rename*,** e um rename arrasta o arquivo, o `id:` do
  frontmatter, os wikilinks do vault inteiro e a lista `bloqueada-por` das outras
  tasks. Por isso o id novo passa pela mesma validação de uma task nova: ele é
  também o nome da branch.
- **O `core` renderiza o painel, quem chama decide quando.** Os comandos de
  `escrita` e `deslocar` regeneram o `CURRENT.md` uma vez só, no fim; as
  operações do `core` nunca o fazem.
- **Todo byte gravado passa por `vault::escrita`.** O arquivo é lido, o
  resultado é calculado, e só se grava se mudou, com temporário na mesma pasta e
  conferência por hash antes do `rename` (spec, §1.5): o Syncthing só vê o
  arquivo final, e uma mudança de outro escritor entre a leitura e a gravação
  vira conflito (saída 3), nunca perda. O lock cobre dois `freitask` na mesma
  máquina, e leitura nunca escreve.
- **O relógio e o ambiente entram na borda.** `cli` lê o dia
  (`FREITASK_TODAY` ou o local) uma vez por comando e o passa a todos; a TUI,
  que fica aberta por horas, recebe o relógio e o pergunta a cada comando. O
  `core` nunca o lê, o que deixa adiada, prazo e cabeçalho do painel testáveis
  com o dia fixo.
- **A TUI roda em processo, com `update` puro e `view` que só lê** (spec,
  §1.7). A TUI monta o pedido e chama as mesmas funções da CLI, pelo mesmo
  caminho, então não há subprocesso e não há segunda cópia de regra. Regra nova
  vai no `core`; `cli` e `tui` só traduzem.

## Até o cutover

Enquanto a branch `reescrever-o-freitask` não entra em `main`, `lua/` continua
sendo o freitask instalado e o **oráculo** do `make diff` (spec, §3.4): não se
muda comportamento nele. `tui/` da raiz (a TUI antiga, que chama a CLI instalada) e os
`tests/*.lua` com `tests/run.sh` e `tests/runner.lua` ficam como estão. O que
sai no cutover está na §6.1 da spec. Os portões dessas pastas seguem no bloco de
`docs/agents/verificacao.md`.

## Convenções

- **Comentário responde "por quê", não "o quê".** O código já diz o que faz; o
  que se perde com o tempo é a razão de ele fazer *assim* — e é essa razão que
  impede alguém (você, daqui a seis meses, ou um agente) de "simplificar" um
  guarda que existe por um motivo. Onde a razão é longa demais para o código,
  ela está em `freitask.md`, não duplicada aqui.
- **Especificação de formato mora em `freitask.md`.** Duas cópias de uma spec
  divergem; a do código era a que ficava desatualizada.

---

## Testes

A pirâmide e o que cada camada prova estão na §3 da spec; quem escreve teste de
aceite segue `docs/agents/aceite.md`. Onde cada camada mora:

- **Unitários** ficam no próprio `src/` de cada crate (`#[cfg(test)]`);
  **propriedades** e testes de integração, em `crates/*/tests/`.
- **Vaults sintéticos** vêm de `tests/gerador/`, que o `core`, o `vault` e o
  diferencial usam.
- **Aceite, congelado:** `tests/aceite/`. Roda o binário de verdade com `HOME`
  num diretório temporário.
- **Diferencial contra o oráculo Lua:** `tests/diferencial/` (ver o `README.md`
  dele). As divergências aprovadas estão em `tests/aceite/divergencias.toml`.
- **TUI:** teclas e propriedades de `update` em
  `crates/freitask/src/tui/estado/testes.rs` e
  `crates/freitask/src/tui/propriedades.rs`; o desenho, em `crates/freitask/src/tui/tela.rs` com os
  snapshots de `crates/freitask/src/tui/snapshots/`; o contrato em processo, na
  `Sessao`, usado pelo aceite.
- **Mutação:** configurada em `.cargo/mutants.toml`.

`tests/aceite/`, snapshots e goldens **não se editam nem se apagam** (spec, §4):
a regra, as travas e o que fazer com teste que parece errado estão no
`AGENTS.md` do repo.

O que segue **sem** teste automatizado: o desenho da TUI além do que os
snapshots e as asserções de tela dizem. Isso se verifica abrindo a TUI (ver
`docs/agents/verificacao.md`, "Como testar à mão").

## Ao mexer

Os alvos são os do `Makefile` (`make help`); o agente não digita `cargo test` à
mão (o runner não acusa seleção vazia). Por ordem:

1. `make check`
2. `make test`
3. `make aceite`
4. `make diff` quando muda comportamento observável da CLI (precisa de `nvim`
   no PATH)
5. antes de fechar uma fatia, `make portoes` (grava o `PORTOES.md`)

Quem mexer em `lua/` ou em `tui/` antes do cutover roda também os portões
dessas pastas, que estão no bloco de `docs/agents/verificacao.md`.
