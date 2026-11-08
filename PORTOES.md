# PORTOES.md — gerado por `make portoes` em 2026-11-08T12:21:11-03:00 no commit 5ee9721

| portão | estado | tempo | comando |
| --- | --- | --- | --- |
| check | verde | 0s | `make check` |
| test | verde | 0s | `make test` |
| hash-aceite | verde | 0s | `scripts/hash-aceite.sh --conferir` |
| aceite | verde | 0s | `make aceite` |
| diff | pulado | - | PORTOES_SEM_DIFF=1 |
| mutantes | pulado | - | PORTOES_SEM_MUTANTES=1 |
| catraca | verde | 1s | `scripts/catraca.sh` |

## Aceite congelado

- hash: aa62efec04f309b70638c87efe3c0f609c98b6a8b8a806d461b072460f13ab55
- `git diff --stat <tag>..HEAD -- tests/aceite`: sem tag aceite/* ainda

## Catraca

```
catraca: testes=42 asserts=37
catraca: delta testes=0 asserts=0
```

## Testes

```
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 37 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```
