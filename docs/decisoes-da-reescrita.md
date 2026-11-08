# Decisões da reescrita (log do chat 3)

O que a spec (`~/ObsidianVault/projects/workflow-ia/reescrita-do-freitask-spec.md`)
não respondia e foi decidido aqui, com o porquê. Uma entrada por decisão, na
ordem em que surgiram. Mudança de teste congelado só entra depois do veredito
de um subagente que o reavaliou contra a spec (§4, camada 1), e fica listada
na seção "Testes reavaliados".

## Fatia 0

1. **`rust-version = "1.99"`.** O notebook tem `cargo 1.99.1` da distro, sem
   rustup; a spec manda fixar o stable do dia em que o chat 3 começa, e é o
   que se consegue compilar e verificar aqui. Edição 2024.
2. **ratatui 0.29, não 0.30.** O `tui/` atual é 0.29 e o porte de `app.rs` e
   `ui.rs` é direto; o 0.30 existe (resolveu 0.30.2) e muda API sem ganho para
   o que a TUI faz. `vt100` ficou em 0.15 porque o 0.18 exige
   `unicode-width ^0.2.2` e o ratatui 0.29 fixa `=0.2.0`.
3. **O subagente escreve em `.aceite-rascunho/<fatia>/`, e `make congelar`
   leva para `tests/aceite/<fatia>/`.** O `permissions.deny` de Edit/Write em
   `tests/aceite/**` (spec §4, camada 3) vale para o subagente também, então
   ele não consegue gravar lá. O rascunho é gitignored; o `congelar` copia,
   regenera `tests/aceite/Cargo.toml` (um `[[test]]` por pasta, por script,
   nunca à mão) e grava `ACEITE.sha256`. Recusa pasta que já existe: teste
   congelado não se regrava.
4. **O hash exclui `mutantes.baseline` e `contagem.baseline`.** A spec os põe
   em `tests/aceite/` e manda a catraca regravá-los a cada fatia; se entrassem
   no hash, toda fatia fechada quebraria o hash. Só `make baseline` os escreve.
5. **A trava de Bash não está ativa nesta sessão.** O `.claude/settings.json`
   do repo só é lido quando o Claude Code sobe dentro do repo, e esta sessão
   subiu em `~`. Medido: `echo tests/aceite` passou. As travas valem para as
   sessões seguintes e para a conferência do dono; nesta, a disciplina é a dos
   scripts (`congelar`, `hash-aceite --conferir` em `make portoes`) e a regra
   de não tocar `tests/aceite/` por caminho nenhum.
6. **Formato de `divergencias.toml`.** Cada entrada tem `id`, `motivo`,
   `teste`, e três regex: `cenario` (nome do cenário, `<corpus>::<nome>`),
   `onde` (`codigo`, `stdout`, `stderr`, `json:<ponteiro>`, `arquivo:<rel>`) e
   `detalhe` opcional. A primeira entrada que casa cobre a diferença. Está
   documentado em `tests/diferencial/README.md`.
7. **Ordem dos achados do `doctor` é canonizada antes de comparar.** O Lua
   itera `pairs()` para `id-duplicado`, e a ordem varia entre processos; o
   harness ordena `findings` por (kind, path, msg) e, no texto, os pares de
   linhas. Nenhum consumidor depende da ordem (`check.sh` filtra por level).
8. **O harness passa `LANG=LC_ALL=C.UTF-8` e herda `TZ`.** O `weekday:` do
   painel vem de `os.date("%A")`, que é sensível a locale; o vault real está em
   inglês (`weekday: Sunday`, medido), e o binário novo imprime sempre em
   inglês.
9. **Corpus sintético também recebe as sequências de escrita.** A spec fala em
   "sequências de escrita em cópias" para o vault real; A e B já são cópias, e
   as mesmas sequências rodam em todo corpus. Os placeholders `{task1}`,
   `{task2}`, `{proj1}`, `{proj2}` são resolvidos pelo oráculo sobre uma cópia
   à parte, escolhendo ids sem homônimo; sem valor, o cenário é pulado e o
   relatório diz.
10. **Aceite da fatia 0 nasce depois das travas.** Os testes da fatia 0 são
    sobre as travas (settings, hook, mutants.toml, Makefile, hash, tag
    `oraculo-lua`) e sobre `divergencias.toml`; não há comportamento de CLI a
    falhar antes, então a regra "confirmar que falham" não se aplica a eles. As
    fatias de 1 em diante seguem a regra à risca.
11. **`migrate_format` não ganha subcomando.** O oráculo tem a função
    (`doctor.lua:41`) mas nenhuma porta da CLI a chama; a migração do
    `archived: true` legado não acontece hoje por comando nenhum. Paridade
    manda não inventar a porta; o `doctor --fix` continua reserializando o
    bloco (o que normaliza o formato legado do bloco, como hoje).
13. **`scripts/descongelar.sh` desfaz um congelamento ainda sem tag.** O
    primeiro rascunho congelado não passava no `cargo fmt --check` e ninguém
    pode formatar um teste congelado; antes da tag `aceite/<fatia>` o
    congelamento não está selado, então o script devolve a pasta ao rascunho
    (e recusa quando a tag existe). `scripts/rascunho.sh` passou a rodar o
    `rustfmt` sobre o rascunho, para isso não se repetir.
14. **`make portoes` da fatia 0 roda com `PORTOES_SEM_DIFF=1
    PORTOES_SEM_MUTANTES=1`.** O portão da fatia é o `make diff-oraculo`, que
    passou com 0 divergências; o `make diff` contra o esqueleto não faz
    sentido ainda, e não há produção para mutar.
12. **Fatia 0 fecha sem `make mutantes`.** As crates de produção estão vazias
    (não há mutante possível); o alvo passa a rodar a partir da fatia 1.

## Fatia 1

15. **O usage de `freitask help` fica byte a byte o de hoje; `show`, `version` e
    `help --json` não entram nele.** A spec (§2.1) diz "o usage de hoje" e o
    diferencial compara o `help` em texto; acrescentar linhas quebraria a
    paridade sem entrada em `divergencias.toml` (D10 cobre só o que tem cara de
    comando novo). Os três estão em `docs/freitask.md`, em `freitask <sub>
    --help` (`usage::EXTRAS`) e na gramática de `help --json`.
16. **`kebab` aceita texto que começa com `-`.** O aceite da fatia 1 pede
    `freitask kebab "--foo--bar--"` → `foo-bar`, caso que o Lua recusava por
    acidente (o `parse_flags` tratava o texto como flag). Não há flag em
    `kebab` além de `--help`, então o posicional ganha `allow_hyphen_values`, e
    `--help`/`-h` continuam sendo ajuda por uma checagem explícita no
    subcomando. Nenhum cenário do diferencial passa texto assim.
17. **`FREITASK_TODAY` inválida é erro de uso (saída 2) em qualquer comando.**
    A variável é lida uma vez em `cli::run`; aceitar em silêncio um valor que
    não é data faria o painel sair com o dia errado sem aviso.
18. **`bloqueada-por` é gravada crua, sem aspas.** O `yaml_scalar` de hoje põe
    aspas em valor que começa com `[`, o que faria `[a, b]` virar a string
    `"[a, b]"` para o Obsidian. `md::set_frontmatter_com(.., citar=false)`
    grava a lista como lista; as outras chaves seguem pelo `yaml_scalar`.
19. **Mensagens de erro do clap só onde o Lua não tinha mensagem própria.** Os
    posicionais são `Option` e a CLI emite as mensagens de hoje (`falta o
    texto`, `falta o projeto`…); flags com valor têm `num_args = 0..=1` para
    que `--volta-em` sozinho vire `falta o valor de --volta-em`, como hoje.
    Flag desconhecida e argumento a mais ficam com a mensagem do clap (spec
    §2.1).

## Fatia 2

20. **Rascunho corrigido antes de congelar: `desc` ausente é `""`, não `null`.**
    O autor do aceite da fatia 2 decidiu `null`; o JSON de hoje (medido no
    oráculo) emite `"desc": ""`, e a spec §2.2 exige a mesma forma. A correção
    foi pedida ao próprio autor enquanto o rascunho ainda não estava congelado
    (a tag `aceite/f2-leitura` sela só depois); fica registrada aqui por
    transparência, com o motivo.
21. **Comandos que escrevem (`new`, `set`, `rebuild`, …) criam `projects/` e
    `.freitask/status.json` quando faltam, como o `ensure_root` de hoje.** A
    leitura pura (I6, D7) é só das leituras e do `doctor` sem `--fix`; o
    `rebuild` é escrita, o oráculo cria os dois, e nenhuma entrada de
    `divergencias.toml` cobre um `rebuild` que não os crie. Paridade.
22. **`list`, `show` e `projects` não criam nada, e o oráculo também não.**
    Medido no `cli.lua`: só `doctor`, `new`, `new-project`, `rebuild`,
    `encerrar` e `reabrir` chamam `ensure_root`. Por isso D7 só se exercita
    nos cenários de `doctor` (fatia 5), não nas leituras.
23. **Regras dos campos novos do `list --json`.** `historico`: as linhas
    `- AAAA-MM-DD — …` da última seção `## Histórico` (a mesma que o
    `append_entry` usa), até a próxima `## seção`. `checklist`: itens `- [ ]`,
    `- [x]`/`- [X]` das linhas depois do bloco (um item dentro do bloco é
    nota). `notes`: as linhas 4+ do bloco sem o `> `, como o parser já lê.
    Arquivo em `tasks/` sem callout entra no `list` com `status_num` 0,
    `title` e `desc` vazios e `callout` do sentinela (`invalid`), como hoje.
24. **Os testes da seção "As dores" (§3.7) moram na pasta da fatia que os
    cobre, com o nome da tabela**, não num `tests/aceite/dores.rs` único: um
    arquivo só não poderia ser congelado por fatia. O relatório final lista
    onde cada um está.
25. **A rodada de mutação da fatia 1 completou apesar da cota de disco.** O
    `cargo mutants` copia a árvore para `$TMPDIR`; o `/tmp` desta máquina é
    tmpfs com cota, e um worker morreu na cópia com "Disk quota exceeded"
    (saída 2 do comando), mas os outros cobriram os 642 mutantes planejados:
    619 mortos, 23 inviáveis, **0 sobreviventes** (`missed.txt` vazio), em
    `core`, `vault` e no binário. O `Makefile` passou a apontar `TMPDIR` para
    `target/tmp` para a próxima rodada não depender do `/tmp`.

## Fatia 3

26. **Os cenários de task existente da fatia 3 são os que a D4 já cobre.**
    O gerador grava task em CRLF, com BOM e sem `\n` final, e a D4 só casa os
    cenários nomeados no `cenario` dela. Por isso o harness ganhou só
    `escrita-basica` e `recusas-escrita` (arquivo novo e recusas), e o `set`
    sobre task existente se confere em `escrita-existente` e
    `set-preserva-bytes`, lidos até o primeiro passo de fatia futura.
27. **D7 sai do portão da fatia 3 e fica com a fatia 5.** A D7 é sobre o
    `doctor` sem `--fix`, que só existe na fatia 5; o aceite da fatia 3 não a
    testa.
28. **`new` e `new-project` garantem a raiz antes de validar, como o
    `ensure_root` de hoje; `new-project` não regenera o painel.** Paridade
    medida no oráculo; o aceite da fatia 3 roda as recusas num vault que já
    tem a raiz.
29. **O lock é tomado depois da checagem de argumentos e antes de abrir o
    vault.** Erro de uso (falta argumento, flag sem valor, nada a mudar) não
    espera pelo lock; tudo o que lê o vault para escrever, sim.
30. **`Vault::gravar` cria a pasta pai que falta**, como o
    `escrever_se_mudou` da fatia 2 fazia (o primeiro `daily/`). Os planos do
    `core` continuam pedindo a pasta com `CriarDir` explícito.
31. **`set --prazo` e `set --bloqueada-por` saem 2 com "ainda não foram
    portados"** até as fatias 9 e 10; o clap já os conhece por causa do
    `help --json`.
32. **Reavaliação pedida: o `onde` da D2 não cobre `daily/`.** No cenário
    `ponto-e-ponto-ponto` dos sintéticos 3, 5, 7 e 8, o oráculo aceita
    `new .. diff-y`, roda o `rebuild` e arquiva o `CURRENT.md` de outro dia em
    `daily/<data>.md`; o novo recusa (D2) e não regenera. O manifesto acusa
    `arquivo:daily` e `arquivo:daily/2026-01-02.md` "só em A", fora do `onde`
    congelado (`(tasks|projects)(/.*)?|[^/]+\.md`). É consequência direta da
    D2, não muda em fatia nenhuma, e faria o diferencial completo da fatia 8
    falhar. Um subagente novo reavalia a entrada contra a spec.

## Fatia 4

33. **O plano de um deslocamento é `CriarDir` (se falta), `Mover`, a
    gravação do conteúdo novo no endereço novo e uma gravação por nota que
    cita a task**, nessa ordem, como hoje (arquivo primeiro, links depois,
    §1.5). `Mover` é um `renameat2(RENAME_NOREPLACE)` com CAS contra os bytes
    lidos; num sistema de arquivos sem `NOREPLACE`, `link` + `unlink` (o
    `link` também recusa destino que existe). A gravação seguinte faz CAS
    contra os mesmos bytes, já no nome novo: uma falha entre as duas deixa o
    conteúdo antigo com o nome novo (`rename_e_atomico`, `59875bf`), que o
    `doctor` acusa como `bloco-dessincronizado`.
34. **As notas em que se procuram referências são os `.md` não ocultos do
    vault, menos `daily/`, `CURRENT.md` (o `ref_candidates` de hoje),
    `*.sync-conflict-*` (I7) e symlink.** Symlink fica de fora porque
    regravar por cima dele com `rename` o trocaria por um arquivo comum; o
    `glob()` de hoje o seguia. Nenhum corpus do diferencial tem symlink.
35. **Link em frontmatter continua sendo reescrito, como hoje.** A spec §1.4
    diz "frontmatter fica fora do scanner", mas não há entrada em
    `divergencias.toml` para isso e o aceite da fatia 4 o deixou de fora. O
    vault real tem 530 wikilinks curtos em frontmatter de 172 notas
    (`related:` de ADRs e reviews, alguns com id de task): tirá-los do
    scanner faria o `rename` deixá-los pendurados. Fica a paridade até o
    dono decidir.
36. **I1, I2 e I5 rodam no crate `vault`** (`tests/deslocamentos.rs`), sobre
    vaults do gerador gravados num diretório temporário, com os planos do
    `core` e o executor de verdade. I5 vale para task com bloco canônico e
    sem `id:` de frontmatter diferente do nome do arquivo: o primeiro
    deslocamento reserializa o bloco e alinha o `id:`, como hoje (é o que o
    `doctor --fix` faria). Conferido que as duas propriedades falham com a
    reescrita de referências desligada e com uma linha do corpo apagada.
37. **Reavaliação pedida: D1 e D5 não cobrem o aviso em stderr.** No
    diferencial, `archive`, `unarchive`, `rename` e `move` imprimem
    `N referência(s) atualizada(s) em M arquivo(s)` com N e M diferentes do
    oráculo quando a task é citada em tabela (D1: o novo reescreve, o
    oráculo não) ou em código (D5: o contrário). O `onde` das duas não
    inclui `stderr`, e o `cenario` da D5 não inclui
    `patologico-link-em-codigo::escrita-existente`. Os arquivos ficam iguais
    no fim, a menos de D4. Um subagente novo reavalia as duas entradas.

## Testes reavaliados

- **D2 (`divergencias.toml`), fatia 3.** Pedido na entrada 32. Veredito do
  subagente reavaliador: procede (§3.4 item 5; o `daily/` "só em A" decorre
  da recusa aprovada, conferido no oráculo num HOME temporário). Mudança: o
  `onde` ganhou `daily(/[^/]+\.md)?` (mais estreito que o `daily(/.*)?`
  proposto, porque o oráculo só grava um `.md` direto em `daily/`) e o
  `motivo` passou a dizer que o oráculo regenera o painel. O `cenario` segue
  limitado a `::ponto-e-ponto-ponto$`. Hash regravado no mesmo commit.
- **D1 e D5 (`divergencias.toml`), fatia 4.** Pedido na entrada 37.
  Veredito do subagente reavaliador: procede com ajuste. Ele refez as
  trajetórias do oráculo e do novo passo a passo, e os 26 avisos fora da
  lista vêm só de link em tabela (D1) e em código (D5). Mudanças: as duas
  entradas ganharam `stderr` no `onde`, e o `detalhe` ganhou só os dois
  formatos medidos do aviso, sem aceitar "1 linhas em A, 0 em B" (que
  esconderia um aviso perdido); o `cenario` da D5 ficou `::codigo$`, porque
  a alternativa `escrita-existente` sempre caía na D1, que vem antes com o
  mesmo `onde`; o `teste` da D5 continua vazio, porque o aceite da fatia 0
  exige o nome da tabela §3.7, que não tem linha para a D5. Rodado com a
  lista nova: 26 diferenças a menos fora da lista e nenhuma a mais. Hash
  regravado no mesmo commit. O reavaliador também achou um bug fora da
  lista: crase sem par escondia o resto da linha da reescrita, e o `doctor`
  de hoje só pula crases em par. Corrigido no scanner (`links::scan_linha`),
  com teste; o vault real tem 11 linhas assim.
