-- freitask.ui.picker — o picker de dois níveis (projetos → tasks), no Snacks.
--
-- Só UI: nenhuma regra de task mora aqui. Arquivar/desarquivar são ações com
-- confirmação (<C-r>) e não campos do form, de propósito — mover arquivo como
-- efeito colateral de salvar é o tipo de mágica que come dado.

local C = require("freitask.config")
local board = require("freitask.board")
local cache = require("freitask.cache")
local edit = require("freitask.edit")
local path_ = require("freitask.path")
local status = require("freitask.status")
local task = require("freitask.task")

local M = {}

---Level 2: tasks de um único projeto.
---@param project string
function M.open_tasks(project)
  status.ensure_root()
  status.load_status()
  -- Arquivadas ficam ocultas por default e são lidas do disco só quando este
  -- toggle liga (cache.archived_entries_for) — nunca entram no cache.
  local show_archived = false
  Snacks.picker.pick({
    source = "obsidian_tasks",
    title = "Tasks: "
      .. project
      .. "   ⏎ open · C-t new · C-x del · C-r archive · C-u archived · C-e edit · C-o back · ? help",
    show_empty = true,
    finder = function()
      local items = {}
      for _, e in ipairs(cache.entries_for(project)) do
        items[#items + 1] = { text = e.task_id, file = e.path, entry = e }
      end
      if show_archived then
        for _, e in ipairs(cache.archived_entries_for(project)) do
          items[#items + 1] = { text = e.archived .. " " .. e.task_id, file = e.path, entry = e }
        end
      end
      return items
    end,
    format = function(item)
      local e = item.entry
      local st = status.status_meta(e.status_num)
      local hl = st.hl_group or "Normal"
      local out = {}
      if e.archived then
        -- Prefixo em Comment: a task arquivada aparece visivelmente rebaixada,
        -- para não competir com o board ativo na leitura da lista.
        out[#out + 1] = { string.format("[%s] ", e.archived), "Comment" }
      end
      vim.list_extend(out, {
        { (st.icon or "") .. " ", hl },
        { st.title or ("Status " .. tostring(e.status_num)), hl },
        { " - ", "SnacksPickerDelim" },
        { e.task_id, e.archived and "Comment" or "SnacksPickerLabel" },
      })
      return out
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
      -- <C-t>: cria uma nova task no form posicional (mesmo buffer da edição).
      new_task = function(picker)
        picker:close()
        vim.schedule(function()
          edit.create_task_form(project)
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
        cache.remove_cache_entry(item.file)
        board.rebuild_current({ quiet = true })
        picker:find()
      end,
      -- <C-r>: arquiva a task (perguntando o tipo) ou, se ela já estiver
      -- arquivada, desarquiva — a mesma tecla nos dois sentidos.
      -- vim.fn.confirm e não vim.ui.select: é bloqueante e não abre uma janela
      -- que dispute foco com a do picker.
      archive_task = function(picker, item)
        item = item or picker:current()
        if not item then
          return
        end
        local e = item.entry
        if e.archived then
          if not task.confirm_unarchive(e.task_id) then
            return
          end
          task.unarchive_task(item.file)
        else
          local tipo = task.prompt_archive_type(e.task_id, e.status_num)
          if not tipo then
            return
          end
          task.archive_task(item.file, tipo)
        end
        board.rebuild_current({ quiet = true })
        picker:find()
      end,
      -- <C-u>: mostra/esconde as tasks arquivadas do projeto na mesma lista.
      toggle_archived = function(picker)
        show_archived = not show_archived
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
          edit.edit_task_file(file)
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
          ["<c-r>"] = { "archive_task", mode = { "i", "n" }, desc = "Archive/unarchive task" },
          ["<c-u>"] = { "toggle_archived", mode = { "i", "n" }, desc = "Toggle archived" },
          ["<c-e>"] = { "edit_task", mode = { "i", "n" }, desc = "Edit task" },
          ["<c-o>"] = { "back_to_projects", mode = { "i", "n" }, desc = "Back to projects" },
        },
      },
    },
  })
end

---Level 1: projetos.
function M.open_projects()
  status.ensure_root()
  status.load_status()
  cache.build_cache()
  Snacks.picker.pick({
    source = "obsidian_projects",
    title = "Task Projects   ⏎ open · C-c new · C-a CURRENT.md · ? help",
    show_empty = true,
    preview = "none",
    layout = { preview = false },
    finder = function()
      local items = {}
      for _, project in ipairs(cache.list_projects()) do
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
          local id = path_.kebab(input)
          if id == "" then
            vim.notify("freitask: nome de projeto inválido", vim.log.levels.WARN)
            return
          end
          vim.fn.mkdir(C.root .. "/" .. id, "p")
          picker:find()
        end)
      end,
      -- <C-a>: regenera e abre o dashboard CURRENT.md.
      open_current = function(picker)
        picker:close()
        vim.schedule(function()
          board.rebuild_current()
          vim.cmd.edit(vim.fn.fnameescape(C.root .. "/CURRENT.md"))
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

return M
