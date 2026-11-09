# PORTOES.md — gerado por `make portoes` em 2026-11-09T17:12:24-03:00 no commit 946e9ec

| portão | estado | tempo | comando |
| --- | --- | --- | --- |
| check | verde | 0s | `make check` |
| test | verde | 22s | `make test` |
| hash-aceite | verde | 0s | `scripts/hash-aceite.sh --conferir` |
| aceite | verde | 16s | `make aceite` |
| diff | **vermelho** | 11927s | `make diff` (ver target/portoes/diff.log) |
| mutantes | pulado | - | PORTOES_SEM_MUTANTES=1 |
| catraca | verde | 1s | `scripts/catraca.sh` |

## Aceite congelado

- hash: fb0d2bbdb9d286cd0d27cdaa5fb3a498d3e52d482fc619248ac9be369fe73ea9
- `git diff --stat aceite/f0-fundacao..HEAD -- tests/aceite`:  21 files changed, 12383 insertions(+), 12 deletions(-)

## Catraca

```
catraca: testes=788 asserts=1542
catraca: delta testes=0 asserts=0
```

## Mutação

```
mutantes: caught=287 missed=0 (core=0, resto=0) timeout=0 unviable=24
```

## Diferencial

```
diferencial: oráculo vs binário; relatório em target/diferencial/relatorio.md
cenários: 290 rodados, 12 pulados
divergências: 52358 no total, 52215 cobertas, 143 fora da lista
entradas: exercitadas [D1, D2, D3, D4, D5, D7, D10, D8], não exercitadas []
```

Relatório completo: `target/diferencial/relatorio.md`.

## Testes

```
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 37 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 38 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 116 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.11s
test result: ok. 101 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 5.05s
test result: ok. 103 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 5.06s
test result: ok. 118 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 5.10s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 213 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 26 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.20s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.43s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.69s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```
