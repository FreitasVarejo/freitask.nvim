-- freitask.edit — resolver O QUE editar e persistir o resultado.
--
-- Fica entre o form (freitask.ui.form, que só desenha) e o arquivo. É aqui que
-- mora a decisão mais delicada do módulo: uma mudança de id é um RENAME, com
-- tudo que ele arrasta junto — o arquivo, o `id:` do frontmatter, os wikilinks
-- do vault inteiro e a branch git de mesmo nome. Por isso o form valida antes
-- de confirmar: uma linha apagada por acidente desloca os campos posicionais e
-- chegaria aqui como um rename que ninguém pediu.

local C = require("freitask.config")
local board = require("freitask.board")
local cache = require("freitask.cache")
local form = require("freitask.ui.form")
local fs = require("freitask.fs")
local links = require("freitask.links")
local md = require("freitask.md")
local model_ = require("freitask.model")
local path_ = require("freitask.path")
local status = require("freitask.status")
local task = require("freitask.task")

local M = {}

---Resolve a task cujo callout está sob o cursor no buffer atual.
---@return freitask.Ctx|nil
function M.resolve_under_cursor()
  status.ensure_root()
  status.load_status()
  local buf = vim.api.nvim_get_current_buf()
  local path = vim.api.nvim_buf_get_name(buf)
  local lines = vim.api.nvim_buf_get_lines(buf, 0, -1, false)
  local lnum = vim.api.nvim_win_get_cursor(0)[1]

  local s, e = md.block_around(lines, lnum)
  if not s then
    vim.notify("freitask: cursor não está sobre um callout", vim.log.levels.WARN)
    return nil
  end
  -- permissivo como o parser: uma task com tipo inválido (status 0) precisa
  -- continuar abrindo no form, senão o estado quebrado seria inconsertável.
  if not lines[s]:match(">%s*%[![^%]]*%]") then
    vim.notify("freitask: bloco sob o cursor não é um callout de task", vim.log.levels.WARN)
    return nil
  end

  local model = model_.parse_block(vim.list_slice(lines, s, e))
  if model.id == "" then
    vim.notify("freitask: callout sem [[id]]", vim.log.levels.WARN)
    return nil
  end

  local is_current = path:match("/CURRENT%.md$") ~= nil
  local project, task_path, archived
  if is_current then
    for i = s, 1, -1 do
      local p = lines[i]:match("^##%s+(.+)$")
      if p then
        project = vim.trim(p)
        break
      end
    end
    if not project then
      vim.notify("freitask: não achei o projeto (## ...) acima do callout", vim.log.levels.WARN)
      return nil
    end
    -- Task arquivada nunca aparece no CURRENT.md, logo archived é sempre nil aqui.
    task_path = path_.task_file(project, model.id, nil)
  else
    -- split_task_path devolve (project, id, archived); o id vem do próprio
    -- caminho e aqui já temos o do bloco, então só o 1º e o 3º interessam.
    local parts = { path_.split_task_path(path) }
    project, archived = parts[1], parts[3]
    if not project then
      vim.notify("freitask: buffer não é um arquivo de task", vim.log.levels.WARN)
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
    archived = archived,
    task_path = task_path,
    model = model,
  }
end

---Aplica um modelo editado: persiste no arquivo-fonte (com rename via
---delete+recreate quando o id muda) e reflete na origem (buffer da task ou
---bloco em CURRENT.md).
---@param ctx freitask.Ctx contexto de resolve_under_cursor / edit_task_file
---@param nm freitask.Model novo modelo vindo do form
function M.apply_edit(ctx, nm)
  status.load_status()
  if nm.id == "" then
    vim.notify("freitask: id inválido", vim.log.levels.WARN)
    return
  end

  local old_id = ctx.model.id
  local renaming = nm.id ~= old_id
  -- id == nome da branch: renomear o id renomeia a branch (via delete+recreate).

  local project = ctx.project
  -- ctx.archived acompanha o rename: sem ele, renomear o id de uma task
  -- arquivada a ressuscitaria em tasks/<projeto>/ — e no board — em silêncio.
  local old_path = (ctx.source == "task") and ctx.task_path or path_.task_file(project, old_id, ctx.archived)
  local new_path = renaming and path_.task_file(project, nm.id, ctx.archived) or old_path
  if renaming and vim.fn.filereadable(new_path) == 1 then
    vim.notify("freitask: já existe task com id " .. nm.id, vim.log.levels.WARN)
    return
  end

  nm.project, nm.archived = project, ctx.archived
  local new_block = model_.serialize_block(nm)
  local tbuf = fs.loaded_buf(old_path)

  if renaming then
    if tbuf and vim.bo[tbuf].modified then
      vim.notify("freitask: salve o arquivo da task antes de renomear o id", vim.log.levels.WARN)
      return
    end
    -- rename de verdade, em vez de escrever-o-novo + apagar-o-velho: se a
    -- escrita falhar no meio, o arquivo antigo não chega a ser destruído.
    if vim.fn.filereadable(old_path) == 1 and vim.fn.rename(old_path, new_path) ~= 0 then
      vim.notify("freitask: não consegui renomear para " .. nm.id, vim.log.levels.ERROR)
      return
    end
    local src = tbuf and vim.api.nvim_buf_get_lines(tbuf, 0, -1, false)
      or fs.read_lines(new_path)
    local s, e = md.first_block_range(src)
    local out = s and md.splice(src, s, e, new_block) or vim.deepcopy(new_block)
    -- `id:` do frontmatter (injetado pelo obsidian.nvim) espelha o nome do
    -- arquivo; sem isto ele ficaria apontando para o id antigo depois do rename.
    md.update_frontmatter_key(out, "id", nm.id)
    vim.fn.writefile(out, new_path)
    if tbuf then
      fs.retarget_buf(tbuf, new_path)
    end
    cache.remove_cache_entry(old_path)
    cache.update_cache_entry(new_path)
    -- Renomear muda o basename, então quebra TAMBÉM os links curtos `[[id]]`
    -- espalhados pelo vault — não só os path-qualified.
    links.retarget_links(old_path, new_path)
  else
    if tbuf then
      local s, e = md.first_block_range(vim.api.nvim_buf_get_lines(tbuf, 0, -1, false))
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
      task.file_replace_callout(old_path, new_block)
    end
    cache.update_cache_entry(old_path)
  end

  -- Reflete in place no CURRENT.md quando a edição começou lá.
  if ctx.source == "current" and vim.api.nvim_buf_is_valid(ctx.buf) then
    vim.api.nvim_buf_set_lines(ctx.buf, ctx.block_start - 1, ctx.block_end, false, new_block)
    vim.api.nvim_buf_call(ctx.buf, function()
      vim.cmd("silent keepjumps write")
    end)
  end

  -- Editar uma task arquivada NÃO a desarquiva, nem mesmo quando o novo callout
  -- é de task ativa: mover arquivo como efeito colateral de um save é o tipo de
  -- mágica que come dado. O aviso existe para que o silêncio não seja lido como
  -- "voltou pro board".
  if ctx.archived then
    vim.notify(
      string.format("freitask: %s atualizada, mas segue em archived/%s (<leader>oa para desarquivar)", nm.id, ctx.archived),
      vim.log.levels.INFO
    )
  else
    vim.notify("freitask: task atualizada (" .. nm.id .. ")", vim.log.levels.INFO)
  end
end

---Ponto de entrada do keymap: edita o callout sob o cursor.
function M.edit_under_cursor()
  local ctx = M.resolve_under_cursor()
  if not ctx then
    return
  end
  local title = ctx.archived and ("Editar task (archived/" .. ctx.archived .. ")") or "Editar task"
  form.edit_task_form(ctx.model, { title = title }, function(nm)
    M.apply_edit(ctx, nm)
  end)
end

---Abre o form para um arquivo de task específico (usado pelo picker).
---@param path string
function M.edit_task_file(path)
  status.ensure_root()
  status.load_status()
  local s, e, block = md.first_block(fs.read_lines(path))
  if not s then
    vim.notify("freitask: arquivo de task sem callout", vim.log.levels.WARN)
    return
  end
  local model = model_.parse_block(block)
  local project, _, archived = path_.split_task_path(path)
  if not project then
    vim.notify("freitask: não é um arquivo de task: " .. path, vim.log.levels.WARN)
    return
  end
  local ctx = {
    source = "task",
    buf = vim.fn.bufnr(path),
    block_start = s,
    block_end = e,
    project = project,
    archived = archived,
    task_path = path,
    model = model,
  }
  local title = archived and ("Editar task (archived/" .. archived .. ")") or "Editar task"
  form.edit_task_form(model, { title = title }, function(nm)
    M.apply_edit(ctx, nm)
  end)
end

---Abre o form vazio para criar uma task no projeto — mesmo buffer e mesmo
---parser da edição, então só existe um formato para aprender. O id em branco é
---derivado do título por parse_form.
---@param project string
function M.create_task_form(project)
  status.ensure_root()
  status.load_status()
  local seed = { status_num = 1, title = "", id = "", desc = "", extras = {}, project = project }
  form.edit_task_form(seed, { title = "Nova task em " .. project }, function(nm)
    local path = C.root .. "/" .. project .. "/" .. nm.id .. ".md"
    if vim.fn.filereadable(path) == 1 then
      vim.notify("freitask: task já existe: " .. nm.id, vim.log.levels.WARN)
      return
    end
    nm.project = project
    local out = task.template(nm)
    vim.fn.writefile(out, path)
    cache.update_cache_entry(path)
    board.rebuild_current({ quiet = true })
    vim.cmd.edit(vim.fn.fnameescape(path))
  end)
end

return M
