#!/usr/bin/env bash
# Cria os snapshots do insta que AINDA NÃO EXISTEM (spec §3.5: "os `.snap` são
# criados uma vez pelo implementador e ficam sob a trava de §4"). Decisão do
# dono na fatia 7 (docs/decisoes-da-reescrita.md, seção "Fatia 7").
#
# Nunca sobrescreve: se um teste gera um `.snap.new` ao lado de um `.snap` que
# já existe, o snapshot mudou; o script apaga o `.snap.new`, lista o caso e sai
# 1. Snapshot que mudou continua falhando em `make test` e só muda por
# reavaliação (spec §4, camada 1), nunca por aqui.
#
# Como: roda `cargo test` sem `CI` e sem as variáveis do insta que o Makefile
# exporta, para o insta gravar `.snap.new` de cada snapshot que falta, e então
# renomeia cada `.snap.new` sem `.snap` correspondente. Depois, `make test`.
#
# Uso: scripts/snapshots-novos.sh [args do cargo test, ex.: -p freitask tui]

set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/.."

# Um `.snap.new` que sobrou de antes confundiria a conta: o script começa limpo.
sobras=$(find crates -name '*.snap.new' -not -path '*/target/*')
if [[ -n "$sobras" ]]; then
  echo "snapshots-novos: há .snap.new de antes; apague-os e rode de novo:" >&2
  echo "$sobras" >&2
  exit 1
fi

[[ $# -gt 0 ]] || set -- -p freitask
mkdir -p target
criados=0
recusados=0
# Um teste com dois snapshots para no primeiro que falta (o insta entra em
# pânico), e o segundo só nasce na rodada seguinte: repete até uma rodada não
# criar nada.
for _ in 1 2 3 4 5 6 7 8; do
  # A falha é esperada (snapshot que falta faz o teste falhar); o que conta é
  # o que o insta deixou em disco, conferido abaixo.
  env -u CI -u INSTA_UPDATE -u INSTA_FORCE_PASS cargo test "$@" >target/snapshots-novos.log 2>&1 || true
  nesta=0
  while IFS= read -r -d '' novo; do
    snap="${novo%.new}"
    if [[ -e "$snap" ]]; then
      echo "snapshots-novos: RECUSADO, já existe $snap (o snapshot mudou; reavaliação, §4)" >&2
      rm -f "$novo"
      recusados=$((recusados + 1))
    else
      mv "$novo" "$snap"
      echo "criado: $snap"
      criados=$((criados + 1))
      nesta=$((nesta + 1))
    fi
  done < <(find crates -name '*.snap.new' -not -path '*/target/*' -print0)
  [[ $recusados -eq 0 && $nesta -gt 0 ]] || break
done

echo "snapshots-novos: $criados criado(s), $recusados recusado(s); log em target/snapshots-novos.log"
[[ $recusados -eq 0 ]] || exit 1
echo "agora: leia cada .snap criado e rode make test"
