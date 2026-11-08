#!/usr/bin/env bash
# Hash dos testes de aceitação congelados (spec §4, camada 2).
#
# Cobre todo arquivo sob tests/aceite/, exceto:
# - ACEITE.sha256, que é o próprio hash;
# - mutantes.baseline e contagem.baseline, as catracas que `make baseline`
#   regrava a cada fatia fechada (só crescem; scripts/catraca.sh vigia).
#
# Uso: scripts/hash-aceite.sh            imprime o hash
#      scripts/hash-aceite.sh --conferir compara com ACEITE.sha256 e sai 1 se divergir
#      scripts/hash-aceite.sh --gravar   regrava ACEITE.sha256 (só o `make congelar` chama)

set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/.."
dir=tests/aceite
arquivo="$dir/ACEITE.sha256"

calcular() {
  (
    cd "$dir"
    find . -type f \
      ! -name ACEITE.sha256 ! -name mutantes.baseline ! -name contagem.baseline \
      ! -path './target/*' -print0 \
      | LC_ALL=C sort -z \
      | xargs -0 sha256sum
  ) | sha256sum | cut -d' ' -f1
}

case "${1:-}" in
  --conferir)
    atual=$(calcular)
    gravado=$(cut -d' ' -f1 <"$arquivo" 2>/dev/null || true)
    if [[ "$atual" != "$gravado" ]]; then
      echo "aceite: hash divergente (gravado ${gravado:-<vazio>}, atual $atual)" >&2
      exit 1
    fi
    echo "aceite: hash confere ($atual)"
    ;;
  --gravar)
    atual=$(calcular)
    echo "$atual  tests/aceite" >"$arquivo"
    echo "aceite: hash gravado ($atual)"
    ;;
  *)
    calcular
    ;;
esac
