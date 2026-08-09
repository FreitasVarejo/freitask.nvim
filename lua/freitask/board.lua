-- freitask.board — o CURRENT.md, painel do dia.
--
-- Gerado do zero a cada regen, exceto a seção `## Notas Avulsas`, que é
-- autoral e por isso é extraída e recolocada. Antes de sobrescrever um board
-- de ontem, arquiva-o em tasks/daily/<data>.md: aquilo vira histórico, e
-- histórico não se reescreve (por isso daily/ fica fora do retarget_links).

local C = require("freitask.config")
local cache = require("freitask.cache")
local fs = require("freitask.fs")
local status = require("freitask.status")

local M = {}

---Extrai o corpo autoral de `## Notas Avulsas` para preservá-lo entre regens.
---@param path string
---@return string[]
function M.extract_loose_notes(path)
  if vim.fn.filereadable(path) == 0 then
    return {}
  end
  local out, capturing = {}, false
  for _, l in ipairs(fs.read_lines(path)) do
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
  local lines = fs.read_lines(path)
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
  status.ensure_root()
  status.load_status()
  cache.build_cache()
  local path = C.root .. "/CURRENT.md"
  local buf = vim.fn.bufnr(path)
  if buf ~= -1 and vim.api.nvim_buf_is_loaded(buf) and vim.bo[buf].modified then
    if not opts.quiet then
      vim.notify("freitask: CURRENT.md tem alterações não salvas; regen pulado", vim.log.levels.WARN)
    end
    return
  end

  local today = os.date("%Y-%m-%d")
  local prev = frontmatter_date(path)
  if prev and prev < today then
    local daily_dir = C.root .. "/daily"
    if vim.fn.isdirectory(daily_dir) == 0 then
      vim.fn.mkdir(daily_dir, "p")
    end
    local archive = daily_dir .. "/" .. prev .. ".md"
    if vim.fn.filereadable(archive) == 0 then
      vim.fn.writefile(fs.read_lines(path), archive)
    end
  end

  local notes = M.extract_loose_notes(path)
  local projects = cache.list_projects()
  local total = #(cache.cache or {})

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
    local entries = cache.entries_for(project)
    if #entries == 0 then
      lines[#lines + 1] = "_(sem tasks)_"
      lines[#lines + 1] = ""
    else
      for _, e in ipairs(entries) do
        -- bloco vem do cache (scan_task já o leu); sem I/O aqui.
        for _, cl in ipairs(e.block or {}) do
          lines[#lines + 1] = cl
        end
        lines[#lines + 1] = ""
      end
    end
  end

  vim.fn.writefile(lines, path)
  if buf ~= -1 and vim.api.nvim_buf_is_loaded(buf) then
    fs.reload_buf(buf)
  end
end

return M
