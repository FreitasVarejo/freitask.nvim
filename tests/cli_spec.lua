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
    eq(3, r.status_num)
    eq("example", r.callout)
    eq("Um novo", r.title)
    eq("", r.desc)
    eq("dropped", r.suggested_archive)
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
