-- util.freitask — FACHADA. O freitask mora em `lua/freitask/`.
--
-- Este arquivo existe só para que `require("util.freitask")` siga funcionando:
-- a CLI (`vault/.local/bin/freitask` → util/freitask_cli.lua), o spec do
-- plugin e qualquer keymap antigo apontam para cá. Um require é barato; um
-- caminho quebrado num script que você chama de dentro do Obsidian, não.
--
-- Código novo deve requerer `freitask` (ou o submódulo específico:
-- `freitask.model`, `freitask.task`, …) direto. Ver docs/freitask-internals.md.

return require("freitask")
