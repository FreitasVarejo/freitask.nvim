-- Luacheck do freitask.nvim.

std = "lua54"
max_line_length = 120

globals = {
  "vim",
  "Snacks",
}

-- Os specs rodam sob tests/runner.lua, que injeta o vocabulário de teste como
-- global (é o que dá aos arquivos a leitura de describe/it sem boilerplate de
-- require em cada bloco).
files["tests/*_spec.lua"] = {
  globals = {
    "describe",
    "it",
    "eq",
    "truthy",
    "falsy",
  },
}
