#!/usr/bin/env bash
# `make portoes` (spec §3 e §4): roda cada alvo, confere o hash dos testes
# congelados e a catraca, e grava PORTOES.md — o relatório de dois minutos que
# o dono lê. Cada portão sai verde ou vermelho; o script sai 1 se algum caiu.
#
# Variáveis: PORTOES_SEM_MUTANTES=1 pula o cargo-mutants (minutos); o relatório
# diz que pulou. PORTOES_SEM_DIFF=1 pula o diferencial (sem nvim).

set -uo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/.."
export CI=true INSTA_UPDATE=no INSTA_FORCE_PASS=0

out=PORTOES.md
log=target/portoes
mkdir -p "$log"
falhas=0
linhas=()

rodar() {
  local nome="$1"; shift
  local arquivo="$log/$nome.log"
  local inicio=$SECONDS
  if "$@" >"$arquivo" 2>&1; then
    linhas+=("| $nome | verde | $((SECONDS - inicio))s | \`$*\` |")
    echo "portão $nome: verde"
  else
    linhas+=("| $nome | **vermelho** | $((SECONDS - inicio))s | \`$*\` (ver $arquivo) |")
    echo "portão $nome: VERMELHO (ver $arquivo)"
    falhas=1
  fi
}

pular() {
  linhas+=("| $1 | pulado | - | $2 |")
  echo "portão $1: pulado ($2)"
}

rodar check make check
rodar test make test
rodar hash-aceite scripts/hash-aceite.sh --conferir
# O runner não acusa seleção vazia: só vale como portão se há alvo [[test]].
if grep -q '^\[\[test\]\]' tests/aceite/Cargo.toml 2>/dev/null; then
  rodar aceite make aceite
else
  pular aceite "tests/aceite ainda sem testes congelados"
fi
if [[ "${PORTOES_SEM_DIFF:-}" == 1 ]]; then
  pular diff "PORTOES_SEM_DIFF=1"
elif command -v nvim >/dev/null; then
  rodar diff make diff
else
  pular diff "sem nvim no PATH"
fi
if [[ "${PORTOES_SEM_MUTANTES:-}" == 1 ]]; then
  pular mutantes "PORTOES_SEM_MUTANTES=1"
else
  rodar mutantes make mutantes
fi
rodar catraca scripts/catraca.sh

# A PRIMEIRA fatia congelada (spec §4, camada 2), em ordem de versão.
tag_aceite=$(git tag -l 'aceite/*' | sort -V | head -1)
if [[ -n "$tag_aceite" ]]; then
  alterados=$(git diff --stat "$tag_aceite"..HEAD -- tests/aceite | tail -1)
  [[ -z "$alterados" ]] && alterados="0 arquivos alterados"
else
  alterados="sem tag aceite/* ainda"
fi

{
  echo "# PORTOES.md — gerado por \`make portoes\` em $(date -Is) no commit $(git rev-parse --short HEAD)"
  echo
  echo "| portão | estado | tempo | comando |"
  echo "| --- | --- | --- | --- |"
  printf '%s\n' "${linhas[@]}"
  echo
  echo "## Aceite congelado"
  echo
  echo "- hash: $(scripts/hash-aceite.sh 2>/dev/null)"
  echo "- \`git diff --stat ${tag_aceite:-<tag>}..HEAD -- tests/aceite\`: $alterados"
  echo
  echo "## Catraca"
  echo
  echo '```'
  grep -E '^catraca:' "$log/catraca.log" || true
  echo '```'
  if [[ -f "$log/mutantes.log" ]]; then
    echo
    echo "## Mutação"
    echo
    echo '```'
    scripts/mutantes.sh --placar 2>/dev/null || grep -E '^mutantes' "$log/mutantes.log" || true
    echo '```'
  fi
  if [[ -f "$log/diff.log" ]]; then
    echo
    echo "## Diferencial"
    echo
    echo '```'
    grep -E '^(diferencial|cenários|divergências|entradas)' "$log/diff.log" || tail -20 "$log/diff.log"
    echo '```'
    echo
    echo "Relatório completo: \`target/diferencial/relatorio.md\`."
  fi
  if [[ -f "$log/test.log" ]]; then
    echo
    echo "## Testes"
    echo
    echo '```'
    grep -E '^test result' "$log/test.log" || true
    echo '```'
  fi
} >"$out"

echo "portoes: relatório em $out"
exit $falhas
