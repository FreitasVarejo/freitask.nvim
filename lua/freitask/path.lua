-- freitask.path — tudo que se deriva de um CAMINHO de arquivo.
--
-- O caminho é a fonte de verdade de duas coisas: a que projeto uma task
-- pertence e se ela está arquivada. Não há flag de frontmatter para nenhuma
-- das duas — ver o comentário de M.is_archived e docs/freitask.md.

local C = require("freitask.config")
local status = require("freitask.status")

local M = {}

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
function M.kebab(s)
  -- vim.fn.tolower e não s:lower(): o lower() do Lua só mexe em bytes ASCII,
  -- então "Á" não virava "á", não casava a tabela de dobra abaixo e acabava
  -- descartado por [^%w%s%-] — "Ávida" saía como "vida". Passou a importar
  -- agora que a criação deriva o id do título.
  s = vim.fn.tolower(s)
  for from, to in pairs(ACCENTS) do
    s = s:gsub(from, to)
  end
  -- `_` precisa entrar na classe permitida: `%w` do Lua é só letra+dígito, e
  -- sem ele o underscore era APAGADO aqui, antes de a regra abaixo poder
  -- convertê-lo em dash — "foo_bar" saía "foobar", não "foo-bar".
  s = s:gsub("[^%w%s%-_]", "") -- descarta o que não for word/space/dash/underscore
  s = s:gsub("[%s_]+", "-") -- espaços e underscores viram dash
  s = s:gsub("%-+", "-") -- colapsa dashes repetidos
  s = s:gsub("^%-+", ""):gsub("%-+$", "") -- apara dashes nas pontas
  return s
end

--- Caminhos de task ----------------------------------------------------------

---Decompõe o caminho de um arquivo de task. Aceita as duas formas:
---  tasks/<projeto>/<id>.md                    → project, id, nil
---  tasks/<projeto>/archived/<tipo>/<id>.md    → project, id, tipo
---Qualquer outra coisa sob tasks/ (projeto reservado, subdiretório
---desconhecido, tipo de archived inválido, arquivo de conflito do Syncthing)
---devolve nil — é o guarda único que impede um arquivo fora de padrão de
---entrar no cache ou no board.
---@param path string
---@return string|nil project, string|nil task_id, string|nil archived
function M.split_task_path(path)
  local project, rest = path:match(".*/tasks/([^/]+)/(.+)%.md$")
  if not project or C.RESERVED[project] then
    return nil
  end
  -- `foo.sync-conflict-20260809-123456-ABCDEFG.md`: o Syncthing cria esse
  -- arquivo ao lado do original quando dois dispositivos editam a mesma task.
  -- Sem este guarda ele passaria por uma task legítima de id
  -- "foo.sync-conflict-…" e apareceria no board como um clone fantasma da task
  -- real — com o agravante de o <C-r> poder arquivar a cópia errada. Ele NÃO é
  -- escondido: M.doctor o reporta, porque o conflito é informação que você
  -- precisa resolver, não lixo a varrer para baixo do tapete.
  if rest:match("%.sync%-conflict%-") then
    return nil
  end
  local tipo, id = rest:match("^archived/([^/]+)/([^/]+)$")
  if tipo then
    if not C.ARCHIVED_SET[tipo] then
      return nil
    end
    return project, id, tipo
  end
  if rest:find("/") then
    return nil -- subdiretório que não é archived/<tipo>: não é uma task
  end
  return project, rest, nil
end

---Caminho absoluto do arquivo de uma task.
---@param project string
---@param id string
---@param archived string|nil tipo de arquivamento, ou nil para task ativa
---@return string
function M.task_file(project, id, archived)
  if archived then
    return string.format("%s/%s/archived/%s/%s.md", C.root, project, archived, id)
  end
  return string.format("%s/%s/%s.md", C.root, project, id)
end

---Diretório vault-relative de uma task, usado como alvo do wikilink da linha 2.
---@param project string
---@param archived string|nil
---@return string
function M.vault_dir(project, archived)
  if archived then
    return string.format("tasks/%s/archived/%s", project, archived)
  end
  return "tasks/" .. project
end

---Verdadeiro se a task está arquivada. O CAMINHO é a fonte de verdade (não há
---mais flag `archived: true` no frontmatter — ver M.migrate_format).
---@param path string
---@return boolean
function M.is_archived(path)
  local _, _, archived = M.split_task_path(path)
  return archived ~= nil
end

---Tipo de arquivamento sugerido para um status — só o DEFAULT do prompt.
---@param status_num integer|nil
---@return string
function M.suggest_archive_type(status_num)
  local callout = status.status_meta(status_num).callout
  if C.DONE_CALLOUTS[callout] then
    return "done"
  elseif C.FAILED_CALLOUTS[callout] then
    return "failed"
  end
  return "dropped"
end

return M
