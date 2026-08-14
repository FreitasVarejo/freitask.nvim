-- util.freitask_cli — entrada de linha de comando do freitask.
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
local function parse_flags(from)
  local pos, flags = {}, {}
  for i = from, #args do
    local a = args[i]
    local f = a:match("^%-%-(.+)$")
    if f then
      flags[f] = true
    else
      pos[#pos + 1] = a
    end
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

  freitask list [--json] [--archived]
      Lista as tasks.

NUNCA mova nem renomeie um arquivo de task com `mv`: as referências do vault
ficam penduradas e o histórico não é escrito. Use os comandos acima.
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
        say(string.format("%-6s %-22s %s\n         %s%s", f.level, f.kind, f.path, f.msg, f.fixed and "  [reparado]" or ""))
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

--- list ----------------------------------------------------------------------
if cmd == "list" then
  local _, flags = parse_flags(2)
  T.load_status()
  T.build_cache()
  local rows = {}
  for _, p in ipairs(T.list_projects()) do
    for _, e in ipairs(T.entries_for(p)) do
      rows[#rows + 1] =
        { project = p, id = e.task_id, status = T.status_meta(e.status_num).title, archived = vim.NIL, path = e.path }
    end
    if flags.archived then
      for _, e in ipairs(T.archived_entries_for(p)) do
        rows[#rows + 1] =
          { project = p, id = e.task_id, status = T.status_meta(e.status_num).title, archived = e.archived, path = e.path }
      end
    end
  end
  if flags.json then
    say(vim.json.encode(rows))
  else
    for _, r in ipairs(rows) do
      say(
        string.format("%-14s %-34s %-12s %s", r.project, r.id, r.status, r.archived ~= vim.NIL and ("[" .. r.archived .. "]") or "")
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
