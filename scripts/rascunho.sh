#!/usr/bin/env bash
# Compila e roda um rascunho de testes de aceitação ANTES de congelá-lo, fora
# do workspace: copia .aceite-rascunho/<fatia>/ para target/rascunho/<fatia>/
# com um Cargo.toml equivalente ao do pacote `aceite` e roda `cargo test` ali.
# É como o subagente que escreve os testes confere que eles compilam (e que
# falham contra o esqueleto) sem tocar em tests/aceite/.
#
# Uso: scripts/rascunho.sh <fatia> [args do cargo test]

set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/.."
fatia="${1:?uso: rascunho.sh <fatia> [args]}"
shift || true
origem=".aceite-rascunho/$fatia"
[[ -f "$origem/main.rs" ]] || { echo "rascunho: não há $origem/main.rs" >&2; exit 1; }

raiz="$(pwd)"
# O rascunho é formatado no lugar: depois de congelado ninguém pode tocá-lo, e
# `make check` roda `cargo fmt --check` sobre tests/aceite também.
rustfmt --edition 2024 "$origem"/*.rs
dest="target/rascunho/$fatia"
rm -rf "$dest"
mkdir -p "$dest/testes"
cp -R "$origem"/. "$dest/testes/"
# Arquivos da raiz (schema/, divergencias.toml) ficam acessíveis em ../raiz.
if [[ -d .aceite-rascunho/raiz ]]; then
  mkdir -p "$dest/raiz"
  cp -R .aceite-rascunho/raiz/. "$dest/raiz/"
fi

cat >"$dest/Cargo.toml" <<EOF
[package]
name = "rascunho-$fatia"
version = "0.0.0"
edition = "2024"
publish = false
autotests = false
autobins = false

[lib]
path = "lib.rs"

[[test]]
name = "$fatia"
path = "testes/main.rs"

[dev-dependencies]
assert_cmd = "2"
predicates = "3"
tempfile = "3"
serde_json = "1"
jsonschema = "0.60"
sha2 = "0.10"
toml = "1"
gerador = { path = "$raiz/tests/gerador" }

[workspace]
EOF
printf '//! rascunho\n' >"$dest/lib.rs"

# O binário sob teste é o do workspace; o rascunho não o compila.
cargo build -q -p freitask
export CI=true INSTA_UPDATE=no INSTA_FORCE_PASS=0
export FREITASK_REPO_RAIZ="$raiz"
# Os arquivos da raiz vêm do rascunho quando existe um, senão dos já congelados.
if [[ -d "$dest/raiz" ]]; then
  export FREITASK_ACEITE_RAIZ="$raiz/$dest/raiz"
else
  export FREITASK_ACEITE_RAIZ="$raiz/tests/aceite"
fi
cd "$dest"
cargo test --target-dir "$raiz/target/rascunho/target" "$@"
