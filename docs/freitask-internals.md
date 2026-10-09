# Freitask — mapa dos módulos

Documento para quem vai **mexer no código**. Para o formato das tarefas, o
fluxo de uso e o contrato com agentes de IA, ver [`freitask.md`](freitask.md) —
este aqui não repete nada de lá.

Duas partes. O **motor** vive em `lua/freitask/`: a raiz do repo entra no
`runtimepath` de um `nvim --clean -l` e `require("freitask")` resolve a partir
de `lua/` — o Neovim é só o interpretador, nenhuma janela abre. O único
consumidor do motor é a CLI (`lua/freitask/cli.lua`, embrulhada pelo shim
`freitask` do dotfiles). A **TUI** vive em `tui/` (Rust) e é consumidora da
CLI, não do motor: lê o JSON dela e muda o vault chamando subcomandos.

```
freitask-tui ──(processo, JSON)──▶ freitask (shim) ──▶ nvim -l cli.lua ──▶ require("freitask")
agentes, scripts, healthcheck ────▶ freitask (shim) ──┘
```

Regra nova do motor vai no motor, e a TUI a enxerga pela CLI. Uma regra
reescrita em Rust seria a segunda cópia que `cli.lua` existe para evitar.

---

## A pilha

Ela é **acíclica por construção**, e a regra para mantê-la assim é: um módulo só
pode requerer módulos à sua esquerda.

```
config ──┬── fs ────┬── path ──┬── model ──┬── cache ──┬── task ── board ── doctor
         └── status ┘   md ────┘           └── links ──┘
```

`meta` entrou depois e ocupa a mesma faixa de `model` e `links`: requer `md`, é
requerido por `cache` e `doctor`. Fora do desenho só para não redesenhá-lo —
a regra de "só requer quem está à esquerda" vale para ele igual.

Tamanho de módulo não está aqui de propósito: contagem em contrato apodrece
(ADR 0009 e 0014 no vault). Quando precisar, `wc -l lua/freitask/*.lua`.

| Módulo | Responsabilidade | Não é responsabilidade dele |
| --- | --- | --- |
| `config` | Caminhos, tipos de arquivamento, `status.json` default, tabelas de callout. Sem lógica, sem estado. | Decidir qualquer coisa. |
| `types` | Só `---@class`: `Model`, `Entry`, `StatusMeta`, `Finding`, `TaskRef`. | Ter código. |
| `fs` | Ler arquivo, recarregar/reapontar buffer, varrer o vault. | Saber o que é uma task. |
| `status` | O vocabulário de status lido do `status.json`, e o mapa reverso callout → número. | Saber o status de uma *task* — isso é o callout dela. |
| `path` | Tudo que se deriva de um **caminho**: projeto, id, arquivamento, `kebab`. | Ler arquivo. |
| `md` | Linhas de markdown: blockquote, frontmatter, seção `##`, splice. | Saber o que é uma task. |
| `model` | **Parser e serializer do bloco.** Puro: linhas entram, `freitask.Model` sai. | Tocar disco, notificar, conhecer buffer. |
| `cache` | O índice em memória das tasks **ativas**, com o bloco de cada uma. | Enxergar arquivadas (elas são lidas do disco sob demanda). |
| `links` | Reescrever os wikilinks do vault quando um arquivo muda de endereço. | Decidir que ele mudou. |
| `meta` | O eixo de EXECUÇÃO no frontmatter (`dono`/`desde`/`dominio`) e a aritmética de data que o sustenta. | Saber a FASE — isso é o callout. |
| `task` | Operações sobre o **arquivo** de uma task: criar, editar (`apply_edit`, que vira rename quando o id muda), mover, apagar, localizar por id. | Regenerar o board — quem chama decide quando. |
| `board` | O `CURRENT.md`. | Ser fonte de verdade de coisa alguma. |
| `doctor` | Verificação, reparo e migração de formato. | Adivinhar (não inventa data nem resolve conflito). |
| `cli` | Os subcomandos: argumentos, saída (texto ou `--json`) e código de saída. | Ter regra — é shim sobre o `init`. |
| `init` | A API pública: os submódulos + a superfície plana (as atribuições `M.<nome> =` no fim do arquivo). | Ter lógica. |

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
- **`task.apply_edit` trata mudança de id como *rename*,** e um rename arrasta
  o arquivo, o `id:` do frontmatter e os wikilinks do vault inteiro. Por isso
  o id novo passa pela mesma validação de uma task nova: ele é também o nome da
  branch.
- **`task` não chama `board.rebuild_current`.** Quem chama é que sabe se vale a
  pena: a CLI regenera uma vez só, no fim de cada subcomando.
- **Sobra consciência de buffer em `fs`, `links`, `task` e `board`** (pular
  buffer sujo, reapontar buffer renomeado). Era do tempo em que o motor rodava
  dentro do Neovim do usuário; sob `nvim -l` não há buffer carregado e esses
  ramos não disparam.

## A TUI (`tui/`)

| Arquivo | Responsabilidade |
| --- | --- |
| `engine.rs` | A porta para a CLI: roda `freitask <args>`, decodifica o JSON, devolve o stderr do motor como erro. `FREITASK_BIN` troca o executável. |
| `app.rs` | O estado da tela e o que cada tecla faz. **Puro**: tecla entra, `Effect` sai — é o que os testes cobrem. |
| `ui.rs` | Desenho (ratatui). Só lê o `App`. |
| `main.rs` | O laço: executa os efeitos (processo, editor, leitura do preview) e redesenha. Abrir arquivo dentro de um terminal do Neovim (`$NVIM`) vai para o Neovim pai por `--remote-expr`. |

`cargo test` roda os testes; o dotfiles instala com `cargo install --path tui`.

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
tests/run.sh          # a suíte inteira
tests/run.sh freitask # filtra por nome de arquivo
```

`tests/runner.lua` é um harness mínimo rodando sob `nvim --clean -l`
— sem busted, sem plenary. A suíte testa um módulo que só toca arquivos e
`vim.fn`; uma dependência externa aqui seria um passo de setup que ninguém
lembra de fazer antes de mexer no parser.

Cobertura em `freitask_spec.lua`: as funções **puras** — `path.kebab`,
`path.split_task_path`, `path.suggest_archive_type`, `status.status_meta`,
`model.is_status_text`, `model.parse_block` (formato atual, legado e status 0),
`model.serialize_block`, o round-trip entre os dois, `md.first_block_range`,
`md.block_around`, `md.splice`, `md.append_history`,
`md.update_frontmatter_key`, `links.rewrite_link`, o eixo de execução de
`meta` e, da criação, `task.validate_new` e `task.template` — a validação foi
separada da escrita justamente para caber aqui: as regras de um id novo (kebab,
projeto reservado) são a parte interessante, e `create_task` fica só com o
disco.

`cli_spec.lua` cobre a outra metade, **de ponta a ponta**: cada caso roda a
CLI num processo `nvim -l` de verdade com `HOME` apontando para um diretório
temporário (o módulo deriva o vault de `~`, então nada toca o vault real). Passa
por `new`, `set`, `list`/`projects`/`statuses --json`, `archive`, `unarchive`,
`rename`, `delete` e `doctor`, e confere o contrato JSON, os códigos de saída,
os dois formatos de wikilink num arquivo que referencia a task, o
`## Histórico` e o `CURRENT.md`.

`tui/` tem os próprios testes (`cargo test`), do estado e das teclas; o
desenho em si se verifica abrindo a TUI.

O que segue **sem** teste automatizado: `doctor --fix`. Para
o `--fix`, um vault descartável basta — crie o estado quebrado à mão e rode
`HOME="$(mktemp -d)" … cli.lua doctor --fix` como `cli_spec.lua` faz.

## Ao mexer

1. `tests/run.sh`
2. `luacheck lua tests`
3. `(cd tui && cargo test)` se mexeu na TUI — ou no contrato JSON que ela lê
4. `(cd tui && cargo fmt --check && cargo clippy --all-targets -- -D warnings)` se mexeu na TUI
5. `freitask doctor` no vault real (read-only; deve sair verde)

O luacheck **não** pega `mod.foo` apontando para uma função que não existe —
`mod` é uma tabela válida e o erro só aparece em runtime, possivelmente meses
depois, num caminho que você não exercita. Se você mover funções entre módulos,
vale rodar um checador de referências cruzadas (foi assim que dois erros de
substituição apareceram durante a divisão do monolito).
