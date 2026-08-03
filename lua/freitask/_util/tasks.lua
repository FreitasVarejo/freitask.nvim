-- util.tasks — núcleo do task-manager baseado em Obsidian.
--
-- Opera exclusivamente sob ~/ObsidianVault/tasks/. Cada projeto é um
-- subdiretório (id kebab-case) e cada task é um arquivo markdown cujo bloco
-- inicial (`>` blockquote) codifica status/título/id, e da linha 4 em diante
-- linhas arbitrárias (impedimentos, notas — texto livre). O id da task (=
-- nome do arquivo) é também o nome da branch associada; por isso não há linha
-- `Branch:` separada — o wikilink `[[id]]` na sua própria linha representa
-- ambos. Formato atual do bloco:
--   > [!todo] **Título da task**     ← tipo do callout dá ícone/cor + título
--   > [[id-da-task]]                 ← link do obsidian (id == branch)
--   > 1 - Backlog                    ← texto do status
-- O número/nome do status é derivado do TIPO do callout da linha 1 (mapeado
-- via status.json). O parser também aceita os formatos legados (status na
-- linha 1, título na linha 2, e/ou linha `> Branch:`) para migração; ver
-- M.migrate_format.
--
-- Este módulo é `require`-ável (`require("util.tasks")`) para que tanto o
-- plugin do picker (plugins/task-manager.lua) quanto os keymaps buffer-local
-- de edição por cursor possam consumi-lo. Toca apenas arquivos + Snacks +
-- vim.ui, portanto NÃO usa a API do obsidian.nvim (configurado à parte).

local M = {}

local root = vim.fn.expand("~/ObsidianVault/tasks")

-- Diretórios sob tasks/ que NÃO são projetos.
local RESERVED = { daily = true, templates = true }

-- Metadados de status padrão, espelhados em tasks/status.json no primeiro uso e
-- usados como fallback quando o arquivo está ausente ou corrompido.
local DEFAULT_STATUS_JSON = [[{
  "1": { "title": "Backlog", "callout": "todo", "icon": "", "hl_group": "DiagnosticInfo" },
  "2": { "title": "In Progress", "callout": "example", "icon": "", "hl_group": "DiagnosticHint" },
  "3": { "title": "Blocked", "callout": "warning", "icon": "󰂃", "hl_group": "DiagnosticWarn" },
  "4": { "title": "Review", "callout": "question", "icon": "󰋗", "hl_group": "DiagnosticVirtualTextInfo" },
  "5": { "title": "Done", "callout": "success", "icon": "", "hl_group": "DiagnosticOk" }
}]]

-- Dobra de acentos multibyte-safe para ids kebab-case (títulos PT-BR).
local ACCENTS = {
  ["á"] = "a",
  ["à"] = "a",
  ["â"] = "a",
  ["ã"] = "a",
  ["ä"] = "a",
  ["é"] = "e",
  ["ê"] = "e",
  ["è"] = "e",
  ["ë"] = "e",
  ["í"] = "i",
  ["ì"] = "i",
  ["î"] = "i",
  ["ï"] = "i",
  ["ó"] = "o",
  ["ô"] = "o",
  ["õ"] = "o",
  ["ò"] = "o",
  ["ö"] = "o",
  ["ú"] = "u",
  ["ù"] = "u",
  ["û"] = "u",
  ["ü"] = "u",
  ["ç"] = "c",
  ["ñ"] = "n",
}

---Converte uma string arbitrária num identificador kebab-case.
---@param s string
---@return string
local function kebab(s)
  s = s:lower()
  for from, to in pairs(ACCENTS) do
    s = s:gsub(from, to)
  end
  s = s:gsub("[^%w%s%-]", "") -- descarta o que não for word/space/dash
  s = s:gsub("[%s_]+", "-") -- espaços e underscores viram dash
  s = s:gsub("%-+", "-") -- colapsa dashes repetidos
  s = s:gsub("^%-+", ""):gsub("%-+$", "") -- apara dashes nas pontas
  return s
end

--- State ---------------------------------------------------------------------

M.status = nil ---@type table<string, table>|nil
M.cache = nil ---@type table[]|nil

---Garante que a raiz de tasks e o status.json existem.
function M.ensure_root()
  if vim.fn.isdirectory(root) == 0 then
    vim.fn.mkdir(root, "p")
  end
  local sj = root .. "/status.json"
  if vim.fn.filereadable(sj) == 0 then
    vim.fn.writefile(vim.split(DEFAULT_STATUS_JSON, "\n"), sj)
  end
end

---Carrega metadados de status de tasks/status.json (cai no default).
function M.load_status()
  local sj = root .. "/status.json"
  if vim.fn.filereadable(sj) == 1 then
    local ok, decoded = pcall(function()
      return vim.json.decode(table.concat(vim.fn.readfile(sj), "\n"))
    end)
    if ok and type(decoded) == "table" then
      M.status = decoded
      return
    end
    vim.notify("task-manager: não consegui parsear status.json, usando defaults", vim.log.levels.WARN)
  end
  M.status = vim.json.decode(DEFAULT_STATUS_JSON)
end

---Lê só a primeira linha de um arquivo de task e extrai o número do status.
---@param path string
---@return integer|nil
function M.parse_status_num(path)
  local blk = M.read_callout(path)
  if #blk == 0 then
    return nil
  end
  if not M.status then
    M.load_status()
  end
  return M.parse_block(blk).status_num
end

--- Modelo de task (parse/serialize do bloco de callout) ----------------------

---Remove o prefixo de blockquote ("> " / ">") de uma linha.
---@param line string
---@return string
local function strip_quote(line)
  return (line:gsub("^%s*>%s?", ""))
end

---Mapa tipo-de-callout → número de status (ex.: todo→1), a partir do status.json.
---@return table<string, integer>
local function status_by_callout()
  local rev = {}
  if M.status then
    for k, v in pairs(M.status) do
      if v.callout then
        rev[v.callout] = tonumber(k)
      end
    end
  end
  return rev
end

---Verdadeiro se `content` é um "texto de status" (ex.: "1 - Backlog") coerente
---com o status.json — usado p/ não confundir a linha de status com uma nota.
---@param content string
---@return boolean, integer|nil
local function is_status_text(content)
  local snum, stitle = content:match("^(%d+)%s*%-%s*(.+)$")
  if snum and M.status and M.status[snum] and vim.trim(stitle) == (M.status[snum].title or "") then
    return true, tonumber(snum)
  end
  return false, nil
end

---Parseia o bloco `>` de uma task num modelo estruturado. O status vem do TIPO
---do callout na linha 1; título da linha 1 (novo) ou de uma linha `**...**`
---(legado); id do primeiro `[[...]]`. Linhas de status-texto, id-só e `Branch:`
---legada são consumidas; o resto vira `extras` (texto livre).
---@param block string[] linhas cruas do bloco (com `>`)
---@return table model { status_num, title, id, extras[] }
function M.parse_block(block)
  if not M.status then
    M.load_status()
  end
  local rev = status_by_callout()
  local model = { status_num = 1, title = "", id = "", extras = {} }

  -- Linha 1: sempre "> [!callout] <resto>". O callout dá o status.
  local first = block[1] and strip_quote(block[1]) or ""
  local callout = first:match("^%[!(%w+)%]")
  if callout and rev[callout] then
    model.status_num = rev[callout]
  end
  local rest = first:gsub("^%[!%w+%]%s*", "")
  if rest ~= "" and not (is_status_text(rest)) then
    -- resto da linha 1 é o título (novo formato); no legado seria status-texto.
    model.title = vim.trim(rest:match("%*%*(.-)%*%*") or rest)
  end

  -- Linhas 2..n
  for i = 2, #block do
    local content = strip_quote(block[i])
    if model.id == "" then
      local lid = content:match("%[%[(.-)%]%]")
      if lid then
        model.id = vim.trim((lid:gsub("|.*$", "")))
      end
    end
    local is_only_link = content:match("^%s*%[%[.-%]%]%s*$") ~= nil
    local statusy, snum = is_status_text(content)
    local is_branch = content:match("^[Bb]ranch:") ~= nil
    local bold = content:match("%*%*(.-)%*%*")
    if is_only_link or statusy or is_branch then
      if statusy and not callout then
        model.status_num = snum -- fallback quando a linha 1 não tinha callout
      end
    elseif bold and model.title == "" then
      model.title = vim.trim(bold)
    else
      model.extras[#model.extras + 1] = content
    end
  end
  return model
end

---Reconstrói o bloco `>` a partir do modelo (ordem canônica).
---@param model table
---@return string[]
function M.serialize_block(model)
  local st = (M.status and M.status[tostring(model.status_num)]) or { callout = "note", title = "" }
  local out = {
    string.format("> [!%s] **%s**", st.callout or "note", model.title or ""),
    string.format("> [[%s]]", model.id or ""),
    string.format("> %s - %s", model.status_num, st.title or ""),
  }
  local extras = vim.deepcopy(model.extras or {})
  while #extras > 0 and vim.trim(extras[#extras]) == "" do
    table.remove(extras)
  end
  for _, e in ipairs(extras) do
    out[#out + 1] = (vim.trim(e) == "") and ">" or ("> " .. e)
  end
  return out
end

--- Localização de blocos -----------------------------------------------------

---Substitui lines[s..e] por `repl`, retornando um novo array.
---@param lines string[]
---@param s integer
---@param e integer
---@param repl string[]
---@return string[]
local function splice(lines, s, e, repl)
  local out = {}
  for i = 1, s - 1 do
    out[#out + 1] = lines[i]
  end
  for _, r in ipairs(repl) do
    out[#out + 1] = r
  end
  for i = e + 1, #lines do
    out[#out + 1] = lines[i]
  end
  return out
end

---Índice (1-based) da primeira linha de conteúdo após um frontmatter YAML
---opcional (`---` … `---`). Retorna 1 quando não há frontmatter.
---@param lines string[]
---@return integer
local function content_start(lines)
  if lines[1] == "---" then
    for i = 2, #lines do
      if lines[i] == "---" then
        return i + 1
      end
    end
  end
  return 1
end

---Faixa [start, finish] do primeiro bloco `>` de um arquivo, pulando um
---frontmatter YAML e linhas em branco iniciais. nil se não houver callout.
---@param lines string[]
---@return integer|nil, integer|nil
local function first_block_range(lines)
  local start
  for i = content_start(lines), #lines do
    local l = lines[i]
    if l:match("^%s*>") then
      start = i
      break
    elseif not l:match("^%s*$") then
      return nil -- conteúdo não-quote antes de qualquer bloco
    end
  end
  if not start then
    return nil
  end
  local finish = start
  for i = start + 1, #lines do
    if lines[i]:match("^%s*>") then
      finish = i
    else
      break
    end
  end
  return start, finish
end

---Faixa do bloco `>` que contém a linha `lnum` (1-indexed). nil se `lnum` não
---estiver sobre uma linha de blockquote.
---@param lines string[]
---@param lnum integer
---@return integer|nil, integer|nil
local function block_around(lines, lnum)
  if not lines[lnum] or not lines[lnum]:match("^%s*>") then
    return nil
  end
  local s, e = lnum, lnum
  while s > 1 and lines[s - 1]:match("^%s*>") do
    s = s - 1
  end
  while e < #lines and lines[e + 1]:match("^%s*>") do
    e = e + 1
  end
  return s, e
end

---Buffer carregado para `path`, ou nil.
---@param path string
---@return integer|nil
local function loaded_buf(path)
  local b = vim.fn.bufnr(path)
  if b ~= -1 and vim.api.nvim_buf_is_loaded(b) then
    return b
  end
  return nil
end

--- Cache ---------------------------------------------------------------------

---Insere ou atualiza uma task no cache em memória.
---@param path string
function M.update_cache_entry(path)
  local project, task_id = path:match(".*/tasks/([^/]+)/([^/]+)%.md$")
  if not project or RESERVED[project] then
    return
  end
  M.cache = M.cache or {}
  local num = M.parse_status_num(path) or 1
  for _, e in ipairs(M.cache) do
    if e.path == path then
      e.status_num = num
      return
    end
  end
  M.cache[#M.cache + 1] = { project = project, task_id = task_id, status_num = num, path = path }
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
  for _, path in ipairs(vim.fn.glob(root .. "/*/*.md", true, true)) do
    M.update_cache_entry(path)
  end
end

---Tasks cacheadas de um projeto, ordenadas por status e depois id.
---@param project string
---@return table[]
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
  for _, path in ipairs(vim.fn.glob(root .. "/*", true, true)) do
    if vim.fn.isdirectory(path) == 1 then
      local name = vim.fn.fnamemodify(path, ":t")
      if not RESERVED[name] then
        out[#out + 1] = name
      end
    end
  end
  table.sort(out)
  return out
end

--- File operations -----------------------------------------------------------

---Template markdown para uma nova task.
---@param title string título original (não-slugificado)
---@param task_id string id kebab-case
---@param project string id do projeto
---@return string[]
function M.template(title, task_id, project)
  local st = (M.status and M.status["1"]) or { callout = "todo", title = "Backlog" }
  return {
    string.format("> [!%s] **%s**", st.callout or "todo", title),
    string.format("> [[%s]]", task_id),
    string.format("> 1 - %s", st.title or "Backlog"),
    "",
    "## Notas Soltas",
    "- ",
    "",
    "### [" .. project .. "]",
    "- [ ] ",
  }
end

---Reescreve o bloco de callout de um arquivo no disco (substituindo o primeiro
---bloco `>`, ou prependando se não houver).
---@param path string
---@param new_block string[]
local function file_replace_callout(path, new_block)
  local lines = vim.fn.filereadable(path) == 1 and vim.fn.readfile(path) or {}
  local s, e = first_block_range(lines)
  local out
  if s then
    out = splice(lines, s, e, new_block)
  else
    out = vim.deepcopy(new_block)
    out[#out + 1] = ""
    for _, l in ipairs(lines) do
      out[#out + 1] = l
    end
  end
  vim.fn.writefile(out, path)
end

--- CURRENT.md dashboard ------------------------------------------------------

---Lê o bloco de callout inicial (linhas `>` contíguas) de um arquivo de task.
---@param path string
---@return string[]
function M.read_callout(path)
  if vim.fn.filereadable(path) == 0 then
    return {}
  end
  local lines = vim.fn.readfile(path)
  local s, e = first_block_range(lines)
  if not s then
    return {}
  end
  local out = {}
  for i = s, e do
    out[#out + 1] = lines[i]
  end
  return out
end

---Extrai o corpo autoral de `## Notas Avulsas` para preservá-lo entre regens.
---@param path string
---@return string[]
function M.extract_loose_notes(path)
  if vim.fn.filereadable(path) == 0 then
    return {}
  end
  local out, capturing = {}, false
  for _, l in ipairs(vim.fn.readfile(path)) do
    if l:match("^##%s+Notas Avulsas") then
      capturing = true
    elseif capturing and l:match("^##%s+") then
      break -- próxima seção
    elseif capturing then
      out[#out + 1] = l
    end
  end
  while #out > 0 and out[1]:match("^%s*$") do
    table.remove(out, 1)
  end
  while #out > 0 and out[#out]:match("^%s*$") do
    table.remove(out)
  end
  if #out == 1 and out[1]:match("^%-%s*$") then
    out = {} -- descarta o placeholder solitário
  end
  return out
end

---Data do frontmatter de CURRENT.md (YYYY-MM-DD) ou nil.
---@param path string
---@return string|nil
local function frontmatter_date(path)
  if vim.fn.filereadable(path) == 0 then
    return nil
  end
  local lines = vim.fn.readfile(path)
  if lines[1] ~= "---" then
    return nil
  end
  for i = 2, #lines do
    if lines[i] == "---" then
      break
    end
    local d = lines[i]:match("^date:%s*(%d%d%d%d%-%d%d%-%d%d)")
    if d then
      return d
    end
  end
  return nil
end

---Regenera tasks/CURRENT.md como o "board de hoje": frontmatter date/weekday,
---título de planning diário, `## Notas Avulsas` preservadas e uma seção
---`## <project>` por projeto. Antes de sobrescrever, se o board vigente for de
---um dia anterior, arquiva-o em tasks/daily/<data>.md. Recusa sobrescrever um
---CURRENT.md com edições não salvas.
---@param opts? { quiet?: boolean } quiet suprime o aviso de "regen pulado"
function M.rebuild_current(opts)
  opts = opts or {}
  M.ensure_root()
  M.load_status()
  M.build_cache()
  local path = root .. "/CURRENT.md"
  local buf = vim.fn.bufnr(path)
  if buf ~= -1 and vim.api.nvim_buf_is_loaded(buf) and vim.bo[buf].modified then
    if not opts.quiet then
      vim.notify("task-manager: CURRENT.md tem alterações não salvas; regen pulado", vim.log.levels.WARN)
    end
    return
  end

  local today = os.date("%Y-%m-%d")
  local prev = frontmatter_date(path)
  if prev and prev < today then
    local daily_dir = root .. "/daily"
    if vim.fn.isdirectory(daily_dir) == 0 then
      vim.fn.mkdir(daily_dir, "p")
    end
    local archive = daily_dir .. "/" .. prev .. ".md"
    if vim.fn.filereadable(archive) == 0 then
      vim.fn.writefile(vim.fn.readfile(path), archive)
    end
  end

  local notes = M.extract_loose_notes(path)
  local projects = M.list_projects()
  local total = #(M.cache or {})

  local lines = {
    "---",
    "date: " .. today,
    "weekday: " .. os.date("%A"),
    "---",
    "",
    "# Planning diário " .. os.date("%d/%m/%y"),
    "",
    string.format("> Painel gerado automaticamente — %d tasks em %d projetos.", total, #projects),
    "> Abra o gestor com `<leader>ob` · regenere com `<C-a>` · edite o callout sob o cursor com `<leader>oe`.",
    "",
    "## Notas Avulsas",
    "",
  }
  if #notes == 0 then
    lines[#lines + 1] = "- "
  else
    for _, l in ipairs(notes) do
      lines[#lines + 1] = l
    end
  end
  lines[#lines + 1] = ""

  for _, project in ipairs(projects) do
    lines[#lines + 1] = "## " .. project
    lines[#lines + 1] = ""
    local entries = M.entries_for(project)
    if #entries == 0 then
      lines[#lines + 1] = "_(sem tasks)_"
      lines[#lines + 1] = ""
    else
      for _, e in ipairs(entries) do
        for _, cl in ipairs(M.read_callout(e.path)) do
          lines[#lines + 1] = cl
        end
        lines[#lines + 1] = ""
      end
    end
  end

  vim.fn.writefile(lines, path)
  if buf ~= -1 and vim.api.nvim_buf_is_loaded(buf) then
    vim.api.nvim_buf_call(buf, function()
      vim.cmd("silent edit!")
    end)
  end
end

---Migra todos os arquivos de task do formato legado (`> **Título** - [[id]]` +
---linha `> Branch:`) para o atual (título e `[[id]]` em linhas separadas, sem
---`Branch:`). Idempotente. Retorna a quantidade de arquivos reescritos.
---@return integer
function M.migrate_format()
  M.ensure_root()
  M.load_status()
  local n = 0
  for _, path in ipairs(vim.fn.glob(root .. "/*/*.md", true, true)) do
    local project = path:match(".*/tasks/([^/]+)/[^/]+%.md$")
    if project and not RESERVED[project] then
      local lines = vim.fn.readfile(path)
      local s, e = first_block_range(lines)
      if s then
        local blk = {}
        for i = s, e do
          blk[#blk + 1] = lines[i]
        end
        local model = M.parse_block(blk)
        if model.id == "" then
          model.id = vim.fn.fnamemodify(path, ":t:r") -- fallback: id = nome do arquivo
        end
        local newblk = M.serialize_block(model)
        if not vim.deep_equal(newblk, blk) then
          vim.fn.writefile(splice(lines, s, e, newblk), path)
          M.update_cache_entry(path)
          n = n + 1
        end
      end
    end
  end
  return n
end

--- Edição por contexto do cursor + form -------------------------------------

---Resolve a task cujo callout está sob o cursor no buffer atual.
---@return table|nil ctx { source, buf, block_start, block_end, project, task_path, model }
function M.resolve_under_cursor()
  M.ensure_root()
  M.load_status()
  local buf = vim.api.nvim_get_current_buf()
  local path = vim.api.nvim_buf_get_name(buf)
  local lines = vim.api.nvim_buf_get_lines(buf, 0, -1, false)
  local lnum = vim.api.nvim_win_get_cursor(0)[1]

  local s, e = block_around(lines, lnum)
  if not s then
    vim.notify("task-manager: cursor não está sobre um callout", vim.log.levels.WARN)
    return nil
  end
  if not lines[s]:match(">%s*%[!%w+%]") then
    vim.notify("task-manager: bloco sob o cursor não é um callout de task", vim.log.levels.WARN)
    return nil
  end

  local block = {}
  for i = s, e do
    block[#block + 1] = lines[i]
  end
  local model = M.parse_block(block)
  if model.id == "" then
    vim.notify("task-manager: callout sem [[id]]", vim.log.levels.WARN)
    return nil
  end

  local is_current = path:match("/CURRENT%.md$") ~= nil
  local project, task_path
  if is_current then
    for i = s, 1, -1 do
      local p = lines[i]:match("^##%s+(.+)$")
      if p then
        project = vim.trim(p)
        break
      end
    end
    if not project then
      vim.notify("task-manager: não achei o projeto (## ...) acima do callout", vim.log.levels.WARN)
      return nil
    end
    task_path = root .. "/" .. project .. "/" .. model.id .. ".md"
  else
    project = path:match(".*/tasks/([^/]+)/[^/]+%.md$")
    if not project or RESERVED[project] then
      vim.notify("task-manager: buffer não é um arquivo de task", vim.log.levels.WARN)
      return nil
    end
    task_path = path
  end

  return {
    source = is_current and "current" or "task",
    buf = buf,
    block_start = s,
    block_end = e,
    project = project,
    task_path = task_path,
    model = model,
  }
end

-- Rótulos do form (edições devem preservar os prefixos exatos).
local F = {
  title = "Título: ",
  id = "Id / Branch: ",
  status = "Status: ",
  notes = "── Notas (texto livre; uma por linha) ──",
}

---Reconstrói um modelo a partir das linhas do form buffer.
---@param bl string[]
---@param base table modelo original (fallback dos campos)
---@return table
local function parse_form(bl, base)
  local m = {
    status_num = base.status_num,
    title = base.title,
    id = base.id,
    extras = {},
  }
  local in_notes = false
  for _, l in ipairs(bl) do
    if in_notes then
      m.extras[#m.extras + 1] = l
    elseif l == F.notes then
      in_notes = true
    elseif vim.startswith(l, F.title) then
      m.title = vim.trim(l:sub(#F.title + 1))
    elseif vim.startswith(l, F.id) then
      m.id = kebab(vim.trim(l:sub(#F.id + 1)))
    elseif vim.startswith(l, F.status) then
      local n = l:sub(#F.status + 1):match("(%d+)")
      if n then
        m.status_num = tonumber(n)
      end
    end
  end
  while #m.extras > 0 and vim.trim(m.extras[#m.extras]) == "" do
    table.remove(m.extras)
  end
  return m
end

---Abre um form buffer flutuante para editar uma task de uma vez.
---@param model table
---@param on_confirm fun(new_model: table)
function M.edit_task_form(model, on_confirm)
  M.load_status()
  local st = (M.status and M.status[tostring(model.status_num)]) or {}
  local lines = {
    F.title .. (model.title or ""),
    F.id .. (model.id or ""),
    F.status .. model.status_num .. " - " .. (st.title or ""),
    F.notes,
  }
  for _, e in ipairs(model.extras or {}) do
    lines[#lines + 1] = e
  end

  local buf = vim.api.nvim_create_buf(false, true)
  vim.api.nvim_buf_set_lines(buf, 0, -1, false, lines)
  vim.bo[buf].bufhidden = "wipe"
  vim.bo[buf].filetype = "markdown"

  local width = 66
  local height = math.max(#lines + 2, 8)
  local win = vim.api.nvim_open_win(buf, true, {
    relative = "editor",
    width = width,
    height = height,
    row = math.floor((vim.o.lines - height) / 2),
    col = math.floor((vim.o.columns - width) / 2),
    style = "minimal",
    border = "rounded",
    title = " Editar task  ⏎ salva · q cancela · C-s status ",
    title_pos = "center",
  })

  local function close()
    if vim.api.nvim_win_is_valid(win) then
      vim.api.nvim_win_close(win, true)
    end
  end

  local function confirm()
    local bl = vim.api.nvim_buf_get_lines(buf, 0, -1, false)
    close()
    on_confirm(parse_form(bl, model))
  end

  local function pick_status()
    local keys = vim.tbl_keys(M.status or {})
    table.sort(keys, function(a, b)
      return tonumber(a) < tonumber(b)
    end)
    local choices = {}
    for _, n in ipairs(keys) do
      choices[#choices + 1] = { num = n, label = n .. " - " .. (M.status[n].title or "") }
    end
    vim.ui.select(choices, {
      prompt = "Status:",
      format_item = function(c)
        return c.label
      end,
    }, function(c)
      if not c or not vim.api.nvim_buf_is_valid(buf) then
        return
      end
      local bl = vim.api.nvim_buf_get_lines(buf, 0, -1, false)
      for i, l in ipairs(bl) do
        if vim.startswith(l, F.status) then
          vim.api.nvim_buf_set_lines(buf, i - 1, i, false, { F.status .. c.label })
          break
        end
      end
    end)
  end

  vim.keymap.set("n", "<CR>", confirm, { buffer = buf, nowait = true, desc = "Salvar task" })
  vim.keymap.set("n", "q", close, { buffer = buf, nowait = true, desc = "Cancelar" })
  vim.keymap.set("n", "<Esc>", close, { buffer = buf, nowait = true, desc = "Cancelar" })
  vim.keymap.set({ "n", "i" }, "<C-s>", pick_status, { buffer = buf, desc = "Escolher status" })
end

---Aplica um modelo editado: persiste no arquivo-fonte (com rename via
---delete+recreate quando o id muda) e reflete na origem (buffer da task ou
---bloco em CURRENT.md).
---@param ctx table contexto de resolve_under_cursor / edit_task_file
---@param nm table novo modelo vindo do form
function M.apply_edit(ctx, nm)
  M.load_status()
  if nm.id == "" then
    vim.notify("task-manager: id inválido", vim.log.levels.WARN)
    return
  end

  local old_id = ctx.model.id
  local renaming = nm.id ~= old_id
  -- id == nome da branch: renomear o id renomeia a branch (via delete+recreate).

  local project = ctx.project
  local old_path = (ctx.source == "task") and ctx.task_path or (root .. "/" .. project .. "/" .. old_id .. ".md")
  local new_path = renaming and (root .. "/" .. project .. "/" .. nm.id .. ".md") or old_path
  if renaming and vim.fn.filereadable(new_path) == 1 then
    vim.notify("task-manager: já existe task com id " .. nm.id, vim.log.levels.WARN)
    return
  end

  local new_block = M.serialize_block(nm)
  local tbuf = loaded_buf(old_path)

  if renaming then
    if tbuf and vim.bo[tbuf].modified then
      vim.notify("task-manager: salve o arquivo da task antes de renomear o id", vim.log.levels.WARN)
      return
    end
    local src = tbuf and vim.api.nvim_buf_get_lines(tbuf, 0, -1, false)
      or (vim.fn.filereadable(old_path) == 1 and vim.fn.readfile(old_path) or {})
    local s, e = first_block_range(src)
    local out = s and splice(src, s, e, new_block) or vim.deepcopy(new_block)
    vim.fn.writefile(out, new_path)
    if tbuf then
      pcall(vim.api.nvim_buf_set_name, tbuf, new_path)
      vim.api.nvim_buf_call(tbuf, function()
        vim.cmd("silent keepjumps write!")
      end)
    end
    vim.fn.delete(old_path)
    M.remove_cache_entry(old_path)
    M.update_cache_entry(new_path)
  else
    if tbuf then
      local s, e = first_block_range(vim.api.nvim_buf_get_lines(tbuf, 0, -1, false))
      if s then
        vim.api.nvim_buf_set_lines(tbuf, s - 1, e, false, new_block)
      else
        local prepend = vim.deepcopy(new_block)
        prepend[#prepend + 1] = ""
        vim.api.nvim_buf_set_lines(tbuf, 0, 0, false, prepend)
      end
      vim.api.nvim_buf_call(tbuf, function()
        vim.cmd("silent keepjumps write")
      end)
    else
      file_replace_callout(old_path, new_block)
    end
    M.update_cache_entry(old_path)
  end

  -- Reflete in place no CURRENT.md quando a edição começou lá.
  if ctx.source == "current" and vim.api.nvim_buf_is_valid(ctx.buf) then
    vim.api.nvim_buf_set_lines(ctx.buf, ctx.block_start - 1, ctx.block_end, false, new_block)
    vim.api.nvim_buf_call(ctx.buf, function()
      vim.cmd("silent keepjumps write")
    end)
  end

  vim.notify("task-manager: task atualizada (" .. nm.id .. ")", vim.log.levels.INFO)
end

---Ponto de entrada do keymap: edita o callout sob o cursor.
function M.edit_under_cursor()
  local ctx = M.resolve_under_cursor()
  if not ctx then
    return
  end
  M.edit_task_form(ctx.model, function(nm)
    M.apply_edit(ctx, nm)
  end)
end

---Abre o form para um arquivo de task específico (usado pelo picker).
---@param path string
function M.edit_task_file(path)
  M.ensure_root()
  M.load_status()
  local lines = vim.fn.filereadable(path) == 1 and vim.fn.readfile(path) or {}
  local s, e = first_block_range(lines)
  if not s then
    vim.notify("task-manager: arquivo de task sem callout", vim.log.levels.WARN)
    return
  end
  local block = {}
  for i = s, e do
    block[#block + 1] = lines[i]
  end
  local model = M.parse_block(block)
  local project = path:match(".*/tasks/([^/]+)/[^/]+%.md$")
  local ctx = {
    source = "task",
    buf = vim.fn.bufnr(path),
    block_start = s,
    block_end = e,
    project = project,
    task_path = path,
    model = model,
  }
  M.edit_task_form(model, function(nm)
    M.apply_edit(ctx, nm)
  end)
end

--- Pickers -------------------------------------------------------------------

---Level 2: tasks de um único projeto.
---@param project string
function M.open_tasks(project)
  M.ensure_root()
  M.load_status()
  Snacks.picker.pick({
    source = "obsidian_tasks",
    title = "Tasks: " .. project .. "   ⏎ open · C-t new · C-x del · C-e edit · C-o back · ? help",
    show_empty = true,
    finder = function()
      local items = {}
      for _, e in ipairs(M.entries_for(project)) do
        items[#items + 1] = { text = e.task_id, file = e.path, entry = e }
      end
      return items
    end,
    format = function(item)
      local e = item.entry
      local st = (M.status and M.status[tostring(e.status_num)]) or {}
      local hl = st.hl_group or "Normal"
      return {
        { (st.icon or "") .. " ", hl },
        { st.title or ("Status " .. tostring(e.status_num)), hl },
        { " - ", "SnacksPickerDelim" },
        { e.task_id, "SnacksPickerLabel" },
      }
    end,
    confirm = function(picker, item)
      if not item then
        return
      end
      picker:close()
      vim.schedule(function()
        vim.cmd.edit(vim.fn.fnameescape(item.file))
      end)
    end,
    actions = {
      -- <C-t>: cria uma nova task a partir de um título.
      new_task = function(picker)
        vim.ui.input({ prompt = "New task title: " }, function(input)
          if not input or input == "" then
            return
          end
          local id = kebab(input)
          if id == "" then
            vim.notify("task-manager: título de task inválido", vim.log.levels.WARN)
            return
          end
          local path = root .. "/" .. project .. "/" .. id .. ".md"
          if vim.fn.filereadable(path) == 1 then
            vim.notify("task-manager: task já existe: " .. id, vim.log.levels.WARN)
            return
          end
          vim.fn.writefile(M.template(input, id, project), path)
          M.update_cache_entry(path)
          picker:close()
          vim.schedule(function()
            vim.cmd.edit(vim.fn.fnameescape(path))
          end)
        end)
      end,
      -- <C-x>: deleta a task selecionada após confirmação.
      delete_task = function(picker, item)
        item = item or picker:current()
        if not item then
          return
        end
        local choice = vim.fn.confirm("Delete task '" .. item.entry.task_id .. "'?", "&Yes\n&No", 2)
        if choice ~= 1 then
          return
        end
        vim.fn.delete(item.file)
        M.remove_cache_entry(item.file)
        picker:find()
      end,
      -- <C-e>: edita a task selecionada no form multi-campo.
      edit_task = function(picker, item)
        item = item or picker:current()
        if not item then
          return
        end
        local file = item.file
        picker:close()
        vim.schedule(function()
          M.edit_task_file(file)
        end)
      end,
      -- <C-o>: volta para a lista de projetos (Level 1).
      back_to_projects = function(picker)
        picker:close()
        vim.schedule(function()
          M.open_projects()
        end)
      end,
    },
    win = {
      input = {
        keys = {
          ["<c-t>"] = { "new_task", mode = { "i", "n" }, desc = "New task" },
          ["<c-x>"] = { "delete_task", mode = { "i", "n" }, desc = "Delete task" },
          ["<c-e>"] = { "edit_task", mode = { "i", "n" }, desc = "Edit task" },
          ["<c-o>"] = { "back_to_projects", mode = { "i", "n" }, desc = "Back to projects" },
        },
      },
    },
  })
end

---Level 1: projetos.
function M.open_projects()
  M.ensure_root()
  M.load_status()
  M.build_cache()
  Snacks.picker.pick({
    source = "obsidian_projects",
    title = "Task Projects   ⏎ open · C-c new · C-a CURRENT.md · ? help",
    show_empty = true,
    preview = "none",
    layout = { preview = false },
    finder = function()
      local items = {}
      for _, project in ipairs(M.list_projects()) do
        items[#items + 1] = { text = project, project = project }
      end
      return items
    end,
    format = function(item)
      return {
        { "  ", "SnacksPickerDirectory" },
        { item.project, "SnacksPickerDirectory" },
      }
    end,
    confirm = function(picker, item)
      if not item then
        return
      end
      picker:close()
      vim.schedule(function()
        M.open_tasks(item.project)
      end)
    end,
    actions = {
      -- <C-c>: cria um novo diretório de projeto.
      new_project = function(picker)
        vim.ui.input({ prompt = "New project name: " }, function(input)
          if not input or input == "" then
            return
          end
          local id = kebab(input)
          if id == "" then
            vim.notify("task-manager: nome de projeto inválido", vim.log.levels.WARN)
            return
          end
          vim.fn.mkdir(root .. "/" .. id, "p")
          picker:find()
        end)
      end,
      -- <C-a>: regenera e abre o dashboard CURRENT.md.
      open_current = function(picker)
        picker:close()
        vim.schedule(function()
          M.rebuild_current()
          vim.cmd.edit(vim.fn.fnameescape(root .. "/CURRENT.md"))
        end)
      end,
    },
    win = {
      input = {
        keys = {
          ["<c-c>"] = { "new_project", mode = { "i", "n" }, desc = "New project" },
          ["<c-a>"] = { "open_current", mode = { "i", "n" }, desc = "Open CURRENT.md" },
        },
      },
    },
  })
end

--- Autocmds ------------------------------------------------------------------

---Mantém o cache fresco e instala o keymap buffer-local de edição por cursor.
function M.setup_autocmd()
  local grp = vim.api.nvim_create_augroup("obsidian_task_manager", { clear = true })

  -- Ao salvar uma task, regenera o CURRENT.md (silenciosamente) para que novas
  -- tasks/mudanças de status apareçam sozinhas no painel.
  vim.api.nvim_create_autocmd("BufWritePost", {
    group = grp,
    pattern = root .. "/*/*.md",
    callback = function(args)
      local project = args.match:match(".*/tasks/([^/]+)/[^/]+%.md$")
      if not project or RESERVED[project] then
        return
      end
      vim.schedule(function()
        M.rebuild_current({ quiet = true })
      end)
    end,
  })

  -- <leader>oe buffer-local: em CURRENT.md e em qualquer arquivo de task.
  vim.api.nvim_create_autocmd("BufEnter", {
    group = grp,
    pattern = { root .. "/*.md", root .. "/*/*.md" },
    callback = function(args)
      vim.keymap.set("n", "<leader>oe", M.edit_under_cursor, {
        buffer = args.buf,
        desc = "Editar callout sob cursor",
      })
    end,
  })
end

return M
