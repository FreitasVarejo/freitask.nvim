#!/usr/bin/env bash
# Placar de mutação (spec §3.6): lê target/mutants/ (saída do cargo-mutants),
# exige zero `missed` em crates/core e, no resto, que o número de `missed` não
# suba em relação a tests/aceite/mutantes.baseline. Cada sobrevivente novo ou
# ganha um teste em tests/staging/ ou uma linha de justificativa no baseline
# (prefixo `justificado:`), que o auditor lê.
#
# Uso: scripts/mutantes.sh --conferir | --gravar | --placar

set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/.."

dir=target/mutants/mutants.out
baseline=tests/aceite/mutantes.baseline
missed="$dir/missed.txt"

[[ -f "$missed" ]] || { echo "mutantes: não há $missed; rode make mutantes" >&2; exit 1; }

total_missed=$(grep -c . "$missed" || true)
core_missed=$(grep -c '^crates/core/' "$missed" || true)
resto_missed=$(grep -vc '^crates/core/' "$missed" || true)
caught=$(grep -c . "$dir/caught.txt" 2>/dev/null || true)
timeout=$(grep -c . "$dir/timeout.txt" 2>/dev/null || true)
unviable=$(grep -c . "$dir/unviable.txt" 2>/dev/null || true)

placar() {
  echo "mutantes: caught=$caught missed=$total_missed (core=$core_missed, resto=$resto_missed) timeout=$timeout unviable=$unviable"
  if (( total_missed > 0 )); then
    echo "sobreviventes:"
    sed 's/^/  /' "$missed"
  fi
}

case "${1:-}" in
  --placar) placar ;;
  --conferir)
    placar
    falhas=0
    if (( core_missed > 0 )); then
      echo "mutantes: $core_missed sobrevivente(s) em crates/core; o limiar é zero" >&2; falhas=1
    fi
    if [[ -f "$baseline" ]]; then
      base=$(awk '/^resto_missed/ {print $2}' "$baseline")
      if (( resto_missed > base )); then
        echo "mutantes: missed fora do core subiu de $base para $resto_missed" >&2; falhas=1
      fi
      # Sobrevivente que não está no baseline (nem como justificado) é novo.
      while IFS= read -r linha; do
        [[ -z "$linha" ]] && continue
        if ! grep -qF -- "$linha" "$baseline"; then
          echo "mutantes: sobrevivente novo sem registro no baseline: $linha" >&2; falhas=1
        fi
      done < <(grep -v '^crates/core/' "$missed")
    fi
    exit $falhas
    ;;
  --gravar)
    {
      echo "# Catraca de mutação (spec §3.6). Gravado por scripts/mutantes.sh --gravar."
      echo "# Cada sobrevivente fora do core fica listado; linhas 'justificado: <mutante> -- <porquê>'"
      echo "# explicam os que não valem um teste. O auditor lê as justificativas."
      echo "resto_missed $resto_missed"
      echo "core_missed $core_missed"
      grep -v '^crates/core/' "$missed" || true
    } >"$baseline"
    echo "mutantes: baseline gravado ($resto_missed fora do core)"
    ;;
  *) echo "uso: mutantes.sh --conferir|--gravar|--placar" >&2; exit 2 ;;
esac
