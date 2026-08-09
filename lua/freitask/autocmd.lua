-- freitask.autocmd — o que dispara sozinho: regen do board ao salvar uma task
-- e os keymaps buffer-local de edição/arquivamento.
--
-- M.toggle_archive_current_file mora aqui, e não no form, pelo motivo do
-- cabeçalho do picker: mover arquivo é ação com confirmação, e uma quarta
-- linha posicional no form faria uma linha apagada sem querer mover arquivo.

local C = require("freitask.config")
local board = require("freitask.board")
local edit = require("freitask.edit")
local path_ = require("freitask.path")
local task = require("freitask.task")

local M = {}

---Arquiva/desarquiva a task do buffer atual — a contraparte do <C-r> do picker
---para quando você já está dentro do arquivo. Não é campo do form de propósito:
---mover arquivo é uma ação com confirmação, e uma quarta linha posicional
---quebraria o contrato "linha 4+ = notas" (uma linha apagada por acidente
---passaria a mover arquivo de lugar).
function M.toggle_archive_current_file()
  local path = vim.api.nvim_buf_get_name(0)
  local project, id, archived = path_.split_task_path(path)
  if not project then
    vim.notify("freitask: buffer não é um arquivo de task", vim.log.levels.WARN)
    return
  end
  local new_path
  if archived then
    if not task.confirm_unarchive(id) then
      return
    end
    new_path = task.unarchive_task(path)
  else
    local tipo = task.prompt_archive_type(id, task.parse_status_num(path))
    if not tipo then
      return
    end
    new_path = task.archive_task(path, tipo)
  end
  if new_path then
    board.rebuild_current({ quiet = true })
  end
end

---Mantém o cache fresco e instala o keymap buffer-local de edição por cursor.
function M.setup_autocmd()
  local grp = vim.api.nvim_create_augroup("freitask", { clear = true })

  -- Ao salvar uma task, regenera o CURRENT.md (silenciosamente) para que novas
  -- tasks/mudanças de status apareçam sozinhas no painel.
  vim.api.nvim_create_autocmd("BufWritePost", {
    group = grp,
    pattern = C.root .. "/*/*.md",
    callback = function(args)
      -- `*` casa `/` em pattern de autocmd, então isto também dispara para
      -- arquivos em archived/; split_task_path devolve o tipo e nós pulamos —
      -- salvar uma task arquivada não deve mexer no board.
      local project, _, archived = path_.split_task_path(args.match)
      if not project or archived then
        return
      end
      vim.schedule(function()
        board.rebuild_current({ quiet = true })
      end)
    end,
  })

  -- <leader>oe buffer-local: em CURRENT.md e em qualquer arquivo de task
  -- (inclusive arquivadas — `*` casa `/`). <leader>oa só faz sentido num
  -- arquivo de task, então é registrado condicionalmente.
  vim.api.nvim_create_autocmd("BufEnter", {
    group = grp,
    pattern = { C.root .. "/*.md", C.root .. "/*/*.md" },
    callback = function(args)
      vim.keymap.set("n", "<leader>oe", edit.edit_under_cursor, {
        buffer = args.buf,
        desc = "Editar callout sob cursor",
      })
      if path_.split_task_path(vim.api.nvim_buf_get_name(args.buf)) then
        vim.keymap.set("n", "<leader>oa", M.toggle_archive_current_file, {
          buffer = args.buf,
          desc = "Arquivar/desarquivar task",
        })
      end
    end,
  })
end

return M
