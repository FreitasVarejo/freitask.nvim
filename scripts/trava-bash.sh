#!/usr/bin/env bash
# Trava de PreToolUse (Bash) do Claude Code — spec da reescrita, §4, camada 3.
#
# Nega o comando quando ele toca os testes congelados ou os snapshots por fora
# das ferramentas que as outras camadas vigiam: qualquer menção a tests/aceite,
# `insta accept`/`insta review`, as variáveis que fazem o insta aceitar por
# conta própria, `--accept`, `sed -i` sobre tests/, e `git checkout`/`git
# restore` sobre tests/ (desfazer um teste que falhou é a mesma coisa que
# apagá-lo).
#
# Limite conhecido: o hook só enxerga a linha de comando. Um script ou
# subprocesso que faça o mesmo passa; por isso existem o hash
# (tests/aceite/ACEITE.sha256) e o auditor em contexto limpo.
#
# Entrada: o JSON do hook em stdin; a decisão sai pelo código: 0 deixa passar,
# 2 bloqueia e manda a razão para o modelo pelo stderr.

set -euo pipefail

cmd=$(jq -r '.tool_input.command // empty' 2>/dev/null || true)
[[ -z "$cmd" ]] && exit 0

padroes=(
  'tests/aceite'
  'insta[[:space:]]+accept'
  'insta[[:space:]]+review'
  'INSTA_UPDATE='
  'INSTA_FORCE_PASS'
  '--accept'
  'sed[[:space:]]+-[a-zA-Z]*i.*tests/'
  'git[[:space:]]+checkout[[:space:]]+--[[:space:]]+.*tests/'
  'git[[:space:]]+restore[[:space:]]+.*tests/'
)

for p in "${padroes[@]}"; do
  if [[ "$cmd" =~ $p ]]; then
    echo "trava (spec §4): comando negado por casar \`$p\`. Testes congelados e snapshots não se editam por aqui; use make congelar (subagente) ou registre a decisão em docs/decisoes-da-reescrita.md." >&2
    exit 2
  fi
done
exit 0
