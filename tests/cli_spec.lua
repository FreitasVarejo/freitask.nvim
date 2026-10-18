-- Testes de ponta a ponta da CLI, num vault descartável.
--
-- Cobre o que freitask_spec.lua não alcança por ser só funções puras: as
-- operações que escrevem no disco (criar, trocar fase, renomear, arquivar,
-- apagar) e o contrato JSON que a TUI e os agentes leem. Cada chamada é um
-- processo `nvim -l` de verdade, com HOME apontando para um diretório
-- temporário — o módulo deriva o vault de `~`, então é só isso que precisa
-- mudar para nunca tocar o vault real.

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
