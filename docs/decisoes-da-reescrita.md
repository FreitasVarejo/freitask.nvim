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
