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
35. **Link em frontmatter é link de verdade e se reescreve** (decisão do
    dono, 2026-11-08). `archive`, `rename` e `move` reescrevem `[[…]]` no
    frontmatter das outras notas, como o Obsidian faz no rename dele, e não
    por paridade com o Lua. A spec §1.4 ("frontmatter fica fora do scanner")
    contraria isso: com ela, o `rename` deixaria pendurados os wikilinks
    curtos de frontmatter (530 em 172 notas no vault real, `related:` de ADRs
    e reviews). Emenda da §1.4 pendente, a cargo do dono (a spec mora no
    vault). Sem divergência: o oráculo faz o mesmo.
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

## Fatia 5

38. **O `doctor` é um `core::doctor::diagnosticar` puro sobre uma
    fotografia do vault** (as notas com os bytes, os `*.sync-conflict-*` e os
    temporários), que devolve os achados e o plano com CAS contra os bytes
    lidos. Sem `--fix` ele não toma o lock nem chama `garantir_raiz`: é
    leitura, e um vault sem `projects/` dá "vault consistente" sem criar
    nada. Com `--fix`: lock, raiz, plano, painel. A ordem dos checks e o texto
    das mensagens são os do `doctor.lua`; o `msg` do `id-duplicado` usa
    `freitask_core::lua_q`, que imita o `%q` do LuaJIT (conferido no `nvim`).
39. **A marca `[reconstruído pelo doctor; data real desconhecida]` vai na
    própria linha do histórico que o `--fix` reconstrói.** O `doctor.lua:253`
    a cola na última linha do arquivo, que é outra seção quando o histórico
    não é a última: o texto da marca acaba fora do histórico. O novo
    acrescenta a linha inteira (data, destino, rótulo, marca) no fim da seção
    Histórico. Sem entrada no `divergencias.toml`: nenhuma diferença fora da
    lista no diferencial vem daí.
40. **O que o `doctor` lê.** Desce em todas as pastas menos as ocultas
    (`.freitask/` é a exceção, só para temporário órfão) e não segue symlink
    de pasta (um laço não trava a varredura); segue symlink de arquivo. As
    notas incluem `daily/`, `CURRENT.md` e os `*.sync-conflict-*`, como o
    `glob` de hoje; o `link-pendurado` pula `daily/`. Temporário órfão é
    `.freitask.*.tmp` (o formato da decisão de escrita) em qualquer pasta
    visitada.
41. **`sync-conflict` acusa qualquer extensão em qualquer lugar sob
    `projects/`, inclusive `_encerrados/`** (I7). O `doctor.lua` só olha
    `projects/*/tasks/**.md`. A diferença não aparece no diferencial (nenhum
    corpus gerado tem conflito fora de `tasks/`, conferido com `find`), por
    isso não tem entrada. O
    `encerrar` usa o mesmo predicado na D3.
42. **`md::tem_entrada_de_arquivamento` confere só dígitos na data**
    (`- dddd-dd-dd — arquivada em`), como o padrão Lua `%d%d%d%d%-%d%d%-%d%d`.
    A versão da fatia 4 exigia data válida, e uma entrada `2026-13-45` virava
    `historico-ausente`, o que o `--fix` "repararia" com uma segunda entrada.
43. **Task que não é UTF-8 fica fora dos checks por task** (callout, bloco,
    frontmatter, histórico), como fica fora do índice desde a fatia 2. Ela
    ainda conta no `sync-conflict`, no `fora-do-padrao` e no `id-duplicado`.
    Nenhum corpus tem task assim.
44. **`encerrar` e `reabrir` são `core::projeto`; o plano é `CriarDir` (se a
    mãe falta), `MoverPasta` e uma gravação por nota que cita o projeto.**
    `MoverPasta` é efeito novo: `renameat2(RENAME_NOREPLACE)` da pasta
    inteira, com recusa se o destino existe (`Conflito`), e `fsync` das duas
    mães. Sem `NOREPLACE`, confere com `symlink_metadata` e faz `rename` (a
    janela entre as duas fica sob o lock; `link` não serve para pasta). A
    reescrita é `links::rewrite_prefix` de `projects/<p>/` para o endereço
    novo sobre `Vault::notas()`, e a nota que mora dentro da pasta movida é
    gravada no endereço novo com CAS contra os mesmos bytes. A recusa da D3
    vem depois da de task ativa, na ordem dos checks do `close_project`:
    quem tem task ativa recebe a mensagem que já conhece.
45. **O harness compara os achados do `doctor` como multiconjunto, e não
    ordenados** (substitui a canonização da decisão 7). Ordenar e comparar
    posição a posição fazia um achado a mais num lado deslocar todos os
    seguintes: 2086 diferenças fora da lista no primeiro diferencial parcial,
    quase todas eco de uns poucos `link-pendurado`. Agora cada achado que só
    um lado tem vira uma diferença `achado só em A: …` ou `achado só em B: …`
    (em `json:/findings` e em `stdout`, onde o par de linhas vira uma só), e o
    resto do JSON e do texto se compara como antes. A chave de cada achado
    é o objeto inteiro, no formato do texto (`level` alinhado em 6, `kind`,
    `path`, `msg`, `fixed`, valor não string em JSON, chave ausente `∅`,
    chave fora do contrato como `+k=v`). Uma primeira versão deixava `level`
    e as chaves extras de fora e engolia o recuo da segunda linha do texto;
    o auditor e a skill `verificar` acharam, e os testes do `comparar.rs`
    prendem a correção. `make diff-oraculo` segue com 0 divergências nos 290
    cenários.
46. **Reavaliação pedida: D1 e D3 não cobrem as consequências medidas na
    fatia 5.** Com o `doctor` portado, o diferencial parcial da fatia acusa
    889 diferenças fora da lista, todas de duas causas:
    - D1 (link `[[…\|…]]` em tabela): o `doctor.lua` corta o alvo no
      primeiro `|` e acusa `link-pendurado` com alvo terminado em `\`; o novo
      usa a regra do `links::scan` e não acusa. Daí o achado "só em A", a
      saída 1 contra 0, `ok` false contra true e a linha de resumo, nos
      cenários `doctor`, `doctor-fix`, `fix-idempotente`, `rebuild`,
      `escrita-nova` e `recusas` de todo corpus com tabela (o `cenario` de
      hoje é só `::(tabela|escrita-existente)$`).
    - D3 (`encerrar` recusa `sync-conflict`): no cenário
      `encerrar-com-conflito`, o oráculo encerra e o novo recusa; o
      `list --json --archived` seguinte tem tamanho diferente e o
      `CURRENT.md` regenerado só existe em A. O `onde` de hoje não tem
      `json:` na raiz nem `CURRENT.md`.

    Rodado com a lista candidata: 0 fora da lista nos cenários da fatia e
    nos das fatias anteriores. Um subagente novo reavalia as duas entradas e
    a decisão 45 (veredito em "Testes reavaliados").

47. **I3 roda no crate `vault`** (`tests/doctor.rs`), sobre vaults do
    gerador (com um temporário órfão a cada três sementes), com o
    `diagnosticar` do `core` e o executor de verdade; o painel fica de fora
    (é a I4). A propriedade: o segundo `--fix` tem plano vazio, nada
    `fixed` e deixa a árvore igual; depois do `--fix` só sobram os kinds sem
    reparo (`sync-conflict`, `link-pendurado`, `sem-callout`,
    `fora-do-padrao`, `id-duplicado`, `status-0`, `prioridade-desconhecida`,
    `volta-em-invalido`), e nenhum achado deles aparece por causa do fix. Não
    é "os mesmos de antes": o `link-pendurado` da linha do link no bloco some
    quando o bloco é reserializado com o endereço certo (semente 1). A lista
    da spec (§3.2) é mais curta porque cita `dependencia-ciclica`, da fatia
    9, e não conta os avisos que o `doctor.lua` também nunca repara. Conferido
    que a propriedade falha com `tem_entrada_de_arquivamento` sempre falsa.

## Fatia 6

48. **`current` é portado nesta fatia.** A §5 não o põe em fatia nenhuma, mas
    a §2.1 o lista como subcomando do binário e o cutover (§6) troca o shim,
    que hoje o trata, pelo binário: sem porte, `freitask current` deixaria
    de funcionar no dia do cutover. A fatia 6 é a da CLI. Faz o que o shim
    faz: `exec nvim "$HOME/ObsidianVault/CURRENT.md"`, sem `--clean` e sem
    conferir que o arquivo existe; `nvim` fora do `PATH` dá a mensagem do
    shim (`nvim não encontrado no PATH`, saída 2), e outra falha do `exec`
    sai 2 com o erro. O aceite só cobre `current --help`; o comportamento
    fica preso em `crates/freitask/tests/current.rs` (um `nvim` falso
    confere o argumento e que é `exec`, não filho). Diferenças do shim:
    `FREITASK_TODAY` inválida sai 2 antes do `exec` (decisão 17), e o
    `nvim` sem permissão de execução sai 2 com mensagem, não 126 do bash; e
    argumento a mais (`current x`) é recusado pelo clap com saída 2, onde o
    shim o ignorava (decisão 19: argumento a mais fica com o clap).
49. **`freitask tui` e `freitask` sem argumentos são uma função só**: TUI
    num terminal (stdin e stdout), senão o usage em stdout e saída 2. Num
    terminal, até a fatia 7, a TUI saía 2 com "ainda não foi portada"; a
    fatia 7 a trouxe (decisões f7.1 a f7.8).
50. **`new --json` tira `project` e `id` do caminho que o plano grava**
    (`split_task_path`), não dos argumentos: o `nova_task` apara o id.
51. **Flag desconhecida no `doctor` não aparece no diferencial.** O aceite
    da fatia 6 fixa a saída 2 do clap (§2.1) onde o oráculo ignora a flag e
    roda; medido em `tests/diferencial/src/cenarios.rs`: nenhum cenário
    passa flag fora de `--archived --desc --fix --help --json --quiet
    --status --title --volta-em`, todas conhecidas dos dois lados. Sem
    entrada no `divergencias.toml`.
52. **`scripts/rascunho.sh` roda `cargo clippy --all-targets -D warnings`**
    no rascunho, com os `[lints]` copiados de `tests/aceite/Cargo.toml`, antes
    do `cargo test`: o congelamento da fatia 6 teve de ser desfeito por um
    `clippy::useless_concat` que só o `make check` pegou.
53. **Reavaliação pedida: D1 no `help-por-subcomando`, D10 e a `tambem`
    geral da D1.** Com o `new --json` portado, a rodada completa do
    diferencial deu 290 cenários, 52237 diferenças, 66 fora da lista: as 83
    previstas sumiram, e ficaram 60 `json:/findings` (`link-pendurado` com
    alvo terminado em `\`) e 6 `json:/ok` (`A=false B=true`), todas no passo
    11 (`doctor --json`) do `help-por-subcomando`; o `codigo` desse passo cai
    na D8. O `diff-json.md` não diverge mais (os dois lados gravam a mesma
    task), então o "só em B" do `motivo` da D10 e a alternativa
    `arquivo:…diff-json\.md` do `onde` estão mortos. Pedido a um subagente
    novo: (1) a forma de pôr o cenário na D1; (2) o `motivo` e o `onde` da
    D10; (3) se a `tambem` geral da D1, que cobre `codigo ^A=1 B=0$` em
    `recusas`, esconde uma recusa aceita em silêncio. O primeiro reavaliador
    foi interrompido com a sessão antes de dar veredito; deixou uma lista
    candidata e um binário de embrulho (recusa que sai 0) em
    `target/f6-reavaliacao/`, sem rodada completa com ela.

## Depois da fatia 6: fatias em paralelo

54. **As fatias 9 e 10 se dividem em parte da CLI (a) e parte da TUI (b), e
    o que vem depois da 6 corre em ondas paralelas**, um chat por worktree.
    Decisão do dono (10/11/2026), para não fazer 7 → 8 → 9 → 10 → 11 em
    série; altera a §5, que divide em fatias seriais numa branch só. O
    porquê da divisão: a leitura das duas já está portada (`core/deps.rs`,
    `meta.rs`, o `show` em `cli.rs`), e só as teclas `b` e `p` dependem da
    TUI da fatia 7. Os testes da TUI chamam `update`/`view` em processo
    (§3.5): congelar o aceite da tecla antes da 7 existir dá um alvo que não
    compila, e erro de compilação aborta o `cargo test -p aceite
    --no-fail-fast` e o `clippy --all-targets` do `make check` inteiros,
    inclusive as fatias 0 a 6. Por isso cada parte tem a sua pasta de
    aceite e a sua tag.
    - **Onda 1**, logo depois da 6: `f7-tui` (a fatia 7 inteira);
      `f9a-dependencias-cli` (`set --bloqueada-por` com recusa de ciclo,
      `rename` que arrasta, campos no JSON, os kinds do `doctor`, I10,
      `set_bloqueada_por_recusa_ciclo_e_nao_grava`,
      `rename_arrasta_a_dependencia_nas_outras_tasks`);
      `f10a-prazo-cli` (`set --prazo`, `dias_para_o_prazo`, os kinds do
      `doctor`, I11, `prazo_invalido_nao_conta_dias_e_o_doctor_acusa`,
      `prazo_nao_muda_a_ordem_do_painel`).
    - **Onda 2**, depois do merge da 7: a fatia 8 no branch integrado, e
      `f9b-dependencias-tui` com `f10b-prazo-tui` (tecla `b` e a marca;
      tecla `p` e o prazo na tela), congeladas só agora.
    - **Onda 3**: a 11, sozinha.

    A 9 e a 10 só fecham (portão da §5) quando a parte b fecha. Nas ondas 1
    e 2 cada chat roda `make check test aceite` (e `make diff` na parte a);
    `make mutantes` e `make test-longo` ficam para o branch integrado (três
    `target/` com `--jobs 4` disputam CPU e disco). No merge, o
    `tests/aceite/Cargo.toml`, o `ACEITE.sha256` e as duas catracas
    conflitam sempre; não se editam à mão: o dono regera com
    `scripts/gerar-cargo-aceite.sh`, `scripts/hash-aceite.sh --gravar` e
    `make baseline`, e confere com `git diff --numstat
    aceite/f0-fundacao..HEAD` sobre o aceite que nenhum arquivo das pastas
    `f*/` perde linha. `ACEITE.sha256` e `contagem.baseline` trocam linha a
    cada congelamento, e `divergencias.toml` muda nas reavaliações; esses
    três ficam fora da conferência.

## Fatia 7

As entradas desta fatia se numeram `f7.N`: as fatias da onda 1 correm em
worktrees paralelas (decisão 54), e um número corrido colidiria no merge.

f7.1. **O aceite da TUI fala com um contrato mínimo em processo,
    `freitask::tui::Sessao`, e com o binário num PTY.** A §3.5 descreve os
    testes do implementador (`update`, `view`, propriedades, snapshots) e a
    decisão 54 supõe que o aceite da TUI chama a TUI em processo; mas quem
    escreve o aceite não vê a implementação, e congelar `State`, `Evento` e
    `enum Comando` com os campos prenderia o desenho interno antes de ele
    existir. O contrato fica em três funções e um enum
    (`crates/freitask/src/tui.rs`): `Sessao::abrir(home, hoje)` abre como a
    TUI abre (`rebuild`, depois a leitura); `tecla(KeyEvent) -> Pedido`
    atualiza o estado e grava no vault o que a tecla manda; `tela(largura,
    altura) -> Buffer` desenha; `Pedido` é `Nada`, `Sair` ou
    `Abrir(PathBuf)`, o que precisa do terminal. O aceite afirma o que a
    pessoa vê (a grade, com estilo) e o que o vault guarda, não a forma
    interna do estado; `update` e `Comando` ficam livres e são testados pela
    §3.5 no `make test`. O que só o binário prova (o terminal volta do modo
    raw, `$NVIM`, `$EDITOR` e o `rebuild` na volta) vai num PTY com
    `portable-pty` e `vt100`. O pacote `freitask` ganhou um alvo `[lib]`, e o
    `aceite` ganhou as dependências `freitask`, `ratatui`, `portable-pty` e
    `vt100` (`scripts/gerar-cargo-aceite.sh` e `scripts/rascunho.sh`); no
    merge, o `Cargo.toml` do aceite se regera pelo script, como a decisão 54
    já manda. Quem escreve o aceite lê também `tui/src/`, a TUI de hoje: ela
    é o oráculo da paridade (§1.7) e não há diferencial para a tela.

f7.2. **Os comandos que escrevem passam a receber o vault e o lock num
    `Contexto` e a devolver um `Resultado`; a CLI só imprime.** A TUI da
    spec (§1.7) não chama subprocesso, então `new`, `new-project`, `set`,
    `delete`, `rebuild`, `archive`, `unarchive`, `rename`, `move`, `encerrar`
    e `reabrir` não podiam seguir imprimindo, devolvendo `ExitCode` e lendo
    `HOME`. Forma: `escrita.rs`, `deslocar.rs`, `linhas.rs` e o novo
    `saida.rs` saíram do binário para o `[lib]` (continuam no mesmo diretório;
    `cli.rs`, `doctor.rs` e `usage.rs` ficam no binário). Cada comando é
    `fn(&Contexto, …) -> Resultado`, com `Contexto { vault, trava, hoje }` e
    `Resultado = Result<Saida, Falha>`: `Saida` é a linha de stdout mais os
    avisos que foram a stderr no caminho; `Falha` é tudo que foi a stderr,
    em ordem, sem o `freitask: `, mais o código de saída. Um `Corrida`
    acumula o stderr enquanto o comando roda (o aviso de `status.json`
    quebrado, o aviso de referências reescritas, a primeira linha de uma
    recusa de `archive`), para a ordem das linhas ser a de sempre. O
    embrulho da CLI (`cli.rs`, `escrevendo` e `imprimir`) resolve o lock e o
    vault do ambiente, chama o comando e imprime o prefixo, os avisos, a
    linha e o código: a saída dos subcomandos não muda, e o aceite das
    fatias 0 a 6 é o que prova. A TUI chama as mesmas funções com o vault de
    `<home>/ObsidianVault` e o lock de `saida::caminho_da_trava(Some(home))`;
    a barra mostra `Falha::para_a_barra`, as linhas aparadas e juntas
    com ` · `, como o `engine.rs` de hoje fazia com o stderr da CLI (uma
    recusa de `archive` mostra então o motivo e "operação não concluída (veja
    as mensagens acima)", como hoje). `Vault::travar` saiu do `vault` (só a
    CLI o usava; o caminho do lock agora vem de `saida::caminho_da_trava`,
    com a mesma mensagem sem `XDG_RUNTIME_DIR` nem `HOME`). `kebab` virou
    `escrita::kebab`, e é o que a TUI chama para sugerir o id. O `doctor`
    fica no binário, com os seus embrulhos em `cli.rs`. Para o merge com as
    fatias 9a e 10a: elas tocam o braço `Set` de `cli.rs` e `escrita::set`,
    e vão conflitar com esta forma; a resolução é levar a mudança delas para
    a função que recebe o `Contexto`.
    Uma ordem mudou: a CLI resolve o vault (`HOME`) antes de esperar o
    lock. Sem `HOME`, com `XDG_RUNTIME_DIR` definido e o lock ocupado, saía
    3 depois da espera e agora sai 2 na hora; o caso não aparece em nenhum
    cenário do diferencial.
f7.3. **A `Sessao` que não abre sai 1; sem `HOME` num terminal, 2.** `Sessao::
    abrir` só falha com `hoje` inválido ou sem lugar para o lock, e o
    primeiro caso a CLI já recusou antes (saída 2, `FREITASK_TODAY` inválida,
    §2.1). O que sobra é "a TUI não conseguiu abrir", como o `freitask-tui:`
    de hoje (saída 1), com a mensagem em stderr no formato `freitask: …`.
    Terminal que falha (`ratatui::try_init`, também na volta do editor, ou a
    leitura de evento) também sai 1, e o terminal é devolvido (`ratatui::restore`) nos
    dois caminhos. `HOME` não definido é erro de uso, 2, como em `current` e
    nos outros comandos. Fora de terminal segue o usage com saída 2 (decisão
    49).
f7.4. **`update` devolve `Comando`s e os resultados voltam como `Evento`s.**
    `update(State, Evento) -> (State, Vec<Comando>)`. Um `Comando` é o que o
    laço executa contra o vault: `Executar` (a `Escrita` que a CLI recebe,
    montada como o `app.rs` montava os argumentos, mais o que selecionar
    depois e a mensagem de sucesso), `Reler`, `SugerirId`, `LerPreview`,
    `AbrirCurrent`, `Abrir` e `Sair`. O executor (`Sessao`) alimenta o
    `update` com o resultado: `Feito` (que mostra o ok e pede `Reler`),
    `Falhou` (que devolve o modal preenchido, se havia um em voo, e mostra o
    erro), `Dados`, `IdSugerido` e `PreviewLido`, até a fila esvaziar; só o
    que sai do processo (`Sair`, `Abrir`) vira `Pedido`. A leitura do
    preview é um `Comando` que o próprio `update` pede quando a seleção
    muda, uma vez por seleção; `Dados` esquece o preview, então ele é lido
    de novo depois de toda releitura e de toda escrita. `Resize` é um
    evento sem efeito no estado (a tela não depende do tamanho; o laço
    redesenha a cada passo), e existe para o laço e a propriedade da §3.5
    o tratarem como qualquer outro. `R` mostra "relido" no mesmo `update` em
    que pede `Reler`, porque a leitura não falha (o `Dados::ler` devolve
    vazio onde o vault não tem nada, como o `list`).
f7.5. **Uma tabela só (`ATALHOS`) para a ajuda e para o despacho do modo
    normal.** Cada linha traz o rótulo da ajuda, a descrição e as teclas com
    o gesto de cada uma; a ajuda desenha as linhas e o `update` acha o gesto
    pela mesma tabela. O teste `toda_tecla_da_ajuda_tem_handler` afirma as
    duas direções: a tecla listada tem o gesto da tabela e o rótulo a cita; e
    a tecla que a tabela não lista (ASCII imprimível, Ctrl-letra e as
    especiais da crossterm) não muda o estado nem pede nada. Consequência:
    o que o `app.rs` tratava sem listar entrou na ajuda, que ganhou `Home/End`
    no rótulo de `g/G`, uma linha para `Esc` ("limpar o filtro e o aviso") e
    uma para `?`; nenhum texto que o aceite fixa mudou, e a ajuda continua
    cabendo em 100×30 (23 linhas). `b` e `p` das fatias 9b e 10b entram
    depois, como linhas novas da tabela.
f7.6. **O que a TUI mostra sai dos tipos do `core`, não de JSON.** `linhas.rs`
    ganhou `Resumo` (os campos da task que o `list --json` e a TUI usam,
    calculados num lugar só; `linha` o serializa) e `projetos` (o `active` do
    `projects --json`). A TUI guarda o `Vocab` do `core`: as cores e os
    ícones vêm de `Vocab::meta`, e as fases que o `s` oferece são as que o
    `core` aceitaria num `set` (`task::check_status_change`, o que recusa
    `done` e o sentinela 0), em vez do filtro `callout != "done"` da TUI
    de hoje; o menu de arquivar vem de `config::ARCHIVED_TYPES` e
    `archived_label`. Única diferença visível: uma task cujo número de fase
    o vocabulário não tem mostra o ícone do sentinela, em vermelho, no lugar
    de `?`. Nenhuma regra de task mora na TUI.
f7.7. **`Sessao::abrir` e o laço.** Ao abrir, a sessão toma o lock para o
    `rebuild` (é o mesmo `rebuild` da CLI, que o toma), lê o vault e, se o
    `rebuild` falhou, deixa `rebuild falhou: <motivo>` na barra, em vermelho.
    A leitura do `Dados` vem depois do `rebuild`, como a spec manda. O
    `$NVIM` e o `$VISUAL`/`$EDITOR` seguem o `main.rs` de hoje (a expressão
    `luaeval` e os testes foram portados); na volta do editor a sessão
    regenera o painel (ignorando a falha, como hoje) e relê
    (`Sessao::voltou_do_editor`). O laço ignora o evento de tecla que não é
    `Press`. A sessão não lê `HOME`; o lock lê só o `XDG_RUNTIME_DIR` do
    ambiente, e os testes em processo o compartilham com os outros testes
    do mesmo binário (o lock é curto e o `flock` serializa entre
    descritores do mesmo processo).
f7.8. **Os testes da §3.5 no `make test`.** Camada 1: os 21
    testes do `app.rs`, portados, e 16 novos (o `Comando` como fronteira, o
    preview, os resultados que voltam como evento). Camada 2: duas
    propriedades com `proptest` (sequências de teclas, `Resize` de 20×5 a
    200×60, `Falhou`, `IdSugerido` e `Dados` sobre dados gerados: sem
    pânico, índices dentro da lista, modal coerente e dois `Esc` voltam ao
    modo normal; e o desenho em qualquer tamanho e modo). Camada 3: um teste
    de `view` por tela em `TestBackend` (100×30 e 60×15, mais os tamanhos
    estreitos), com asserções sobre a grade; cada tela tem a sua função
    `tela_*` que devolve o `Buffer` e tira o snapshot dela (f7.9). A camada
    4 é o `pty.rs` do aceite. Ficam fora, para as fatias 9b e 10b, a tela da
    task bloqueada, a do prazo e as teclas `b` e `p`.
f7.9. **Os `.snap` nascem por `scripts/snapshots-novos.sh`, que só cria o
    que falta.** Decisão do dono (10/11/2026). A §3.5 manda o implementador
    criar os `.snap` uma vez, mas todas as portas do insta estão fechadas
    pela trava: o `make` exporta `CI=true` e `INSTA_UPDATE=no` (snapshot que
    falta só falha), o `settings.json` nega `Write` em `**/snapshots/**` e o
    hook nega `INSTA_UPDATE=`, `insta accept` e `--accept`. Para a trava, o
    primeiro `.snap` e o `.snap` reaceito depois de uma regressão são o
    mesmo gesto. O script roda o `cargo test` sem `CI` e sem as variáveis do
    insta, renomeia cada `.snap.new` que não tem `.snap` ao lado e recusa
    (apaga e sai 1) o que tem: snapshot que mudou continua falhando no `make
    test` e só muda por reavaliação (§4, camada 1). Repete até uma rodada
    não criar nada, porque o insta para no primeiro snapshot que falta de
    cada teste. Cada tela tira dois, em 100×30 e 60×15 (`foto` em
    `tui/tela.rs`); são 24, lidos um a um antes do commit.

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
- **D1, D3 e o harness do `doctor` (`divergencias.toml`,
  `tests/diferencial/`), fatia 5.** Pedido nas entradas 45 e 46. O subagente
  reavaliador montou um emulador do `cobrir` que reproduz os dois relatórios
  exatamente (congelada: 889 fora; candidata: 0) e testou 11 regressões
  fictícias. Os vereditos:
  - **Harness: procede com ajuste.** A chave do multiconjunto não perde
    nada (depois da correção da decisão 45), mas o prefixo `só em B: `
    colidia com o `^só em B` da D10 (`cenario = '.*'`): qualquer achado a
    mais no novo, de qualquer kind, ficaria coberto, inclusive o
    `link-pendurado` de uma regressão da própria D1. Antes ele virava
    `tamanho A=n B=n+1`, fora da lista. Mudança: o achado sai como
    `achado só em A/B: …`, e o cabeçalho do `divergencias.toml` e o
    `tests/diferencial/README.md` registram a forma.
  - **D1: procede com ajuste.** As 889 diferenças têm duas causas só: 887 da
    D1 (`doctor.lua:327` corta o alvo no primeiro `|`; §1.4 e o teste da D1
    na §3.7 pedem "o `doctor` sai 0") e 2 da D3. Mas a candidata, com
    `cenario`, `onde` e `detalhe` independentes, levava `arquivo:*.md` e
    `stderr` para seis cenários em 21 corpora: cobria 7 das 11 regressões
    fictícias (`CURRENT.md` diferente, bytes do `--fix`, o `doctor` sem `--fix`
    gravando nota) e tirava da D4 as 20 diferenças de CRLF do `doctor-fix`.
    Cortar por corpus não serve: 21 dos 23 corpora não reais têm
    `[[projects/…\|…]]`. Mudança: a lista ganhou cláusulas
    `[[divergencia.tambem]]` (`tests/diferencial/src/divergencias.rs`, com
    teste). A cláusula principal da D1 fica nos cenários congelados, com o
    `arquivo:` restrito a `notas/` e `tasks/`. A `tambem` cobre só
    `codigo`, `stdout` e `json:/(ok|findings)` nos cenários do `doctor`, com
    o achado preso a alvo `projects/<p>/tasks/…\` e o resumo às duas formas
    medidas (`linha 1: A="" B="freitask: vault consistente."` e o par de
    contagens na linha 2). O `motivo` passou a explicar o falso positivo
    sem escrita. Resultado: 9 das 11 regressões fictícias ficam fora da
    lista; as duas que passam (saída e resumo sozinhos) nunca vêm sem o
    `achado só em`, que é avaliado à parte. O teste da fatia 0 aceita a
    chave nova (lê `toml::Table`).
  - **D3: procede.** O `cenario` segue `::encerrar-com-conflito$`; o `onde`
    ganhou a raiz `json:` (`tamanho A=4 B=5` do `list --json --archived`) e
    `arquivo:CURRENT.md` (o `encerrar` do oráculo roda o `rebuild` depois de
    mover; o novo recusa antes). O `motivo` cita o `CURRENT.md`.

  Rodado com a lista nova e o harness ajustado: 216 cenários, 47335
  diferenças, 0 fora da lista; D1 1309, D3 92, D4 20 (volta a ser
  exercitada). Hash regravado no mesmo commit. O reavaliador também notou
  que, no texto, a posição das linhas de achado em relação às de resumo não
  se compara (resíduo de antes, não piorado).
- **D1, cenários `set-preserva-bytes` e `ambiguo` (`divergencias.toml`),
  fatia 5.** A rodada parcial dos dois vereditos anteriores não incluía dois
  cenários que também rodam `doctor --json`. A rodada completa do
  `make portoes` acusou 5 diferenças fora da lista ali, todas o falso
  positivo da D1 em `notas/ref.md` linha 3 (`tests/gerador/src/lib.rs`, o
  `minimo()` de que todo patológico parte). Pedido: pôr os dois cenários na
  `tambem` da D1. Veredito de um subagente reavaliador novo: **procede com
  ajuste**. O `set-preserva-bytes` entra na `tambem` geral: lá os passos de
  `set` e `archive` saem 0 no oráculo, e um `A=1 B=0` não pode vir de
  regressão do novo. O `ambiguo` ganha uma `tambem` própria, só
  `json:/findings` com o mesmo regex do achado, porque a geral levaria
  `codigo ^A=1 B=0$` para a recusa de id ambíguo do passo 1. Ele mediu
  isso com um binário de embrulho que saía 0 nessa recusa: com a candidata a
  regressão sumia, com a ajustada ela fica fora da lista. Resultado na
  rodada completa: 143 fora da lista, todas no `help-por-subcomando`, passos
  11 e 12, depois do `new --json` do passo 10, que o novo ainda recusa
  (`cli.rs`, "ainda não foi portado"). É o critério da fatia 4: 0 fora da
  lista antes do primeiro comando de fatia futura. Hash regravado no mesmo
  commit.

  O reavaliador deixou três pendências para fatias futuras:
  - Na fatia que portar o `new --json`: das 143, 83 somem com o port (o
    oráculo grava a task e roda o `rebuild`, que arquiva o `CURRENT.md` em
    `daily/` e copia para o painel o bloco com endereço errado do
    `sintetico-1`). As outras 60 `link-pendurado` com alvo em `\` e 6
    `ok` continuam: são D1, e o `help-por-subcomando` precisa entrar na D1
    com `onde = '^json:/(ok|findings)$'` (o `codigo` já fica com a D8).
  - O `motivo` da D10 diz que `diff-json.md` existe "só em B", e hoje ela
    aparece "só em A". Isso se acerta na mesma fatia.
  - A `tambem` geral da D1 cobre `codigo ^A=1 B=0$` em todos os passos de
    `recusas`. Um novo que aceitasse em silêncio uma recusa, mantendo a
    mensagem no stderr, passaria. Vale um reavaliador olhar à parte.
- **D1 no `help-por-subcomando`, D10 e a `tambem` geral da D1
  (`divergencias.toml`), fatia 6.** Pedido na entrada 53. Um subagente
  reavaliador novo refez a medição com rodadas completas (base: 290
  cenários, 52237 diferenças, 66 fora) e sete binários de embrulho como
  regressão fictícia. Os vereditos:
  - **`help-por-subcomando`: procede com ajuste.** Uma `tambem` própria, só
    `json:/(ok|findings)`, com o achado preso a alvo `projects/<p>/tasks/…\`.
    Só o passo 11 sai JSON dos dois lados; o resto do cenário segue na D8.
    Pôr o cenário na `tambem` geral tiraria da D8 o `A=1 B=0` do `doctor
    --help`. Medido: com os achados reais sumindo (`r-vazio`), eles ficam
    fora da lista no cenário.
  - **D10: procede.** Nenhuma diferença toca `diff-json.md` (os dois lados
    gravam a mesma task), e a alternativa `--json` do `detalhe` também
    estava morta. O `onde` perdeu `arquivo:…diff-json\.md`; o `detalhe`
    trocou `"freitask: [^"]*(version|show|--json)`, que valia em todo
    cenário, pela forma medida da recusa de `version` e `show`; o `motivo`
    diz o que o oráculo faz com `help --json` e `new --json`.
  - **`tambem` geral da D1: procede, e o buraco era maior que o pedido.**
    Ela e a principal escondiam qualquer recusa que saísse 0 e o `doctor`
    saindo 0 ou dizendo `ok` com erro: com a lista congelada, `doctor
    --quiet` sempre 0 dava 0 fora da lista, e recusas de `move`,
    `unarchive` e afins só apareciam no corpus `real`. Medido: todo `A=1
    B=0` legítimo da D1 é passo de `doctor` em seis corpora onde o falso
    positivo é o único erro do oráculo (`minimo` e os patológicos
    `crlf-bom`, `link-em-codigo`, `link-em-tabela`, `ponto-e-ponto-ponto`,
    `projeto-sem-tasks-e-encerrado`), mais o `archived-true-legado` depois
    do `--fix`; o `ok` e o `vault consistente` têm o mesmo recorte.
    Mudança: `codigo`, `ok` e `linha 1` saíram da principal e da geral para
    uma `tambem` presa a esses corpora e cenários; a principal perdeu
    também `stdout` e as linhas de resumo, mortas.

  Rodado com a lista nova: 290 cenários, 52237 diferenças, 0 fora da
  lista; D1 1380, D10 46500, D2 3553, D3 92, D4 23, D5 6, D7 18, D8 665.
  As sete regressões fictícias ficam todas fora da lista (de 15 a 2181
  diferenças cada; `r-quiet` ia de 0 para 15, `r-recusa-mover` de 13 para
  220), sem diferença nova fora da lista que não venha delas. Resíduo:
  nos seis corpora uma recusa que sai 0 ainda passa, porque o harness não
  filtra cláusula por passo. Hash regravado no mesmo commit.

  O reavaliador deixou, fora do pedido:
  - O harness não filtra cláusula por passo (um `comando` opcional em
    `Clausula`, casado contra `Diferenca.comando`, fecharia o resíduo) e
    só acusa entrada morta, não cláusula nem alternativa morta (por isso
    o `diff-json` sobreviveu). Sem dor medida: não entra agora.
  - A D2 aceita qualquer `codigo` no `ponto-e-ponto-ponto`; quem segura a
    saída 1 do `.` e do `..` é o aceite `ponto_e_ponto_ponto_nao_sao_projeto`.
  - No `escrita-existente`, o passo 8 (`move {task2} {proj1}`) é recusa em
    13 corpora: a `{task2}` já está em `{proj1}`, e o primeiro `move` do
    cenário não move nada.
  - O corpus `real` varia entre rodadas (11 diferenças trocadas entre duas
    bases, todas cobertas por D10 e D2).
