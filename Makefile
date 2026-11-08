# Alvos de verificação do freitask (spec da reescrita, §3). O agente roda estes
# alvos, nunca `cargo test` à mão: o runner não acusa seleção vazia e a sintaxe
# de filtro do cargo é fácil de errar.
#
# Todos exportam CI=true INSTA_UPDATE=no INSTA_FORCE_PASS=0: nenhum snapshot é
# aceito por acidente. HOME só é trocado pelos próprios testes.

export CI := true
export INSTA_UPDATE := no
export INSTA_FORCE_PASS := 0
export CARGO_TERM_COLOR := never

PROPTEST_CASES ?= 256
export PROPTEST_CASES

BIN := target/debug/freitask

.PHONY: check test test-longo aceite diff diff-oraculo mutantes portoes congelar baseline build help

help:
	@sed -n 's/^## //p' Makefile

## check       fmt + clippy (-D warnings; unwrap/expect negados fora de teste)
check:
	cargo fmt --all -- --check
	cargo clippy --workspace --all-targets -- -D warnings

## build       compila o binário freitask (target/debug/freitask)
build:
	cargo build -p freitask

## test        cargo test --workspace (unitários, propriedades, TUI); PROPTEST_CASES=256
test: build
	cargo test --workspace

## test-longo  o mesmo com PROPTEST_CASES=4096 e a semente impressa
test-longo: PROPTEST_CASES := 4096
test-longo: build
	@echo "PROPTEST_CASES=$(PROPTEST_CASES)"
	cargo test --workspace -- --nocapture 2>&1 | tee target/test-longo.log | grep -E 'seed|test result|FAILED|panicked' || true
	@grep -q 'FAILED\|panicked' target/test-longo.log && exit 1 || true

## aceite      os testes congelados de tests/aceite (crate aceite), inclusive schemas
aceite: build
	cargo test -p aceite --no-fail-fast

## diff        diferencial contra o oráculo Lua (precisa de nvim); DIFF_ARGS passa flags ao harness
diff: build
	cargo run -q -p diferencial -- --novo $(BIN) $(DIFF_ARGS)

## diff-oraculo  o oráculo contra ele mesmo (prova o harness): A e B rodam o Lua
diff-oraculo:
	cargo run -q -p diferencial -- --oraculo-vs-oraculo $(DIFF_ARGS)

## mutantes    cargo mutants no workspace (limiar: 0 missed em core; catraca no resto)
# TMPDIR em target/: o cargo-mutants copia a árvore para um temporário, e o
# /tmp desta máquina tem cota pequena (medido: "Disk quota exceeded").
mutantes: build
	mkdir -p target/tmp
	TMPDIR=$(CURDIR)/target/tmp cargo mutants --workspace --jobs 4 --output target/mutants $(MUTANTS_ARGS)
	scripts/mutantes.sh --conferir

## portoes     tudo acima + hash de tests/aceite + catraca; grava PORTOES.md
portoes:
	scripts/portoes.sh

## congelar    FATIA=<nome>: leva .aceite-rascunho/<FATIA>/ para tests/aceite/<FATIA>/ e grava o hash
congelar:
	scripts/congelar.sh $(FATIA)

## baseline    regrava as catracas (contagem e mutantes) com os números atuais
baseline:
	scripts/catraca.sh --gravar
	scripts/mutantes.sh --gravar
