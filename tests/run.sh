#!/usr/bin/env bash
# tests/run.sh — roda a suíte de testes do config do Neovim.
#
# `--clean`: a suíte não deve depender de plugin nenhum nem do estado do
# usuário, pelo mesmo motivo que a CLI do freitask não depende (ver
# vault/.local/bin/freitask). Os módulos testados só tocam arquivos e vim.fn.
#
# Uso: nvim/tests/run.sh [padrão]   — o padrão filtra por nome de arquivo.

set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
nvim_dir="$(dirname "$here")"

if ! command -v nvim &>/dev/null; then
  echo "tests: nvim não encontrado no PATH" >&2
  exit 2
fi

exec nvim --clean --cmd "set runtimepath+=$nvim_dir" -l "$here/runner.lua" "$@"
