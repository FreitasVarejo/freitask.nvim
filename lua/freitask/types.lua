--- Tipos ---------------------------------------------------------------------
--
-- As formas de tabela deste módulo eram descritas em PROSA nos comentários de
-- cada função (`@param model table` + um parágrafo explicando os campos).
-- Prosa não é verificada: um campo renomeado deixava a descrição errada sem
-- que nada reclamasse. Como `---@class`, o lua_ls confere — e a documentação
-- passa a ficar num lugar só.

---Metadados de um status, como vêm do status.json.
---@class freitask.StatusMeta
---@field title string
---@field callout string
---@field icon string
---@field hl_group string

---Bloco de callout de uma task, decomposto. É o modelo que circula entre o
---parser, o form e o serializer.
---@class freitask.Model
---@field status_num integer 0 = callout não reconhecido (ver STATUS_INVALID)
---@field raw_callout string tipo digitado, preservado verbatim quando status_num == 0
---@field title string
---@field id string == nome do arquivo == nome da branch
---@field desc string descrição do estado (a linha em itálico); "" quando ausente
---@field extras string[] notas/impedimentos, texto livre
---@field project string|nil sem ele, o link sai no formato legado `[[id]]`
---@field archived string|nil tipo de arquivamento; move o alvo do link

---Uma task no cache em memória. `block` é guardado para que o regen do
---CURRENT.md não precise reabrir o arquivo.
---@class freitask.Entry
---@field project string
---@field task_id string
---@field status_num integer
---@field block string[]
---@field path string
---@field archived string|nil

---Contexto de uma edição em curso: de onde o bloco veio e para onde volta.
---@class freitask.Ctx
---@field source "current"|"task"
---@field buf integer
---@field block_start integer|nil
---@field block_end integer|nil
---@field project string
---@field archived string|nil
---@field task_path string
---@field model freitask.Model

---Um achado do doctor. `fixed` diz se ESTA passagem reparou.
---@class freitask.Finding
---@field level "error"|"warn"
---@field kind string
---@field path string vault-relative
---@field msg string
---@field fixed boolean

---Arquivo de task já decomposto pelo caminho.
---@class freitask.TaskRef
---@field path string
---@field project string
---@field id string
---@field archived string|nil

return {}
