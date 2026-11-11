# PORTOES.md — gerado por `make portoes` em 2026-11-11T13:31:00-03:00 no commit 85785be

| portão | estado | tempo | comando |
| --- | --- | --- | --- |
| check | verde | 1s | `make check` |
| test | verde | 33s | `make test` |
| hash-aceite | verde | 0s | `scripts/hash-aceite.sh --conferir` |
| aceite | verde | 29s | `make aceite` |
| diff | verde | 95s | `make diff` |
| mutantes | pulado | - | PORTOES_SEM_MUTANTES=1 |
| catraca | verde | 0s | `scripts/catraca.sh` |

## Aceite congelado

- hash: c41ed54f35441e14e2a097640c7c2542a89f29096ae1a68839890cdc82d9754d
- `git diff --stat aceite/f0-fundacao..HEAD -- tests/aceite`:  45 files changed, 24986 insertions(+), 20 deletions(-)

## Catraca

```
catraca: testes=1218 asserts=2640
catraca: delta testes=0 asserts=0
```

## Mutação

```
mutantes: caught=616 missed=0 (core=0, resto=0) timeout=0 unviable=38
```

## Diferencial

```
diferencial: oráculo vs binário; relatório em target/diferencial/relatorio.md
cenários: 290 rodados, 12 pulados
divergências: 52829 no total, 52829 cobertas, 0 fora da lista
entradas: exercitadas [D1, D2, D3, D4, D5, D7, D10, D8], não exercitadas []
```

Relatório completo: `target/diferencial/relatorio.md`.

## Testes

```
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 37 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 38 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
test result: ok. 59 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.06s
test result: ok. 23 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.21s
test result: ok. 116 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.14s
test result: ok. 101 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 5.08s
test result: ok. 103 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 5.08s
test result: ok. 118 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 5.07s
test result: ok. 58 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 5.05s
test result: ok. 64 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.95s
test result: ok. 57 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 5.02s
test result: ok. 34 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.36s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 105 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.39s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 233 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 13 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.15s
test result: ok. 26 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.20s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.58s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.52s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.80s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```
