#!/usr/bin/env bash
# Catraca de números (spec §4, camada 4) e varredura de special-casing barato.
#
# Confere que a contagem de testes e a de asserts não diminuíram em relação a
# tests/aceite/contagem.baseline, que não há `#[ignore]`/`#[should_panic]` sem
# justificativa no log de decisões, nenhum `assert!(true)`, nenhum `cfg!(test)`
# em src/ e nenhuma string com nome de teste/fixture em src/.
#
# Uso: scripts/catraca.sh            confere (sai 1 se algo regrediu)
#      scripts/catraca.sh --gravar   confere e regrava o baseline com os números atuais

set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/.."
export CI=true INSTA_UPDATE=no INSTA_FORCE_PASS=0

baseline=tests/aceite/contagem.baseline
falhas=0

# 1. Contagem de testes: tudo que `cargo test -- --list` enumera no workspace.
testes=$(cargo test --workspace --quiet -- --list 2>/dev/null | grep -c ': test$' || true)
# 2. Contagem de asserts em código de teste (crates/**, tests/**), por macro.
asserts=$(grep -rhoE '\bassert(_eq|_ne|_matches)?!|\bprop_assert(_eq|_ne)?!|\bassert_snapshot!|\bassert_debug_snapshot!|\.assert\(' crates tests --include='*.rs' | wc -l)

echo "catraca: testes=$testes asserts=$asserts"

if [[ -f "$baseline" ]]; then
  base_testes=$(awk '/^testes/ {print $2}' "$baseline")
  base_asserts=$(awk '/^asserts/ {print $2}' "$baseline")
  if (( testes < base_testes )); then
    echo "catraca: testes caíram de $base_testes para $testes" >&2; falhas=1
  fi
  if (( asserts < base_asserts )); then
    echo "catraca: asserts caíram de $base_asserts para $asserts" >&2; falhas=1
  fi
  echo "catraca: delta testes=$((testes - base_testes)) asserts=$((asserts - base_asserts))"
fi

# 3. #[ignore] e #[should_panic] novos exigem justificativa no log de decisões.
ignorados=$(grep -rnE '#\[(ignore|should_panic)' crates tests --include='*.rs' || true)
if [[ -n "$ignorados" ]]; then
  while IFS= read -r linha; do
    arquivo=${linha%%:*}
    if ! grep -qF "$arquivo" docs/decisoes-da-reescrita.md; then
      echo "catraca: $linha sem justificativa em docs/decisoes-da-reescrita.md" >&2; falhas=1
    fi
  done <<<"$ignorados"
fi

# 4. assert!(true) é um teste que não testa.
if grep -rnE 'assert!\(\s*true\s*[,)]' crates tests --include='*.rs'; then
  echo "catraca: assert!(true) encontrado" >&2; falhas=1
fi

# 5. cfg!(test) em src/ é a implementação sabendo que está sendo testada.
if grep -rn 'cfg!(test)' crates/*/src --include='*.rs'; then
  echo "catraca: cfg!(test) em src/" >&2; falhas=1
fi

# 6. Nome de teste ou de fixture dentro de src/: special-casing.
for termo in aceite rascunho fixture diferencial tempdir temp_dir; do
  # Linhas de comentário ficam de fora; o resto é achado.
  hits=$(grep -rniE "\"[^\"]*${termo}[^\"]*\"" crates/*/src --include='*.rs' | grep -vE '^[^:]+:[0-9]+:\s*//' || true)
  if [[ -n "$hits" ]]; then
    echo "catraca: string suspeita (\"$termo\") em src/:" >&2
    echo "$hits" >&2
    falhas=1
  fi
done

if [[ "${1:-}" == "--gravar" && $falhas -eq 0 ]]; then
  printf 'testes %s\nasserts %s\n' "$testes" "$asserts" >"$baseline"
  echo "catraca: baseline gravado"
fi

exit $falhas
