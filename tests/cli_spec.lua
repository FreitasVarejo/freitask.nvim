-- Testes de ponta a ponta da CLI, num vault descartável.
--
-- Cobre o que freitask_spec.lua não alcança por ser só funções puras: as
-- operações que escrevem no disco (criar, trocar fase, renomear, arquivar,
-- mover de projeto, apagar) e o contrato JSON que a TUI e os agentes leem.
-- Cada chamada é um processo `nvim -l` de verdade, com HOME apontando para um
-- diretório temporário — o módulo deriva o vault de `~`, então é só isso que
-- precisa mudar para nunca tocar o vault real.

local repo = debug.getinfo(1, "S").source:sub(2):match("(.*)/tests/")
local cli = repo .. "/lua/freitask/cli.lua"
local home = vim.fn.tempname()
local vault = home .. "/ObsidianVault"
vim.fn.mkdir(vault .. "/projects", "p")

---@return integer code, string stdout, string stderr
local function run(...)
  local cmd = { "nvim", "--clean", "--cmd", "set runtimepath+=" .. repo, "-l", cli, ... }
  local r = vim.system(cmd, { env = { HOME = home }, text = true }):wait()
  return r.code, r.stdout or "", r.stderr or ""
end

local function json(...)
  local code, out, err = run(...)
  eq(0, code, "exit de " .. table.concat({ ... }, " ") .. ": " .. err)
  return vim.json.decode(out)
end

local function read(rel)
  return table.concat(vim.fn.readfile(vault .. "/" .. rel), "\n")
end

local TASK = "projects/meu-projeto/tasks/t1.md"

---Número da fase pelo callout, como o `statuses --json` o devolve. O número
---ordena o board e muda quando o vocabulário cresce, então o teste o pergunta.
local function phase_num(callout)
  for _, r in ipairs(json("statuses", "--json")) do
    if r.callout == callout then
      return r.num
    end
  end
end

---Põe um frontmatter de uma linha no topo do arquivo de uma task, como o
---celular ou um agente faria à mão.
local function set_frontmatter(rel, line)
  local lines = vim.fn.readfile(vault .. "/" .. rel)
  vim.fn.writefile(vim.list_extend({ "---", line, "---" }, lines), vault .. "/" .. rel)
end

describe("cli: projetos", function()
  it("lista vazia é [] e não {}", function()
    local _, out = run("projects", "--json")
    eq("[]\n", out)
    _, out = run("list", "--json")
    eq("[]\n", out)
  end)

  it("new-project cria em kebab e recusa repetido", function()
    local code, out = run("new-project", "Meu Projeto")
    eq(0, code)
    eq("meu-projeto\n", out)
    eq(1, (run("new-project", "meu-projeto")))
    eq(1, (run("new-project", "daily")))
    eq({ { project = "meu-projeto", active = 0 } }, json("projects", "--json"))
  end)
end)

describe("cli: set", function()
  it("troca fase, título e descrição, e stdout é só o caminho", function()
    eq(0, (run("new", "meu-projeto", "t1", "Um", "--desc", "começando")))
    vim.fn.writefile({ "ref [[t1]] e [[projects/meu-projeto/tasks/t1|t1]]" }, vault .. "/n.md")

    local code, out = run("set", "t1", "--status", "example", "--title", "Um novo")
    eq(0, code)
    eq(vault .. "/" .. TASK .. "\n", out)
    local body = read(TASK)
    truthy(body:find("> [!example] Um novo", 1, true), body)
    truthy(body:find("> _começando_", 1, true), body)

    eq(0, (run("set", "t1", "--desc", "")))
    falsy(read(TASK):find("começando", 1, true))
  end)

  it("códigos de saída: recusa 1, uso inválido 2", function()
    eq(1, (run("set", "t1", "--status", "done")))
    eq(2, (run("set", "t1", "--status", "bogus")))
    eq(2, (run("set", "t1")))
    eq(2, (run("set", "t1", "--title", "  ")))
    eq(1, (run("set", "nao-existe", "--status", "todo")))
  end)

  it("regenera o CURRENT.md", function()
    truthy(read("CURRENT.md"):find("[!example] Um novo", 1, true))
  end)
end)

describe("cli: kebab", function()
  it("é o mesmo kebab do motor, acento incluído", function()
    local code, out = run("kebab", "Ávida Revisão_final")
    eq(0, code)
    eq("avida-revisao-final\n", out)
    eq(1, (run("kebab", "!!!")))
  end)
end)

describe("cli: list e statuses", function()
  it("list --json traz os campos da TUI", function()
    local rows = json("list", "--json")
    eq(1, #rows)
    local r = rows[1]
    eq("meu-projeto", r.project)
    eq("t1", r.id)
    eq("Em implementação", r.status)
    eq(phase_num("example"), r.status_num)
    eq("example", r.callout)
    eq("Um novo", r.title)
    eq("", r.desc)
    eq("dropped", r.suggested_archive)
    eq(false, r.hidden)
    eq(vim.NIL, r.prioridade)
    eq(vim.NIL, r.archived)
    eq(vault .. "/" .. TASK, r.path)
  end)

  it("statuses começa pelo sentinela 0 e segue em ordem", function()
    local rows = json("statuses", "--json")
    eq(0, rows[1].num)
    eq("invalid", rows[1].callout)
    eq("todo", rows[2].callout)
    for i = 2, #rows do
      truthy(rows[i].num > rows[i - 1].num)
      truthy(rows[i].icon and rows[i].hl_group)
    end
  end)
end)

describe("cli: fases novas, prioridade e Algum dia", function()
  it("statuses traz Refinada e Algum dia, na ordem da narrativa", function()
    local rows = json("statuses", "--json")
    local by = {}
    for _, r in ipairs(rows) do
      by[r.callout] = r
    end
    truthy(by.tip and by.quote, "as duas fases novas estão no vocabulário")
    eq("Refinada", by.tip.title)
    eq("Algum dia", by.quote.title)
    truthy(by.abstract.num < by.tip.num and by.tip.num < by.example.num)
    truthy(by.check.num < by.quote.num and by.quote.num < by.done.num)
    eq(true, by.quote.hidden)
    eq(false, by.tip.hidden)
    eq(false, rows[1].hidden, "o sentinela também diz")
  end)

  it("a task alta sobe ao topo qualquer que seja a fase", function()
    eq(0, (run("new", "meu-projeto", "t-alta", "Alta, em homologação")))
    eq(0, (run("set", "t-alta", "--status", "question")))
    set_frontmatter("projects/meu-projeto/tasks/t-alta.md", "prioridade: alta")

    local rows = json("list", "--json")
    eq("t-alta", rows[1].id, "a alta em homologação vem antes da normal em implementação")
    eq("alta", rows[1].prioridade)
    eq("t1", rows[2].id)
    eq(vim.NIL, rows[2].prioridade)

    -- O frontmatter foi escrito à mão, como o celular faria: o painel só o vê no
    -- próximo rebuild.
    eq(0, (run("rebuild")))
    local board = read("CURRENT.md")
    truthy(board:find("t-alta", 1, true) < board:find("tasks/t1", 1, true), "o painel segue a mesma ordem")

    -- Arquivada não tem o que priorizar: a marca não vai com ela para a TUI.
    eq(0, (run("archive", "t-alta", "dropped")))
    local achou = false
    for _, r in ipairs(json("list", "--json", "--archived")) do
      if r.id == "t-alta" then
        achou = true
        eq(vim.NIL, r.prioridade)
      end
    end
    truthy(achou, "a task arquivada tem de aparecer no list --archived")
    eq(0, (run("delete", "t-alta")))
  end)

  it("Algum dia sai do CURRENT.md, mas segue no list --json com hidden", function()
    eq(0, (run("new", "meu-projeto", "t-sem-pressa", "Quando der")))
    eq(0, (run("set", "t-sem-pressa", "--status", "quote")))

    local rows = json("list", "--json")
    local by = {}
    for _, r in ipairs(rows) do
      by[r.id] = r
    end
    eq(true, by["t-sem-pressa"].hidden)
    eq(false, by.t1.hidden)
    eq("Algum dia", by["t-sem-pressa"].status)

    local board = read("CURRENT.md")
    falsy(board:find("t-sem-pressa", 1, true), board)
    truthy(board:find("tasks/t1", 1, true), board)
    truthy(board:find("1 tasks em 1 projetos", 1, true), "o total conta só o que o painel mostra: " .. board)
    eq({ { project = "meu-projeto", active = 1 } }, json("projects", "--json"), "projects conta o mesmo que o painel")

    -- Largada, a task deixa de ser "escondida": a arquivada tem o próprio toggle,
    -- e quem a mostra não espera que uma delas continue sumida.
    eq(0, (run("archive", "t-sem-pressa", "dropped")))
    for _, r in ipairs(json("list", "--json", "--archived")) do
      if r.id == "t-sem-pressa" then
        eq("dropped", r.archived)
        eq(false, r.hidden)
      end
    end
    eq(0, (run("delete", "t-sem-pressa")))
  end)

  it("o doctor acusa prioridade desconhecida, e só ela", function()
    eq(0, (run("new", "meu-projeto", "t-typo", "Com typo")))
    set_frontmatter("projects/meu-projeto/tasks/t-typo.md", "prioridade: Alta")
    local code, out = run("doctor", "--json")
    eq(1, code)
    local kinds = {}
    for _, f in ipairs(vim.json.decode(out).findings) do
      kinds[#kinds + 1] = f.kind
    end
    eq({ "prioridade-desconhecida" }, kinds)

    local typo = vault .. "/projects/meu-projeto/tasks/t-typo.md"
    vim.fn.writefile(
      vim.tbl_map(function(l)
        return l == "prioridade: Alta" and "prioridade: alta" or l
      end, vim.fn.readfile(typo)),
      typo
    )
    eq(0, (run("doctor")), "alta, exato, é um valor válido")
    eq(0, (run("delete", "t-typo")))
  end)
end)

-- Fica ANTES do bloco de arquivar/renomear/apagar de propósito: aquele termina
-- num vault com link pendurado (é o que o último caso do bloco confere), e aqui
-- o `doctor` precisa sair limpo. Este bloco apaga tudo que cria.
describe("cli: move", function()
  local A, B = "mv-a", "mv-b"

  ---Caminho vault-relative do arquivo de uma task.
  local function rel(project, id, tipo)
    if tipo then
      return string.format("projects/%s/tasks/archived/%s/%s.md", project, tipo, id)
    end
    return string.format("projects/%s/tasks/%s.md", project, id)
  end

  local function exists(r)
    return vim.fn.filereadable(vault .. "/" .. r) == 1
  end

  local function isdir(r)
    return vim.fn.isdirectory(vault .. "/" .. r) == 1
  end

  local function write(r, lines)
    vim.fn.mkdir(vim.fn.fnamemodify(vault .. "/" .. r, ":h"), "p")
    vim.fn.writefile(lines, vault .. "/" .. r)
  end

  ---Acrescenta linhas ao fim de um arquivo do vault, como quem escreve na task.
  local function append(r, lines)
    vim.fn.writefile(lines, vault .. "/" .. r, "a")
  end

  local function doctor_limpo()
    local code, out = run("doctor")
    eq(0, code, out)
  end

  it("leva a task e reescreve a linha 2 e as referências path-qualificadas, não as curtas", function()
    eq(0, (run("new-project", A)))
    eq(0, (run("new-project", B)))
    eq(0, (run("new", A, "x", "Task que anda", "--desc", "no meio do caminho")))
    eq(0, (run("set", "x", "--status", "example")))
    eq(0, (run("new", A, "vizinha", "Vizinha")))
    append(rel(A, "vizinha"), { "", "ver [[projects/mv-a/tasks/x|x]] e [[x]]" })
    -- Uma nota qualquer, fora de tasks/: o move não pode se limitar a outras tasks.
    write("mv-notas/ref.md", {
      "path [[projects/mv-a/tasks/x|x]]",
      "curto [[x]]",
      "sufixo [[projects/mv-a/tasks/x#Seção|rótulo]]",
    })
    -- daily/ são snapshots do board: ficam como estavam.
    write("daily/2026-01-01.md", { "snapshot [[projects/mv-a/tasks/x|x]]" })

    local code, out = run("move", "x", B)
    eq(0, code)
    eq(vault .. "/" .. rel(B, "x") .. "\n", out)
    falsy(exists(rel(A, "x")), "a task saiu da origem")

    -- Linha 2 no projeto novo; fase, título e descrição vão junto.
    local lines = vim.fn.readfile(vault .. "/" .. rel(B, "x"))
    eq("> [!example] Task que anda", lines[1])
    eq("> [[projects/mv-b/tasks/x|x]]", lines[2])
    eq("> _no meio do caminho_", lines[3])

    local vizinha = read(rel(A, "vizinha"))
    truthy(vizinha:find("[[projects/mv-b/tasks/x|x]]", 1, true), vizinha)
    truthy(vizinha:find("e [[x]]", 1, true), "o link curto segue valendo: " .. vizinha)
    local nota = read("mv-notas/ref.md")
    truthy(nota:find("path [[projects/mv-b/tasks/x|x]]", 1, true), nota)
    truthy(nota:find("curto [[x]]", 1, true), nota)
    truthy(nota:find("[[projects/mv-b/tasks/x#Seção|rótulo]]", 1, true), "sufixo e alias vão juntos: " .. nota)
    falsy(nota:find("mv-a", 1, true), nota)
    eq("snapshot [[projects/mv-a/tasks/x|x]]", read("daily/2026-01-01.md"), "daily/ não se reescreve")

    -- O painel e a listagem já enxergam o endereço novo.
    local board = read("CURRENT.md")
    truthy(board:find("projects/mv-b/tasks/x", 1, true), board)
    falsy(board:find("projects/mv-a/tasks/x", 1, true), board)
    local achou = false
    for _, r in ipairs(json("list", "--json")) do
      if r.id == "x" then
        achou = true
        eq("mv-b", r.project)
        eq(vault .. "/" .. rel(B, "x"), r.path)
        eq("example", r.callout)
      end
    end
    truthy(achou, "x tem de aparecer no list")

    doctor_limpo()
  end)

  it("registra a mudança no ## Histórico, em log", function()
    local body = read(rel(B, "x"))
    truthy(body:find("\n## Histórico\n", 1, true), body)
    truthy(body:match("\n%- %d%d%d%d%-%d%d%-%d%d — movida de `mv%-a` para `mv%-b`"), body)

    -- Voltar é outro evento, não a edição do anterior.
    eq(0, (run("move", "x", A)))
    eq(0, (run("move", "x", B)))
    local _, n = read(rel(B, "x")):gsub("— movida de", "")
    eq(3, n)
    doctor_limpo()
  end)

  it("task arquivada continua arquivada, no mesmo tipo e na mesma fase", function()
    for _, tipo in ipairs({ "done", "dropped", "failed" }) do
      local id = "arq-" .. tipo
      eq(0, (run("new", A, id, "Arquivada " .. tipo)))
      eq(0, (run("archive", id, tipo)))
      local antes = vim.fn.readfile(vault .. "/" .. rel(A, id, tipo))
      local alvo = "tasks/archived/" .. tipo .. "/" .. id
      write("mv-notas/" .. id .. ".md", { "ref [[projects/mv-a/" .. alvo .. "|" .. id .. "]]" })

      local code, out = run("move", id, B)
      eq(0, code, tipo)
      eq(vault .. "/" .. rel(B, id, tipo) .. "\n", out)
      falsy(exists(rel(A, id, tipo)))
      falsy(exists(rel(B, id)), "não desarquiva: " .. tipo)

      local lines = vim.fn.readfile(vault .. "/" .. rel(B, id, tipo))
      eq(antes[1], lines[1], "a fase não muda: " .. tipo)
      eq("> [[projects/mv-b/" .. alvo .. "|" .. id .. "]]", lines[2])
      local body = table.concat(lines, "\n")
      truthy(body:find("— arquivada em `archived/" .. tipo .. "`", 1, true), body)
      truthy(body:find("— movida de `mv-a` para `mv-b`", 1, true), body)
      truthy(
        body:find("arquivada em", 1, true) < body:find("movida de", 1, true),
        "o log mantém a ordem dos fatos: " .. body
      )
      eq("ref [[projects/mv-b/" .. alvo .. "|" .. id .. "]]", read("mv-notas/" .. id .. ".md"))
    end

    local arquivadas = {}
    for _, r in ipairs(json("list", "--json", "--archived")) do
      if r.id:find("^arq%-") then
        arquivadas[r.id] = r.project .. "/" .. r.archived
      end
    end
    eq({ ["arq-done"] = "mv-b/done", ["arq-dropped"] = "mv-b/dropped", ["arq-failed"] = "mv-b/failed" }, arquivadas)
    doctor_limpo()
  end)

  it("recusa projeto inexistente (sugerindo new-project), reservado e a própria pasta", function()
    eq(0, (run("new", A, "rec", "Recusada")))
    local antes = read(rel(A, "rec"))

    local code, out, err = run("move", "rec", "nao-existe")
    eq(1, code)
    eq("", out)
    truthy(err:find("freitask new-project nao-existe", 1, true), err)
    falsy(isdir("projects/nao-existe"), "projeto nunca nasce em silêncio")

    -- Pasta de projeto sem tasks/ (só spec e decisões) ainda não é projeto.
    local _
    vim.fn.mkdir(vault .. "/projects/so-spec", "p")
    code, _, err = run("move", "rec", "so-spec")
    eq(1, code)
    truthy(err:find("freitask new-project so-spec", 1, true), err)
    falsy(isdir("projects/so-spec/tasks"))

    for _, reservado in ipairs({ "daily", "templates" }) do
      code, _, err = run("move", "rec", reservado)
      eq(1, code, reservado)
      truthy(err:find("reservado", 1, true), err)
      falsy(isdir("projects/" .. reservado))
    end

    code, _, err = run("move", "rec", A)
    eq(1, code)
    truthy(err:find("já está no projeto mv-a", 1, true), err)

    eq(antes, read(rel(A, "rec")), "recusa não escreve nada")
  end)

  it("recusa id já ocupado no destino, ativo ou arquivado em qualquer tipo", function()
    -- O mesmo id em dois projetos deixa o `[[id]]` ambíguo, então a task de
    -- origem é citada pelo caminho.
    eq(0, (run("new", A, "dup", "Origem")))
    eq(0, (run("new", B, "dup", "Destino ativo")))
    local code, _, err = run("move", vault .. "/" .. rel(A, "dup"), B)
    eq(1, code)
    truthy(err:find("já existe uma task com esse id", 1, true), err)
    truthy(exists(rel(A, "dup")), "continua na origem")
    truthy(read(rel(B, "dup")):find("Destino ativo", 1, true), "o do destino não foi sobrescrito")

    for _, tipo in ipairs({ "done", "dropped", "failed" }) do
      local id = "dup-" .. tipo
      -- O do destino é arquivado ANTES de o id existir na origem, senão o id
      -- curto já seria ambíguo para o próprio `archive`.
      eq(0, (run("new", B, id, "Destino arquivado")))
      eq(0, (run("archive", id, tipo)))
      eq(0, (run("new", A, id, "Origem")))
      code, _, err = run("move", vault .. "/" .. rel(A, id), B)
      eq(1, code, tipo)
      truthy(err:find("já existe uma task com esse id", 1, true), err)
      truthy(exists(rel(A, id)), "continua na origem: " .. tipo)
      truthy(exists(rel(B, id, tipo)), "o arquivado do destino segue lá: " .. tipo)
    end

    -- A origem arquivada não ajuda: o id ocupado é o do destino.
    eq(0, (run("new", A, "dup-arq", "Origem")))
    eq(0, (run("archive", "dup-arq", "dropped")))
    eq(0, (run("new", B, "dup-arq", "Destino ativo")))
    code = run("move", vault .. "/" .. rel(A, "dup-arq", "dropped"), B)
    eq(1, code)
    truthy(exists(rel(A, "dup-arq", "dropped")))
  end)

  it("recusa buffer da task com alterações não salvas", function()
    -- Sob `nvim -l` a CLI não tem buffer carregado, então a guarda só dispara
    -- quando o motor é chamado de dentro de um Neovim com a task aberta.
    local script = home .. "/buffer-sujo.lua"
    vim.fn.writefile({
      'local T = require("freitask")',
      "local path = _G.arg[1]",
      'vim.cmd("edit " .. vim.fn.fnameescape(path))',
      'vim.api.nvim_buf_set_lines(0, -1, -1, false, { "não salvo" })',
      "local new, err = T.move_to_project(path, _G.arg[2])",
      'io.stdout:write(vim.json.encode({ new = new or vim.NIL, err = err or vim.NIL }) .. "\\n")',
      "io.stdout:flush()",
    }, script)
    local alvo = vault .. "/" .. rel(A, "rec")
    local antes = read(rel(A, "rec"))
    local r = vim
      .system({ "nvim", "--clean", "--cmd", "set runtimepath+=" .. repo, "-l", script, alvo, B }, {
        env = { HOME = home },
        text = true,
      })
      :wait()
    eq(0, r.code, r.stderr)
    local res = vim.json.decode(r.stdout)
    eq(vim.NIL, res.new)
    truthy(res.err:find("salve", 1, true), vim.inspect(res))
    truthy(exists(rel(A, "rec")), "continua na origem")
    falsy(exists(rel(B, "rec")))
    eq(antes, read(rel(A, "rec")))
  end)

  it("uso inválido sai 2, task inexistente sai 1, e o help lista o move", function()
    eq(2, (run("move")))
    eq(2, (run("move", "rec")))
    eq(1, (run("move", "nao-existe", B)))
    local code, out = run("help")
    eq(0, code)
    truthy(out:find("freitask move", 1, true), out)
  end)

  -- Tudo que este bloco criou sai daqui: os blocos de baixo contam com o vault
  -- como estava (um `list --archived` que espera a primeira linha, o `doctor`).
  vim.fn.delete(vault .. "/projects/" .. A, "rf")
  vim.fn.delete(vault .. "/projects/" .. B, "rf")
  vim.fn.delete(vault .. "/projects/so-spec", "rf")
  vim.fn.delete(vault .. "/mv-notas", "rf")
  vim.fn.delete(vault .. "/daily/2026-01-01.md")
  run("rebuild")
end)

describe("cli: arquivar, renomear, apagar", function()
  it("arquivada em done não troca de fase por set", function()
    eq(0, (run("archive", "t1", "done")))
    eq(1, (run("set", "t1", "--status", "todo")))
    local rows = json("list", "--json", "--archived")
    eq("done", rows[1].archived)
    eq("done", rows[1].callout)
    truthy(read("projects/meu-projeto/tasks/archived/done/t1.md"):find("## Histórico", 1, true))
  end)

  it("rename reescreve os dois formatos de link", function()
    eq(0, (run("unarchive", "t1")))
    eq(0, (run("rename", "t1", "t2")))
    local n = read("n.md")
    truthy(n:find("[[t2]]", 1, true), n)
    truthy(n:find("[[projects/meu-projeto/tasks/t2|t2]]", 1, true), n)
  end)

  it("rename recusa id fora de kebab e id já ocupado, até em archived/", function()
    -- Regressão: `rename t "Id Ruim"` criava `Id Ruim.md` — um arquivo e uma
    -- branch que ninguém digita.
    eq(1, (run("rename", "t2", "Id Ruim")))
    eq(0, (run("new", "meu-projeto", "t3", "Três")))
    eq(0, (run("archive", "t3", "dropped")))
    eq(1, (run("rename", "t2", "t3")))
    eq(1, vim.fn.filereadable(vault .. "/projects/meu-projeto/tasks/t2.md"))
  end)

  it("delete apaga e deixa o link pendurado para o doctor", function()
    eq(0, (run("delete", "t2")))
    eq(0, vim.fn.filereadable(vault .. "/projects/meu-projeto/tasks/t2.md"))
    local code, out = run("doctor")
    eq(1, code)
    truthy(out:find("link-pendurado", 1, true), out)
    eq(1, (run("delete", "t2")))
  end)
end)

vim.fn.delete(home, "rf")
