-- freitask.cache — o índice em memória das tasks ATIVAS.
--
-- Guarda também o bloco de cada task, para que o regen do CURRENT.md não
-- precise reabrir arquivo nenhum. Tasks arquivadas nunca entram aqui de
-- propósito: assim o board, o regen e o resto do módulo seguem enxergando só
-- as ativas, sem um filtro novo em cada leitor. Quem quiser as arquivadas
-- chama M.archived_entries_for, que lê do disco sob demanda.
--
-- `path_` e não `path`: o nome `path` é usado como variável em quase toda
-- função deste arquivo, e sombrear o módulo daria um erro mudo.

local C = require("freitask.config")
local fs = require("freitask.fs")
local md = require("freitask.md")
local model = require("freitask.model")
local path_ = require("freitask.path")
local status = require("freitask.status")

local M = {}

---@type freitask.Entry[]|nil
M.cache = nil

---Lê o arquivo de uma task UMA vez e extrai tudo que o cache precisa.
---Substitui o par is_archived + read_callout, que abria o mesmo arquivo duas
---vezes por task — e o rebuild_current abria uma terceira para reimprimir o
---bloco. Guardando o bloco na entry, o regen não toca mais no disco.
---Não checa mais arquivamento: isso agora é o CAMINHO, resolvido antes de
---chegar aqui por split_task_path.
---@param path string
---@return { block: string[], status_num: integer }|nil
function M.scan_task(path)
  if vim.fn.filereadable(path) == 0 then
    return nil
  end
  local _, _, block = md.first_block(fs.read_lines(path))
  block = block or {}

  status.ensure()
  -- Arquivo de task sem callout é um arquivo quebrado: 0 o deixa no topo do
  -- projeto em vez de escondê-lo no meio do Backlog.
  local status_num = (#block > 0) and model.parse_block(block).status_num or 0
  return { block = block, status_num = status_num }
end

---Insere ou atualiza uma task no cache em memória. Tasks arquivadas nunca
---entram no cache — logo, nunca aparecem no CURRENT.md.
---@param path string
function M.update_cache_entry(path)
  local project, task_id, archived = path_.split_task_path(path)
  if not project or archived then
    return
  end
  M.cache = M.cache or {}
  local scan = M.scan_task(path)
  if not scan then
    M.remove_cache_entry(path)
    return
  end
  for _, e in ipairs(M.cache) do
    if e.path == path then
      e.status_num = scan.status_num
      e.block = scan.block
      return
    end
  end
  M.cache[#M.cache + 1] = {
    project = project,
    task_id = task_id,
    status_num = scan.status_num,
    block = scan.block,
    path = path,
  }
end

---Remove uma task do cache (após deleção).
---@param path string
function M.remove_cache_entry(path)
  if not M.cache then
    return
  end
  for i, e in ipairs(M.cache) do
    if e.path == path then
      table.remove(M.cache, i)
      return
    end
  end
end

---Rescan completo de tasks/*/*.md para o cache.
function M.build_cache()
  M.cache = {}
  for _, path in ipairs(vim.fn.glob(C.root .. "/*/*.md", true, true)) do
    M.update_cache_entry(path)
  end
end

---Tasks cacheadas de um projeto, ordenadas por status e depois id.
---@param project string
---@return freitask.Entry[]
function M.entries_for(project)
  local list = {}
  for _, e in ipairs(M.cache or {}) do
    if e.project == project then
      list[#list + 1] = e
    end
  end
  table.sort(list, function(a, b)
    if a.status_num == b.status_num then
      return a.task_id < b.task_id
    end
    return a.status_num < b.status_num
  end)
  return list
end

---Lista de ids de projeto (subdiretórios da raiz, exceto reservados).
---@return string[]
function M.list_projects()
  local out = {}
  for _, path in ipairs(vim.fn.glob(C.root .. "/*", true, true)) do
    if vim.fn.isdirectory(path) == 1 then
      local name = vim.fn.fnamemodify(path, ":t")
      if not C.RESERVED[name] then
        out[#out + 1] = name
      end
    end
  end
  table.sort(out)
  return out
end

---Lista as tasks arquivadas de um projeto, lidas do disco sob demanda. NÃO
---entram em M.cache de propósito: assim o board, o regen e o resto do módulo
---seguem enxergando só as tasks ativas, sem um filtro novo em cada leitor.
---@param project string
---@return freitask.Entry[]
function M.archived_entries_for(project)
  local out = {}
  for _, path in ipairs(vim.fn.glob(C.root .. "/" .. project .. "/archived/*/*.md", true, true)) do
    local p, id, tipo = path_.split_task_path(path)
    if p then
      local scan = M.scan_task(path)
      out[#out + 1] = {
        project = p,
        task_id = id,
        archived = tipo,
        status_num = scan and scan.status_num or 0,
        block = scan and scan.block or {},
        path = path,
      }
    end
  end
  table.sort(out, function(a, b)
    if a.archived == b.archived then
      return a.task_id < b.task_id
    end
    return a.archived < b.archived
  end)
  return out
end

--      `bar`; a informação some junto com o link);
--   2. as entradas de histórico (são fatos sobre o passado).
-- Note que ARQUIVAR não está nessa lista: o move preserva o basename, o
-- Obsidian resolve `[[foo]]` por basename e o link da linha 2 é derivável — um
-- agente que só faz `mv` para archived/ causa dano inteiramente reparável.

---Todos os arquivos de task do vault (ativos + arquivados), já decompostos.
---@return freitask.TaskRef[]
function M.all_tasks()
  local out = {}
  local globs = { C.root .. "/*/*.md", C.root .. "/*/archived/*/*.md" }
  for _, g in ipairs(globs) do
    for _, path in ipairs(vim.fn.glob(g, true, true)) do
      local project, id, archived = path_.split_task_path(path)
      if project then
        out[#out + 1] = { path = path, project = project, id = id, archived = archived }
      end
    end
  end
  return out
end

return M
