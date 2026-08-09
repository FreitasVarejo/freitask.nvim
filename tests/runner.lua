-- tests/runner.lua — harness mínimo de testes, rodado sob `nvim --clean -l`.
--
-- Não usa busted nem plenary de propósito: os dois exigiriam uma dependência
-- externa instalada para rodar o teste de um módulo que só toca arquivos e
-- vim.fn. O custo aqui é ~80 linhas; o custo lá é um passo de setup que
-- ninguém lembra de fazer antes de mexer no parser.
--
-- Uso: nvim/tests/run.sh [padrão]   (padrão filtra por nome de arquivo)
-- Saída: uma linha por falha + resumo. Código de saída 1 se algo falhou.

local M = { passed = 0, failed = 0, failures = {} }

local stack = {}

---Escreve em stdout sem passar pelo canal de mensagens do Neovim (o `\n` da
---última linha se perde quando os.exit encerra o processo — mesmo motivo do
---`say` da CLI).
local function say(s)
  io.stdout:write(tostring(s) .. "\n")
end

---@param name string
---@param fn fun()
function _G.describe(name, fn)
  stack[#stack + 1] = name
  local ok, err = pcall(fn)
  stack[#stack] = nil
  if not ok then
    M.failed = M.failed + 1
    M.failures[#M.failures + 1] = { label = name .. " (erro fora de it)", err = err }
  end
end

---@param name string
---@param fn fun()
function _G.it(name, fn)
  local label = table.concat(stack, " › ") .. " › " .. name
  local ok, err = pcall(fn)
  if ok then
    M.passed = M.passed + 1
  else
    M.failed = M.failed + 1
    M.failures[#M.failures + 1] = { label = label, err = err }
  end
end

---Igualdade profunda. Compara com vim.deep_equal para que tabelas (modelos,
---arrays de linhas) sejam comparáveis por valor.
function _G.eq(expected, actual, msg)
  if not vim.deep_equal(expected, actual) then
    error(
      string.format(
        "%s\n    esperado: %s\n    obtido:   %s",
        msg or "valores diferentes",
        vim.inspect(expected),
        vim.inspect(actual)
      ),
      2
    )
  end
end

function _G.truthy(v, msg)
  if not v then
    error(msg or "esperava valor verdadeiro, obtive " .. vim.inspect(v), 2)
  end
end

function _G.falsy(v, msg)
  if v then
    error(msg or "esperava valor falso/nil, obtive " .. vim.inspect(v), 2)
  end
end

--- Execução -------------------------------------------------------------------

local here = debug.getinfo(1, "S").source:sub(2):match("(.*)/")
local filter = (_G.arg or {})[1]

local specs = vim.fn.glob(here .. "/*_spec.lua", true, true)
table.sort(specs)

for _, spec in ipairs(specs) do
  if not filter or spec:find(filter, 1, true) then
    local chunk, load_err = loadfile(spec)
    if not chunk then
      M.failed = M.failed + 1
      M.failures[#M.failures + 1] = { label = spec, err = load_err }
    else
      local ok, err = pcall(chunk)
      if not ok then
        M.failed = M.failed + 1
        M.failures[#M.failures + 1] = { label = spec .. " (erro ao carregar)", err = err }
      end
    end
  end
end

for _, f in ipairs(M.failures) do
  say("FALHOU  " .. f.label)
  say("        " .. tostring(f.err):gsub("\n", "\n        "))
  say("")
end

say(string.format("%d passou, %d falhou", M.passed, M.failed))
io.stdout:flush()
os.exit(M.failed > 0 and 1 or 0)
