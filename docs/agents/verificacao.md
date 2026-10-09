# Verificação

Adaptador da skill `verificar` neste repo. O bloco abaixo é lido pelo script da
skill: globs do que mudou, dois espaços, o comando na raiz do repo. A lista e o
porquê de cada portão estão em `docs/freitask-internals.md`, "Ao mexer".

```portoes
*                     tests/run.sh
lua/**|tests/**       luacheck lua tests
tui/**                cd tui && cargo test
tui/**                cd tui && cargo fmt --check && cargo clippy --all-targets -- -D warnings
lua/**                freitask doctor
```

## Como testar à mão

- `tui/**`: abrir a TUI (`freitask` num terminal) e exercitar a tecla ou a tela
  que mudou — o desenho só se verifica olhando.
- `doctor --fix`: num vault descartável, como `tests/cli_spec.lua` faz.
