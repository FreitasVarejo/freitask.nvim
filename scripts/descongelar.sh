#!/usr/bin/env bash
# Desfaz um `make congelar` que ainda NÃO foi selado pela tag `aceite/<fatia>`:
# devolve tests/aceite/<fatia>/ para .aceite-rascunho/<fatia>/, regenera o
# Cargo.toml do pacote `aceite` e o hash. Com a tag presente, recusa: o que a
# tag sela só muda pelo caminho da reavaliação (docs/decisoes-da-reescrita.md).
#
# Existe para o caso medido na fatia 0: o rascunho congelado não passava no
# `cargo fmt --check`, e formatar um teste congelado à mão é o que as travas
# proíbem. O rascunho volta, `scripts/rascunho.sh` o formata, e congela-se de
# novo.
#
# Uso: scripts/descongelar.sh <fatia>

set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/.."
fatia="${1:?uso: descongelar.sh <fatia>}"
destino="tests/aceite/$fatia"
origem=".aceite-rascunho/$fatia"

[[ -d "$destino" ]] || { echo "descongelar: não há $destino" >&2; exit 1; }
if git tag -l "aceite/$fatia" | grep -q .; then
  echo "descongelar: a tag aceite/$fatia existe; teste selado não se descongela" >&2
  exit 1
fi
[[ -e "$origem" ]] && { echo "descongelar: $origem já existe" >&2; exit 1; }

mkdir -p .aceite-rascunho
mv "$destino" "$origem"
scripts/gerar-cargo-aceite.sh
scripts/hash-aceite.sh --gravar
echo "descongelado: $origem (rode scripts/rascunho.sh $fatia e depois make congelar FATIA=$fatia)"
