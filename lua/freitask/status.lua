-- freitask.status — o vocabulário de status, lido de .freitask/status.json.
--
-- Guarda o único estado global além do cache: `M.status`, o mapa
-- número → metadados. O status de uma task NÃO mora aqui — ele é derivado do
-- tipo do callout, e este módulo só sabe traduzir um no outro.
--
-- Ver docs/freitask.md, seção "Status".

local C = require("freitask.config")

local M = {}

---@type table<string, freitask.StatusMeta>|nil
M.status = nil

---Metadados do status `num`, com fallback para o sentinela de inválido.
---@param num integer|nil
---@return freitask.StatusMeta
function M.status_meta(num)
  if not num or num == 0 then
    return C.STATUS_INVALID
  end
  return (M.status and M.status[tostring(num)]) or C.STATUS_INVALID
end

---Garante que projects/ e o status.json existem.
function M.ensure_root()
  if vim.fn.isdirectory(C.projects) == 0 then
    vim.fn.mkdir(C.projects, "p")
  end
  local dir = vim.fn.fnamemodify(C.status_file, ":h")
  if vim.fn.isdirectory(dir) == 0 then
    vim.fn.mkdir(dir, "p")
  end
  if vim.fn.filereadable(C.status_file) == 0 then
    vim.fn.writefile(vim.split(C.DEFAULT_STATUS_JSON, "\n"), C.status_file)
  end
end

---Carrega metadados de status de .freitask/status.json (cai no default). É uma
---releitura FORÇADA de propósito: os pontos de entrada a chamam para que uma
---edição do status.json valha na hora, sem reiniciar o Neovim.
function M.load_status()
  local sj = C.status_file
  if vim.fn.filereadable(sj) == 1 then
    local ok, decoded = pcall(function()
      return vim.json.decode(table.concat(vim.fn.readfile(sj), "\n"))
    end)
    if ok and type(decoded) == "table" then
      M.status = decoded
      return
    end
    vim.notify("freitask: não consegui parsear status.json, usando defaults", vim.log.levels.WARN)
  end
  M.status = vim.json.decode(C.DEFAULT_STATUS_JSON)
end

---Garante que há status carregado, sem forçar releitura. Substitui o
---`if not M.status then M.load_status() end` que estava repetido em toda
---função que precisava do mapa — cada repetição era uma chance de esquecer.
function M.ensure()
  if not M.status then
    M.load_status()
  end
end

---Mapa tipo-de-callout → número de status (ex.: todo→1), a partir do status.json.
---Memoizado pela IDENTIDADE da tabela de status, não por um flag: assim tanto
---um M.load_status() quanto alguém trocando M.status na mão invalidam sozinhos.
---Antes o mapa era reconstruído a cada parse_block — isto é, 27 entradas por
---task, a cada build_cache do vault inteiro.
---@return table<string, integer>
local rev_src, rev_map = nil, {}
function M.by_callout()
  if rev_src ~= M.status then
    local rev = {}
    for k, v in pairs(M.status or {}) do
      if v.callout then
        rev[v.callout] = tonumber(k)
      end
    end
    rev_src, rev_map = M.status, rev
  end
  return rev_map
end

return M
