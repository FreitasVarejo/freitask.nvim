-- freitask.task — as operações sobre o ARQUIVO de uma task.
--
-- Criar, mover entre ativo e arquivado, localizar por id. Arquivar e
-- desarquivar são o mesmo motor (move_task) em dois sentidos: nenhum dos dois
-- regenera o CURRENT.md, porque quem chama é que sabe quando vale a pena (o
-- picker regenera uma vez só, no fim).

local C = require("freitask.config")
local cache = require("freitask.cache")
local fs = require("freitask.fs")
local links = require("freitask.links")
local md = require("freitask.md")
local model_ = require("freitask.model")
local path_ = require("freitask.path")
local status = require("freitask.status")

local M = {}

---Template markdown para uma nova task: o bloco, e nada mais.
---
---Ele emitia também `## Notas Soltas` e `### [<projeto>]` com um checkbox
---vazio. Era eco do formato do CURRENT.md (que tem `## Notas Avulsas` e uma
---seção `## <projeto>`) vazado para dentro da task — e nenhuma das tasks do
---vault tem essas seções, porque quem cria pelo form apaga as duas antes de
---escrever. Estrutura fixa que todo mundo apaga não é template, é atrito.
---
---Abaixo do bloco, o arquivo é texto livre. É o que `parse_block` espera e o
---que toda task existente tem.
---@param model freitask.Model
---@return string[]
function M.template(model)
  status.ensure()
  local out = vim.deepcopy(model_.serialize_block(model))
  out[#out + 1] = ""
  return out
end

---Valida projeto e id de uma task nova, sem tocar no disco — a parte com as
---regras, separada para poder ser testada sem vault.
---
---Não valida se o arquivo já existe: isso é disco, e mora em M.create_task.
---@param project string
---@param id string
---@return boolean ok, string|nil err
function M.validate_new(project, id)
  project = vim.trim(project or "")
  id = vim.trim(id or "")

  if project == "" then
    return false, "falta o projeto"
  end
  if project:find("/") then
    return false, "projeto não pode conter '/': " .. project
  end
  if C.RESERVED[project] then
    -- `daily` e `templates` não são projetos; ver o comentário em config.
    return false, string.format("%q é um diretório reservado, não um projeto", project)
  end
  if id == "" then
    return false, "falta o id"
  end
  -- kebab(id) == id em vez de "corrija para mim": o id é o nome do ARQUIVO e o
  -- da BRANCH git. Aceitar "Minha Task" e gravar "minha-task" em silêncio faria
  -- o agente criar a branch com o nome que ele pediu, não com o que existe.
  local k = path_.kebab(id)
  if k ~= id then
    return false, string.format("id precisa ser kebab-case: %q (seria %q)", id, k)
  end
  return true
end

---Cria o arquivo de uma task nova e devolve o caminho.
---
---MOTOR ÚNICO de criação: o form e a CLI passam por aqui. Antes a escrita
---morava dentro de `edit.create_task_form`, alcançável só por quem tivesse um
---Neovim com janela — e era justamente por isso que todo agente montava o
---markdown na mão.
---
---Como o resto de `task`, NÃO regenera o CURRENT.md: quem chama é que sabe se
---vale a pena.
---@param project string
---@param model freitask.Model o id vem em model.id
---@return string|nil path, string|nil err
function M.create_task(project, model)
  status.ensure()
  local id = vim.trim(model.id or "")
  local ok, err = M.validate_new(project, id)
  if not ok then
    return nil, err
  end

  -- Projeto que não existe é CRIADO, sem flag. Um projeto é um diretório sob
  -- projects/ com tasks/ dentro — a presença da pasta é o registro, e exigir
  -- confirmação para criar a pasta seria inventar um cadastro de projetos que
  -- o layout não tem. O risco do typo (`freitask new dotfile ...`) existe, mas
  -- o estrago é uma pasta vazia ao lado, visível na primeira listagem e barata
  -- de desfazer — enquanto a flag seria atrito em todo projeto novo legítimo.
  local dir = path_.task_file(project, id):match("^(.*)/[^/]+$")

  -- Arquivada conta como existente: o id é o nome da branch, e ressuscitar um
  -- id arquivado em silêncio daria duas tasks com a mesma branch.
  local active = path_.task_file(project, id)
  local taken = { active }
  for _, t in ipairs(C.ARCHIVED_TYPES) do
    taken[#taken + 1] = path_.task_file(project, id, t)
  end
  for _, candidate in ipairs(taken) do
    if vim.fn.filereadable(candidate) == 1 then
      return nil, "já existe uma task com esse id: " .. candidate
    end
  end

  if vim.fn.isdirectory(dir) == 0 and vim.fn.mkdir(dir, "p") == 0 then
    return nil, "não consegui criar o diretório " .. dir
  end

  local nm = vim.deepcopy(model)
  nm.id, nm.project, nm.archived = id, project, nil
  if vim.fn.writefile(M.template(nm), active) ~= 0 then
    return nil, "não consegui escrever " .. active
  end
  cache.update_cache_entry(active)
  return active
end

---Reescreve o bloco de callout de um arquivo no disco (substituindo o primeiro
---bloco `>`, ou prependando se não houver).
---@param path string
---@param new_block string[]
function M.file_replace_callout(path, new_block)
  local lines = fs.read_lines(path)
  local s, e = md.first_block_range(lines)
  local out
  if s then
    out = md.splice(lines, s, e, new_block)
  else
    out = vim.deepcopy(new_block)
    out[#out + 1] = ""
    for _, l in ipairs(lines) do
      out[#out + 1] = l
    end
  end
  vim.fn.writefile(out, path)
end

---A fase depois de (des)arquivar. Puro.
---
---`archive <id> done` movia o arquivo e deixava o callout como estava, e o
---`doctor` logo em seguida acusava `callout-vs-pasta` na task que a própria
---CLI acabara de arquivar — a ferramenta produzindo o estado que ela mesma
---reclama. Agora a fase acompanha a pasta.
---
---Só `done` tem regra, nos dois sentidos, porque só ele tem callout: `dropped`
---e `failed` são destinos de arquivamento sem fase correspondente (abandonar
---não é uma fase do trabalho), e o `doctor` também só checa `archived/done/`.
---Ao desarquivar, `done` vira `check` (Pronta): é a fase ativa mais próxima, e
---deixar `done` numa task ativa poria "Arquivada" no board de uma task que não
---está — exatamente o tipo de mentira que arquivar-é-o-caminho existe para
---evitar.
---@param status_num integer|nil
---@param dest string|nil tipo de archived, ou nil para ativa
---@return integer|nil
function M.status_after_move(status_num, dest)
  status.ensure()
  local rev = status.by_callout()
  if dest == "done" then
    return rev["done"] or status_num
  end
  if not dest and status_num == rev["done"] then
    return rev["check"] or status_num
  end
  return status_num
end

---Move o arquivo de uma task entre ativo e arquivado — o motor por trás de
---M.archive_task e M.unarchive_task, que são só nomes para os dois sentidos.
---Reescreve o bloco (o wikilink da linha 2 precisa acompanhar a pasta),
---migra as referências do vault e acerta o cache. NÃO chama rebuild_current:
---quem chama decide quando regenerar (o picker faz isso uma vez só).
---@param path string
---@param dest string|nil tipo de archived, ou nil para desarquivar
---@return string|nil new_path
local function move_task(path, dest)
  local project, id, cur = path_.split_task_path(path)
  if not project then
    vim.notify("freitask: não é um arquivo de task: " .. path, vim.log.levels.WARN)
    return nil
  end
  if dest and not C.ARCHIVED_SET[dest] then
    vim.notify("freitask: tipo de arquivamento inválido: " .. tostring(dest), vim.log.levels.WARN)
    return nil
  end
  if dest == cur then
    return path -- já está onde deveria
  end

  local buf = fs.loaded_buf(path)
  if buf and vim.bo[buf].modified then
    vim.notify("freitask: salve o arquivo da task antes de (des)arquivar", vim.log.levels.WARN)
    return nil
  end

  local new_path = path_.task_file(project, id, dest)
  if vim.fn.filereadable(new_path) == 1 then
    vim.notify("freitask: já existe um arquivo em " .. new_path, vim.log.levels.WARN)
    return nil
  end
  local dir = vim.fn.fnamemodify(new_path, ":h")
  if vim.fn.isdirectory(dir) == 0 then
    vim.fn.mkdir(dir, "p")
  end
  if vim.fn.rename(path, new_path) ~= 0 then
    vim.notify("freitask: não consegui mover " .. id, vim.log.levels.ERROR)
    return nil
  end

  -- O link da linha 2 é path-qualified, então precisa seguir a pasta nova; o
  -- callout acompanha a pasta nos dois sentidos; e o rodapé ganha a entrada de
  -- histórico. Um write só para as três coisas.
  local lines = fs.read_lines(new_path)
  local s, e, block = md.first_block(lines)
  if s then
    local model = model_.parse_block(block)
    if model.id == "" then
      model.id = id
    end
    model.project, model.archived = project, dest
    model.status_num = M.status_after_move(model.status_num, dest)
    lines = md.splice(lines, s, e, model_.serialize_block(model))
  end
  md.append_history(lines, dest)
  vim.fn.writefile(lines, new_path)

  if buf then
    fs.retarget_buf(buf, new_path)
  end

  cache.remove_cache_entry(path)
  if not dest then
    cache.update_cache_entry(new_path)
  end
  links.retarget_links(path, new_path)
  return new_path
end

---Pergunta o tipo de arquivamento, com o default derivado do callout. Existia
---duas vezes — no <C-r> do picker e no <leader>oa — e as duas cópias já tinham
---divergido: uma montava os rótulos de ARCHIVED_TYPES, a outra os tinha
---hardcoded numa string. Acrescentar um quarto tipo teria consertado só metade.
---vim.fn.confirm e não vim.ui.select: é bloqueante e não abre uma janela que
---dispute foco com a do picker.
---@param id string
---@param status_num integer|nil
---@return string|nil tipo nil se cancelado
function M.prompt_archive_type(id, status_num)
  local suggested = path_.suggest_archive_type(status_num)
  local labels, default = {}, 1
  for i, t in ipairs(C.ARCHIVED_TYPES) do
    labels[i] = C.ARCHIVED_PROMPT[t] or t
    if t == suggested then
      default = i
    end
  end
  local choice =
    vim.fn.confirm("Arquivar '" .. id .. "' como:", table.concat(labels, "\n") .. "\n&cancelar", default)
  if choice < 1 or choice > #C.ARCHIVED_TYPES then
    return nil
  end
  return C.ARCHIVED_TYPES[choice]
end

---Confirma o desarquivamento. Contraparte de M.prompt_archive_type.
---@param id string
---@return boolean
function M.confirm_unarchive(id)
  return vim.fn.confirm("Desarquivar '" .. id .. "'?", "&Sim\n&Não", 2) == 1
end

---Arquiva uma task: move para projects/<projeto>/tasks/archived/<tipo>/, tirando-a do
---cache e do CURRENT.md sem apagar nada.
---@param path string
---@param tipo string um de C.ARCHIVED_TYPES
---@return string|nil new_path
function M.archive_task(path, tipo)
  return move_task(path, tipo)
end

---Desarquiva uma task: move de volta para projects/<projeto>/tasks/, devolvendo-a ao
---cache — e portanto ao CURRENT.md na próxima regeneração.
---@param path string
---@return string|nil new_path
function M.unarchive_task(path)
  return move_task(path, nil)
end

---Lê o bloco de callout inicial (linhas `>` contíguas) de um arquivo de task.
---@param path string
---@return string[]
function M.read_callout(path)
  local _, _, block = md.first_block(fs.read_lines(path))
  return block or {}
end

---Lê só a primeira linha de um arquivo de task e extrai o número do status.
---@param path string
---@return integer|nil
function M.parse_status_num(path)
  local blk = M.read_callout(path)
  if #blk == 0 then
    return nil
  end
  status.ensure()
  return model_.parse_block(blk).status_num
end

---Resolve um argumento de linha de comando — caminho ou id — no arquivo da
---task. Existe para a CLI: obrigar um agente a saber se a task está em
---`archived/` para poder citá-la seria justamente o tipo de regra que ele vai
---errar.
---@param ref string caminho (absoluto/relativo) ou id da task
---@return string|nil path, string|nil err
function M.find_task(ref)
  if vim.fn.filereadable(ref) == 1 then
    return vim.fn.fnamemodify(ref, ":p")
  end
  local hits = {}
  for _, t in ipairs(cache.all_tasks()) do
    if t.id == ref then
      hits[#hits + 1] = t.path
    end
  end
  if #hits == 1 then
    return hits[1]
  elseif #hits == 0 then
    return nil, "task não encontrada: " .. ref
  end
  return nil, string.format("id %q é ambíguo (%d arquivos); passe o caminho", ref, #hits)
end

return M
