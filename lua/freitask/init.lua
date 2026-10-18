-- freitask — gestão de tasks sobre o vault do Obsidian.
--
-- Este arquivo é a API PÚBLICA e nada mais: não tem lógica, só reúne o que o
-- consumidor (a CLI, lua/freitask/cli.lua) precisa chamar. Cada regra mora
-- num módulo só; ver docs/freitask-internals.md para o mapa e
-- docs/freitask.md para o formato e o fluxo de uso.
--
-- A ordem dos requires não importa: a pilha é acíclica por construção
-- (config → fs/status → path/md → model → cache/links → task → board →
-- doctor), e o init está no topo, onde ninguém importa de volta.

local M = {}

M.config = require("freitask.config")
M.status = require("freitask.status")
M.fs = require("freitask.fs")
M.path = require("freitask.path")
M.md = require("freitask.md")
M.model = require("freitask.model")
M.meta = require("freitask.meta")
M.cache = require("freitask.cache")
M.links = require("freitask.links")
M.task = require("freitask.task")
M.board = require("freitask.board")
M.doctor_mod = require("freitask.doctor")

--- Superfície plana ----------------------------------------------------------
--
-- Os nomes abaixo são a API que já existia quando tudo isto era um módulo só.
-- Mantê-los não é só compatibilidade: `freitask.archive_task(p, "done")` lê
-- melhor num call site do que `freitask.task.archive_task(p, "done")`, e os
-- submódulos acima seguem disponíveis para quem quiser ser explícito.

M.ARCHIVED_TYPES = M.config.ARCHIVED_TYPES

-- status
M.ensure_root = M.status.ensure_root
M.load_status = M.status.load_status
M.status_meta = M.status.status_meta
M.is_hidden = M.status.is_hidden

-- caminhos e modelo
M.split_task_path = M.path.split_task_path
M.is_archived = M.path.is_archived
M.suggest_archive_type = M.path.suggest_archive_type
M.parse_block = M.model.parse_block
M.serialize_block = M.model.serialize_block

-- cache
M.build_cache = M.cache.build_cache
M.update_cache_entry = M.cache.update_cache_entry
M.remove_cache_entry = M.cache.remove_cache_entry
M.entries_for = M.cache.entries_for
M.archived_entries_for = M.cache.archived_entries_for
M.list_projects = M.cache.list_projects

-- arquivo da task
M.template = M.task.template
M.create_task = M.task.create_task
M.create_project = M.task.create_project
M.delete_task = M.task.delete_task
M.check_status_change = M.task.check_status_change
M.validate_new = M.task.validate_new
M.read_callout = M.task.read_callout
M.parse_status_num = M.task.parse_status_num
M.find_task = M.task.find_task
M.archive_task = M.task.archive_task
M.unarchive_task = M.task.unarchive_task
M.apply_edit = M.task.apply_edit
M.retarget_links = M.links.retarget_links

-- board e manutenção
M.rebuild_current = M.board.rebuild_current
M.doctor = M.doctor_mod.doctor
M.migrate_format = M.doctor_mod.migrate_format

return M
