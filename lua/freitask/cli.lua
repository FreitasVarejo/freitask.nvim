-- freitask.cli — entrada de linha de comando do freitask.
--
-- Roda sob `nvim --headless -l`. É um SHIM FINO sobre o módulo freitask:
-- nenhuma regra é reimplementada aqui. Um segundo motor (em bash ou python) divergiria
-- do Lua em semanas, e aí existiriam duas regras em vez de uma — que é
-- exatamente o problema que esta CLI existe para resolver.
--
-- Ela existe porque o Neovim não é o único escritor do vault: o Obsidian (aqui
-- e no celular) e agentes de IA com shell também escrevem. Um agente não vai
-- chamar uma função Lua dentro do Neovim; vai chamar `mv`. Dar a ele um
-- executável faz o caminho certo ser o mais fácil.
--
-- Códigos de saída: 0 ok · 1 inconsistências/erro de operação · 2 uso inválido.

local ok, T = pcall(require, "freitask")
if not ok then
  io.stderr:write("freitask: não consegui carregar o módulo freitask\n" .. tostring(T) .. "\n")
  os.exit(2)
end

local args = _G.arg or {}
local cmd = args[1]

---Escreve uma linha em stdout. NÃO usa print(): sob `nvim -l` ele passa pelo
---canal de mensagens do Neovim, e o \n da última linha se perde quando
---os.exit encerra o processo. Saída sem quebra de linha final quebra qualquer
---consumidor que leia linha a linha — que é o caso de uso desta CLI.
local function say(s)
  io.stdout:write(tostring(s) .. "\n")
end

-- O motor avisa por vim.notify, que sob `nvim -l` sai em stderr SEM quebra de
-- linha — e o aviso colava no caminho que o comando imprime em stdout logo
-- depois. Aqui cada aviso vira uma linha de stderr: stdout fica só com o dado.
vim.notify = function(msg)
  io.stderr:write(tostring(msg) .. "\n")
end

local function quit(code)
  io.stdout:flush()
  os.exit(code)
end

local function die(msg, code)
  io.stdout:flush()
  io.stderr:write("freitask: " .. msg .. "\n")
  os.exit(code or 2)
end

---Extrai as flags `--x` de args, devolvendo os posicionais e um set de flags.
---
---`value_flags` lista as que CONSOMEM o argumento seguinte (`--desc "texto"`);
---o resto é booleana. A lista é explícita, e não "toda flag consome o
---próximo", porque `freitask list --json` engoliria um posicional em silêncio.
---`--x=valor` funciona para qualquer uma, e é o que um script deve preferir.
---@param from integer
---@param value_flags? table<string, boolean>
---@return string[] pos, table<string, string|boolean> flags
local function parse_flags(from, value_flags)
  value_flags = value_flags or {}
  local pos, flags = {}, {}
  local i = from
  while i <= #args do
    local a = args[i]
    local k, v = a:match("^%-%-([^=]+)=(.*)$")
    if k then
      flags[k] = v
    else
      local f = a:match("^%-%-(.+)$")
      if f and value_flags[f] then
        i = i + 1
        flags[f] = args[i] or die("falta o valor de --" .. f)
      elseif f then
        flags[f] = true
      else
        pos[#pos + 1] = a
      end
    end
    i = i + 1
  end
  return pos, flags
end

local USAGE = [[
freitask — gestão de tasks do vault do Obsidian

  freitask current
      Abre o CURRENT.md no Neovim interativo (tratado pelo shim em bash,
      antes de chegar aqui).

  freitask doctor [--fix] [--json] [--quiet]
      Verifica a integridade do vault. Read-only por padrão; --fix repara o
      que é derivável do caminho (nunca inventa datas nem resolve conflitos).

  freitask archive   <id|caminho> <done|dropped|failed>
  freitask unarchive <id|caminho>
  freitask rename    <id|caminho> <novo-id>
      Movem/renomeiam mantendo os invariantes: wikilink da linha 2, `id:` do
      frontmatter, referências do vault inteiro e o log em `## Histórico`.
      `archive <id> done` também grava o callout `done`, e desarquivar uma
      task `done` a devolve como `check` (Pronta).

  freitask new <projeto> <id> "<título>" [--desc "<estado>"]
      Cria uma task em projects/<projeto>/tasks/<id>.md e regenera o painel.
      O <id> é o nome do arquivo E o da branch git: precisa vir em kebab-case,
      e não é corrigido em silêncio. Recusa projeto reservado e id já existente
      (inclusive arquivado). Projeto que ainda não existe é criado.

  freitask rebuild
      Regenera o CURRENT.md sem tocar em task nenhuma. É o que falta depois que
      um agente sem shell (o claude.ai pelo MCP) criou o arquivo na mão.

  freitask set <id|caminho> [--status <fase>] [--title "<título>"] [--desc "<estado>"]
      Troca a fase (o tipo do callout), o título e/ou a descrição em itálico,
      reserializando o bloco pelo mesmo código de sempre. `--desc ""` apaga a
      descrição. Recusa `done` (quem grava `done` é `archive <id> done`).

  freitask delete <id|caminho>
      Apaga a task, sem perguntar — quem confirma é quem chama. Referências a
      ela ficam penduradas de propósito: o `doctor` as aponta.

  freitask new-project <nome>
      Cria projects/<nome-em-kebab>/tasks/. Projeto já existente é erro.

  freitask list [--json] [--archived]
      Lista as tasks. O JSON traz, por task: project, id, path, archived,
      status (título da fase), status_num, callout, title, desc e
      suggested_archive.

  freitask projects [--json]
      Lista os projetos (inclusive os sem task ativa) e quantas tasks ativas
      cada um tem.

  freitask statuses [--json]
      O vocabulário de fases do .freitask/status.json, em ordem, com ícone e
      grupo de highlight; o 0 é o sentinela "Sem status".

NUNCA mova nem renomeie um arquivo de task com `mv`: as referências do vault
ficam penduradas e o histórico não é escrito. E prefira `new` a escrever o
markdown na mão: o bloco é serializado pelo mesmo código que o Neovim usa.
]]

--- doctor --------------------------------------------------------------------
if cmd == "doctor" then
  local _, flags = parse_flags(2)
  local found = T.doctor({ fix = flags.fix })

  if flags.json then
    say(vim.json.encode({ ok = #found == 0, findings = found }))
  elseif not flags.quiet then
    if #found == 0 then
      say("freitask: vault consistente.")
    else
      for _, f in ipairs(found) do
        say(
          string.format(
            "%-6s %-22s %s\n         %s%s",
            f.level,
            f.kind,
            f.path,
            f.msg,
            f.fixed and "  [reparado]" or ""
          )
        )
      end
      local pend = 0
      for _, f in ipairs(found) do
        if not f.fixed then
          pend = pend + 1
        end
      end
      say(string.format("\n%d achado(s), %d pendente(s).", #found, pend))
      if pend > 0 and not flags.fix then
        say("Repare o que for automático com: freitask doctor --fix")
      end
    end
  end

  -- Só o que sobrou por reparar conta como falha, para que `--fix` deixe o
  -- healthcheck verde quando resolveu tudo.
  for _, f in ipairs(found) do
    if not f.fixed then
      quit(1)
    end
  end
  quit(0)
end

--- archive / unarchive -------------------------------------------------------
if cmd == "archive" or cmd == "unarchive" then
  local pos = parse_flags(2)
  local ref = pos[1] or die("falta o id ou caminho da task")
  local path, err = T.find_task(ref)
  if not path then
    die(err, 1)
  end

  local new
  if cmd == "archive" then
    local tipo = pos[2] or die("falta o tipo: " .. table.concat(T.ARCHIVED_TYPES, "|"))
    new = T.archive_task(path, tipo)
  else
    new = T.unarchive_task(path)
  end
  if not new then
    die("operação não concluída (veja as mensagens acima)", 1)
  end
  T.rebuild_current({ quiet = true })
  say(new)
  quit(0)
end

--- rename --------------------------------------------------------------------
if cmd == "rename" then
  local pos = parse_flags(2)
  local ref = pos[1] or die("falta o id ou caminho da task")
  local new_id = pos[2] or die("falta o novo id")
  local path, err = T.find_task(ref)
  if not path then
    die(err, 1)
  end

  local project, _, archived = T.split_task_path(path)
  local model = T.parse_block(T.read_callout(path))
  if model.id == "" then
    model.id = vim.fn.fnamemodify(path, ":t:r")
  end
  local ctx = {
    source = "task",
    buf = -1,
    project = project,
    archived = archived,
    task_path = path,
    model = model,
  }
  local nm = vim.deepcopy(model)
  nm.id = new_id
  T.apply_edit(ctx, nm)
  local expected = vim.fn.fnamemodify(path, ":h") .. "/" .. new_id .. ".md"
  if vim.fn.filereadable(expected) == 0 then
    die("rename não concluído (id inválido ou já existente?)", 1)
  end
  T.rebuild_current({ quiet = true })
  say(expected)
  quit(0)
end

--- new ----------------------------------------------------------------------
if cmd == "new" then
  local pos, flags = parse_flags(2, { desc = true })
  local project = pos[1] or die("falta o projeto")
  local id = pos[2] or die("falta o id")
  local title = pos[3] or die("falta o título")

  T.ensure_root()
  T.load_status()
  T.build_cache() -- create_task consulta o cache para listar projetos no erro

  local path, err = T.create_task(project, {
    status_num = 1, -- toda task nasce em `todo`; a fase é escolha de quem trabalha
    raw_callout = "",
    title = title,
    id = id,
    desc = type(flags.desc) == "string" and flags.desc or "",
    extras = {},
  })
  if not path then
    die(err, 1)
  end

  -- Aqui SIM regenera: era exatamente isto que faltava quando a task nascia de
  -- um arquivo escrito à mão, e o painel ficava dias para trás.
  T.rebuild_current({ quiet = true })
  say(path)
  quit(0)
end

--- rebuild ------------------------------------------------------------------
if cmd == "rebuild" then
  parse_flags(2)
  T.ensure_root()
  T.rebuild_current({ quiet = true })
  say(T.config.current)
  quit(0)
end

--- set ----------------------------------------------------------------------
if cmd == "set" then
  local pos, flags = parse_flags(2, { status = true, title = true, desc = true })
  local ref = pos[1] or die("falta o id ou caminho da task")
  if flags.status == nil and flags.title == nil and flags.desc == nil then
    die("nada a mudar: passe --status, --title e/ou --desc")
  end
  T.load_status()
  local path, err = T.find_task(ref)
  if not path then
    die(err, 1)
  end

  local project, _, archived = T.split_task_path(path)
  local block = T.read_callout(path)
  if #block == 0 then
    die("task sem callout (rode `freitask doctor`): " .. path, 1)
  end
  local model = T.parse_block(block)
  if model.id == "" then
    model.id = vim.fn.fnamemodify(path, ":t:r")
  end
  local nm = vim.deepcopy(model)

  if flags.status then
    local num, serr = T.check_status_change(flags.status, archived)
    if not num then
      -- Fase que não existe é erro de uso (2); fase existente mas recusada é
      -- operação negada (1) — quem chama pode querer distinguir os dois.
      die(serr, T.status.by_callout()[flags.status] and 1 or 2)
    end
    nm.status_num, nm.raw_callout = num, flags.status
  end
  if flags.title then
    nm.title = vim.trim(flags.title)
    if nm.title == "" then
      die("o título não pode ficar vazio")
    end
  end
  if flags.desc then
    nm.desc = vim.trim(flags.desc)
  end

  T.apply_edit({
    source = "task",
    buf = -1,
    project = project,
    archived = archived,
    task_path = path,
    model = model,
  }, nm)
  local got = T.parse_block(T.read_callout(path))
  if got.status_num ~= nm.status_num or got.title ~= nm.title or got.desc ~= nm.desc then
    die("set não concluído (veja as mensagens acima)", 1)
  end
  T.rebuild_current({ quiet = true })
  say(path)
  quit(0)
end

--- delete -------------------------------------------------------------------
if cmd == "delete" then
  local pos = parse_flags(2)
  local ref = pos[1] or die("falta o id ou caminho da task")
  local path, err = T.find_task(ref)
  if not path then
    die(err, 1)
  end
  local ok_del, derr = T.delete_task(path)
  if not ok_del then
    die(derr, 1)
  end
  T.rebuild_current({ quiet = true })
  say(path)
  quit(0)
end

--- new-project --------------------------------------------------------------
if cmd == "new-project" then
  local pos = parse_flags(2)
  local name = pos[1] or die("falta o nome do projeto")
  T.ensure_root()
  local id, err = T.create_project(name)
  if not id then
    die(err, 1)
  end
  say(id)
  quit(0)
end

--- projects -----------------------------------------------------------------
if cmd == "projects" then
  local _, flags = parse_flags(2)
  T.load_status()
  T.build_cache()
  local rows = {}
  for _, p in ipairs(T.list_projects()) do
    rows[#rows + 1] = { project = p, active = #T.entries_for(p) }
  end
  if flags.json then
    -- vim.json.encode({}) dá "{}", e quem lê espera uma lista.
    say(#rows == 0 and "[]" or vim.json.encode(rows))
  else
    for _, r in ipairs(rows) do
      say(string.format("%-14s %d", r.project, r.active))
    end
  end
  quit(0)
end

--- statuses -----------------------------------------------------------------
if cmd == "statuses" then
  local _, flags = parse_flags(2)
  T.load_status()
  local nums = {}
  for k in pairs(T.status.status or {}) do
    nums[#nums + 1] = tonumber(k)
  end
  table.sort(nums)
  table.insert(nums, 1, 0)
  local rows = {}
  for _, n in ipairs(nums) do
    local st = T.status_meta(n)
    rows[#rows + 1] = { num = n, callout = st.callout, title = st.title, icon = st.icon, hl_group = st.hl_group }
  end
  if flags.json then
    say(vim.json.encode(rows))
  else
    for _, r in ipairs(rows) do
      say(string.format("%d  %-10s %s", r.num, r.callout, r.title))
    end
  end
  quit(0)
end

--- list ----------------------------------------------------------------------

---A linha de uma task no `list`. O bloco já está na entry (o cache o guarda
---para o regen), então isto não relê arquivo nenhum.
---@param e freitask.Entry
---@return table
local function list_row(e)
  local m = T.parse_block(e.block or {})
  local st = T.status_meta(e.status_num)
  return {
    project = e.project,
    id = e.task_id,
    status = st.title,
    status_num = e.status_num,
    -- O tipo digitado, mesmo quando inválido (status 0): é o que o usuário
    -- precisa ver para consertar.
    callout = (m.raw_callout ~= "" and m.raw_callout) or st.callout,
    title = m.title,
    desc = m.desc,
    archived = e.archived or vim.NIL,
    suggested_archive = T.suggest_archive_type(e.status_num),
    path = e.path,
  }
end

if cmd == "list" then
  local _, flags = parse_flags(2)
  T.load_status()
  T.build_cache()
  local rows = {}
  for _, p in ipairs(T.list_projects()) do
    for _, e in ipairs(T.entries_for(p)) do
      rows[#rows + 1] = list_row(e)
    end
    if flags.archived then
      for _, e in ipairs(T.archived_entries_for(p)) do
        rows[#rows + 1] = list_row(e)
      end
    end
  end
  if flags.json then
    say(#rows == 0 and "[]" or vim.json.encode(rows))
  else
    for _, r in ipairs(rows) do
      say(
        string.format(
          "%-14s %-34s %-12s %s",
          r.project,
          r.id,
          r.status,
          r.archived ~= vim.NIL and ("[" .. r.archived .. "]") or ""
        )
      )
    end
  end
  quit(0)
end

if cmd == nil or cmd == "help" or cmd == "-h" or cmd == "--help" then
  say(USAGE)
  quit(cmd == nil and 2 or 0)
end

die("comando desconhecido: " .. tostring(cmd) .. "\n\n" .. USAGE)
