#!/usr/bin/env bash
# Congela os testes de aceitação de uma fatia (spec §4, camadas 1 e 2).
#
# O subagente que escreve os testes não vê a implementação e, pelo
# .claude/settings.json, também não escreve em tests/aceite/: ele grava em
# .aceite-rascunho/<fatia>/. Este script é o passo mecânico que leva o rascunho
# para tests/aceite/<fatia>/, regenera o Cargo.toml do pacote `aceite` a partir
# das pastas presentes (ninguém edita esse Cargo.toml à mão) e regrava o hash.
# O implementador então confirma que `make aceite` está vermelho e faz
# `git tag aceite/<fatia>`.
#
# Só ACRESCENTA: se tests/aceite/<fatia>/ já existe, recusa. Reavaliar um
# teste é outro caminho (docs/decisoes-da-reescrita.md).
#
# Uso: scripts/congelar.sh <fatia>     (ex.: f1-core)

set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/.."

fatia="${1:?uso: congelar.sh <fatia>}"
origem=".aceite-rascunho/$fatia"
destino="tests/aceite/$fatia"

[[ -d "$origem" ]] || { echo "congelar: não há $origem" >&2; exit 1; }
[[ -e "$destino" ]] && { echo "congelar: $destino já existe; teste congelado não se regrava" >&2; exit 1; }
[[ -f "$origem/main.rs" ]] || { echo "congelar: $origem/main.rs é obrigatório (alvo de teste)" >&2; exit 1; }

mkdir -p tests/aceite
cp -R "$origem" "$destino"

# Arquivos soltos que o rascunho traga para a raiz de tests/aceite (schema/,
# divergencias.toml, dores.rs): só se ainda não existirem.
if [[ -d ".aceite-rascunho/raiz" ]]; then
  (cd .aceite-rascunho/raiz && find . -type f -print0) | while IFS= read -r -d '' f; do
    if [[ -e "tests/aceite/$f" ]]; then
      echo "congelar: tests/aceite/$f já existe; ignorando a cópia do rascunho" >&2
    else
      mkdir -p "tests/aceite/$(dirname "$f")"
      cp ".aceite-rascunho/raiz/$f" "tests/aceite/$f"
    fi
  done
fi

scripts/gerar-cargo-aceite.sh
scripts/hash-aceite.sh --gravar
rm -rf "$origem" .aceite-rascunho/raiz
rmdir .aceite-rascunho 2>/dev/null || true

echo "congelado: $destino"
echo "agora: make aceite (deve ficar VERMELHO), depois git tag aceite/$fatia"
