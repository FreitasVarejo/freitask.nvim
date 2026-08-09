-- freitask.ui.form — o form posicional flutuante.
--
-- Só o BUFFER: desenhar, completar tipos de callout e parsear de volta. Quem
-- decide o que fazer com o modelo resultante é freitask.edit — a separação
-- importa porque este arquivo é o único que pode ser trocado por outra UI sem
-- que nada mais mude.
--
-- O buffer não tem rótulos (eles são virtual text): quatro campos por POSIÇÃO
-- — título / id / "<tipo> [descrição]" / notas. Ao contrário do parser de
-- arquivo, este é ESTRITO: um tipo de callout desconhecido é recusado em vez
-- de virar status 0. Aceitá-lo aqui produziria em silêncio a task inválida que
-- o status 0 existe para tornar visível.

local path_ = require("freitask.path")
local status = require("freitask.status")

local M = {}


--- Form posicional ------------------------------------------------------------
--
-- O buffer não tem rótulos — é para uso rápido, quatro campos por posição:
--   1  título
--   2  id / branch
--   3  <tipo-de-callout> [descrição opcional do estado]
--   4+ notas (texto livre)
-- Os rótulos aparecem como virtual text à direita (ver render_form_labels), o
-- que preserva a densidade do buffer sem custar a legibilidade: sem eles, uma
-- linha apagada por acidente desloca todos os campos em silêncio — e como
-- apply_edit trata mudança de id como rename, o deslocamento chega a mexer em
-- arquivo. Por isso o confirm também VALIDA antes de aplicar.

local FORM_LABELS = { "título", "id / branch", "tipo de status + descrição" }
local form_ns = vim.api.nvim_create_namespace("obsidian_task_form")

---Desenha os rótulos das 3 primeiras linhas como virtual text.
---@param buf integer
local function render_form_labels(buf)
  if not vim.api.nvim_buf_is_valid(buf) then
    return
  end
  vim.api.nvim_buf_clear_namespace(buf, form_ns, 0, -1)
  local n = vim.api.nvim_buf_line_count(buf)
  for i, label in ipairs(FORM_LABELS) do
    if i <= n then
      vim.api.nvim_buf_set_extmark(buf, form_ns, i - 1, 0, {
        virt_text = { { "  " .. label, "Comment" } },
        virt_text_pos = "eol",
      })
    end
  end
end

---Completion de tipos de callout, para a linha 3 do form. Sem isso o formato
---posicional exigiria decorar os 27 tipos.
---@param findstart integer
---@param base string
function M.callout_omnifunc(findstart, base)
  if findstart == 1 then
    local line = vim.api.nvim_get_current_line()
    local col = vim.api.nvim_win_get_cursor(0)[2]
    local start = col
    while start > 0 and line:sub(start, start):match("[%w_]") do
      start = start - 1
    end
    return start
  end
  status.ensure()
  local out = {}
  local keys = vim.tbl_keys(status.status or {})
  table.sort(keys, function(a, b)
    return tonumber(a) < tonumber(b)
  end)
  for _, k in ipairs(keys) do
    local st = status.status[k]
    if st.callout and vim.startswith(st.callout, base) then
      out[#out + 1] = { word = st.callout, menu = k .. " · " .. (st.title or "") }
    end
  end
  return out
end

---Reconstrói um modelo a partir das linhas do form posicional.
---Estrito de propósito: o parser de arquivo é tolerante (tipo desconhecido →
---status 0), mas aqui recusamos, senão um typo viraria uma task inválida em
---silêncio — que é justamente o bug que o status 0 existe para tornar visível.
---@param bl string[]
---@param base freitask.Model modelo original (fallback de project/raw_callout)
---@return freitask.Model|nil model, string|nil err
local function parse_form(bl, base)
  if #bl < 3 then
    return nil, "form precisa de ao menos 3 linhas (título, id, status)"
  end
  status.ensure()

  local title = vim.trim(bl[1])
  local id = path_.kebab(vim.trim(bl[2]))
  if id == "" then
    id = path_.kebab(title) -- id em branco deriva do título (usado na criação)
  end
  if title == "" and id == "" then
    return nil, "título e id vazios"
  end
  if id == "" then
    return nil, "id inválido"
  end

  local tag, desc = vim.trim(bl[3]):match("^(%S+)%s*(.*)$")
  if not tag then
    return nil, "linha 3 precisa começar com um tipo de callout (ex.: todo)"
  end
  local rev = status.by_callout()
  local status_num = rev[tag]
  if not status_num then
    return nil, string.format("tipo de callout desconhecido: %q (use <C-s> ou <C-x><C-o>)", tag)
  end

  local extras = {}
  for i = 4, #bl do
    extras[#extras + 1] = bl[i]
  end
  while #extras > 0 and vim.trim(extras[#extras]) == "" do
    table.remove(extras)
  end

  return {
    status_num = status_num,
    raw_callout = tag,
    title = title,
    id = id,
    desc = vim.trim(desc or ""),
    extras = extras,
    project = base and base.project,
  }
end

---Abre o form posicional flutuante. Usado tanto pela edição quanto pela
---criação — um único formato, um único parser.
---@param model freitask.Model modelo inicial (na criação, campos vazios + status default)
---@param opts { title: string }
---@param on_confirm fun(new_model: freitask.Model)
function M.edit_task_form(model, opts, on_confirm)
  status.load_status()
  -- Status 0 devolve o tipo digitado, para o erro ficar visível e editável.
  local tag = (model.status_num == 0) and vim.trim(model.raw_callout or "")
    or (status.status_meta(model.status_num).callout or "todo")
  local status_line = tag
  if vim.trim(model.desc or "") ~= "" then
    status_line = tag .. " " .. vim.trim(model.desc)
  end

  local lines = {
    model.title or "",
    model.id or "",
    status_line,
  }
  for _, e in ipairs(model.extras or {}) do
    lines[#lines + 1] = e
  end

  local buf = vim.api.nvim_create_buf(false, true)
  vim.api.nvim_buf_set_lines(buf, 0, -1, false, lines)
  vim.bo[buf].bufhidden = "wipe"
  vim.bo[buf].filetype = "markdown"
  vim.bo[buf].omnifunc = "v:lua.require'freitask.ui.form'.callout_omnifunc"

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
    title = " " .. (opts.title or "Task") .. "  ⏎ salva · q cancela · C-s status ",
    title_pos = "center",
  })

  render_form_labels(buf)
  vim.api.nvim_create_autocmd({ "TextChanged", "TextChangedI" }, {
    buffer = buf,
    callback = function()
      render_form_labels(buf)
    end,
  })

  local function close()
    if vim.api.nvim_win_is_valid(win) then
      vim.api.nvim_win_close(win, true)
    end
  end

  local function confirm()
    local bl = vim.api.nvim_buf_get_lines(buf, 0, -1, false)
    local nm, err = parse_form(bl, model)
    if not nm then
      -- não fecha: o buffer fica aberto com o texto para corrigir.
      vim.notify("freitask: " .. err, vim.log.levels.WARN)
      return
    end
    close()
    on_confirm(nm)
  end

  local function pick_status()
    local keys = vim.tbl_keys(status.status or {})
    table.sort(keys, function(a, b)
      return tonumber(a) < tonumber(b)
    end)
    local choices = {}
    for _, n in ipairs(keys) do
      choices[#choices + 1] = { callout = status.status[n].callout, label = n .. " - " .. (status.status[n].title or "") }
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
      -- troca só o tipo, preservando a descrição do estado que já estiver lá.
      local cur = vim.api.nvim_buf_get_lines(buf, 2, 3, false)[1] or ""
      local rest = vim.trim(cur):match("^%S+%s+(.*)$") or ""
      local new = (rest ~= "") and (c.callout .. " " .. rest) or c.callout
      vim.api.nvim_buf_set_lines(buf, 2, 3, false, { new })
      render_form_labels(buf)
    end)
  end

  vim.keymap.set("n", "<CR>", confirm, { buffer = buf, nowait = true, desc = "Salvar task" })
  vim.keymap.set("n", "q", close, { buffer = buf, nowait = true, desc = "Cancelar" })
  vim.keymap.set("n", "<Esc>", close, { buffer = buf, nowait = true, desc = "Cancelar" })
  vim.keymap.set({ "n", "i" }, "<C-s>", pick_status, { buffer = buf, desc = "Escolher status" })
end





return M
