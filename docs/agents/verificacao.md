# Verificação

Adaptador da skill `verificar` neste repo. O bloco abaixo é lido pelo script da
skill: globs do que mudou, dois espaços, o comando na raiz do repo. A lista e o
porquê de cada portão estão em `docs/freitask-internals.md`, "Ao mexer".

```portoes
Cargo.*|crates/**|tests/**/*.rs|Makefile|scripts/**  make check && make test
Cargo.*|crates/**|tests/aceite/**|tests/gerador/**|Makefile|scripts/**  make aceite
crates/**|tests/aceite/**|scripts/**  scripts/hash-aceite.sh --conferir
lua/**|tests/*.lua|tests/run.sh|tests/runner.lua  tests/run.sh
lua/**|tests/*.lua    luacheck lua tests
tui/**                cd tui && cargo test
tui/**                cd tui && cargo fmt --check && cargo clippy --all-targets -- -D warnings
lua/**                freitask doctor
```

As linhas de `lua/**` e `tui/**` valem enquanto essas pastas existirem (até o
cutover, `docs/freitask-internals.md`, "Até o cutover").

## Como testar à mão

- TUI: abrir com `cargo run -p freitask` ou, depois de `make build`, com
  `target/debug/freitask`, num terminal, e exercitar a tecla ou a tela que
  mudou — o desenho só se verifica olhando. O vault é o de `$HOME`: para tecla
  que escreve, use um `HOME` temporário, como o aceite monta
  (`docs/agents/aceite.md`, "Como o teste roda o binário"), e o binário direto,
  não o `cargo run`.
- `doctor --fix`: num vault descartável, com `HOME` temporário, como o aceite e
  `tests/cli_spec.lua` montam.
