#!/bin/sh
# Um `freitask` de mentira para os testes do engine.rs: anota cada chamada, uma
# por linha, em $FAKE_FREITASK_LOG e responde o mínimo que a TUI lê.
# FAKE_FREITASK_REBUILD=falha faz o `rebuild` recusar, como o motor faria.
echo "$*" >>"$FAKE_FREITASK_LOG"
case "$1" in
rebuild)
  if [ "$FAKE_FREITASK_REBUILD" = falha ]; then
    echo "freitask: CURRENT.md tem alterações não salvas" >&2
    exit 1
  fi
  echo /v/CURRENT.md
  ;;
statuses | projects | list)
  echo '[]'
  ;;
esac
