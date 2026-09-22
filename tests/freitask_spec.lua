-- Testes das funções PURAS do freitask.
--
-- Só entra aqui o que não toca disco: parser/serializer do bloco, derivação de
-- id, decomposição de caminho, reescrita de wikilink, manipulação de linhas.
-- É a fatia que erra em silêncio — um link reescrito errado só se manifesta
-- semanas depois, quando você clica nele — e é a que o refactor vai mover de
-- arquivo, então precisa de rede antes.
--
-- Os testes fixam o status.json DEFAULT em vez de deixar o módulo ler o do
-- vault: senão a suíte passaria ou falharia conforme o estado da máquina.
--
-- Cada função é requerida do SEU módulo. Antes da divisão elas eram locais de
-- um arquivo de 2400 linhas e o spec as alcançava por uma tabela `__test`
-- exportada só para isso; agora são API do módulo onde moram.

local C = require("freitask.config")
local md = require("freitask.md")
local model = require("freitask.model")
local path = require("freitask.path")
local status = require("freitask.status")
local links = require("freitask.links")
local meta = require("freitask.meta")
local task = require("freitask.task")

status.status = vim.json.decode(C.DEFAULT_STATUS_JSON)

---Impede que qualquer teste caia no caminho de I/O que releria o status.json
---do vault e reintroduziria a dependência de máquina.
status.load_status = function() end

local VAULT = vim.fn.expand("~/ObsidianVault")

describe("kebab", function()
  it("dobra acentos multibyte", function()
    eq("configuracao-da-api", path.kebab("Configuração da API"))
    eq("avida-tarefa", path.kebab("Ávida Tarefa"))
  end)

  it("descarta pontuação e colapsa separadores", function()
    eq("fix-the-thing", path.kebab("Fix: the thing!"))
    eq("foo-bar", path.kebab("  foo   bar  "))
    eq("foo-bar", path.kebab("--foo--bar--"))
  end)

  it("converte underscore em dash", function()
    -- Regressão: `%w` do Lua não inclui `_`, então a classe permitida precisa
    -- listá-lo explicitamente — senão ele é apagado antes da regra que o
    -- converteria, e "foo_bar" sai "foobar".
    eq("foo-bar", path.kebab("foo_bar"))
    eq("foo-bar", path.kebab("foo __ bar"))
  end)

  it("devolve string vazia quando não sobra nada", function()
    eq("", path.kebab("!!!"))
    eq("", path.kebab(""))
  end)
end)

describe("split_task_path", function()
  it("decompõe uma task ativa", function()
    local p, id, arch = path.split_task_path(VAULT .. "/projects/bjju-web/tasks/fazer-x.md")
    eq("bjju-web", p)
    eq("fazer-x", id)
    eq(nil, arch)
  end)

  it("decompõe uma task arquivada", function()
    local p, id, arch = path.split_task_path(VAULT .. "/projects/bjju-web/tasks/archived/done/fazer-x.md")
    eq("bjju-web", p)
    eq("fazer-x", id)
    eq("done", arch)
  end)

  it("recusa tipo de arquivamento desconhecido", function()
    falsy(path.split_task_path(VAULT .. "/projects/bjju-web/tasks/archived/talvez/fazer-x.md"))
  end)

  it("recusa diretórios reservados", function()
    falsy(path.split_task_path(VAULT .. "/projects/daily/tasks/2026-08-09.md"))
    falsy(path.split_task_path(VAULT .. "/projects/templates/tasks/task.md"))
  end)

  it("recusa arquivo de conflito do Syncthing", function()
    falsy(path.split_task_path(VAULT .. "/projects/bjju-web/tasks/x.sync-conflict-20260809-123456-ABCDEFG.md"))
  end)

  it("recusa subdiretório que não é archived/<tipo>", function()
    falsy(path.split_task_path(VAULT .. "/projects/bjju-web/tasks/notas/x.md"))
  end)
end)

describe("status_meta", function()
  it("resolve um status válido", function()
    eq("todo", status.status_meta(1).callout)
    eq("Não iniciada", status.status_meta(1).title)
  end)

  it("cai no sentinela para 0 e nil", function()
    eq("invalid", status.status_meta(0).callout)
    eq("invalid", status.status_meta(nil).callout)
    eq("invalid", status.status_meta(999).callout)
  end)
end)

describe("is_status_text", function()
  it("reconhece um título vigente do status.json", function()
    local ok, num = model.is_status_text("1 - Não iniciada")
    truthy(ok)
    eq(1, num)
  end)

  it("reconhece o vocabulário legado que já não existe", function()
    local ok, num = model.is_status_text("1 - Backlog")
    truthy(ok)
    eq(1, num)
    local ok2, num2 = model.is_status_text("3 - Blocked")
    truthy(ok2)
    eq(5, num2, "Blocked legado mapeia para o callout warning de hoje")
  end)

  it("usa a chave ATUAL do título, não o dígito escrito na linha", function()
    -- "99 - Arquivada" tem dígito inválido, mas o título casa a entrada 7.
    local ok, num = model.is_status_text("99 - Arquivada")
    truthy(ok)
    eq(7, num)
  end)

  it("não come uma nota que só parece status", function()
    falsy(model.is_status_text("3 - comprar leite"))
    falsy(model.is_status_text("nota qualquer"))
  end)
end)

describe("parse_block", function()
  it("lê o formato atual completo", function()
    local m = model.parse_block({
      "> [!todo] Título da task",
      "> [[projects/bjju-web/tasks/fazer-x|fazer-x]]",
      "> _Em análise do Fábio_",
      "> impedimento: falta VPN",
    })
    eq(1, m.status_num)
    eq("todo", m.raw_callout)
    eq("Título da task", m.title)
    eq("fazer-x", m.id)
    eq("Em análise do Fábio", m.desc)
    eq({ "impedimento: falta VPN" }, m.extras)
  end)

  it("omite a descrição quando a linha 3 não está em itálico", function()
    local m = model.parse_block({
      "> [!todo] T",
      "> [[projects/p/tasks/x|x]]",
      "> uma nota qualquer",
    })
    eq("", m.desc)
    eq({ "uma nota qualquer" }, m.extras)
  end)

  it("não promove itálico a descrição depois de já haver nota", function()
    local m = model.parse_block({
      "> [!todo] T",
      "> [[projects/p/tasks/x|x]]",
      "> nota primeiro",
      "> _isso continua nota_",
    })
    eq("", m.desc)
    eq({ "nota primeiro", "_isso continua nota_" }, m.extras)
  end)

  it("dá status 0 e preserva o tipo digitado quando o callout é desconhecido", function()
    local m = model.parse_block({
      "> [!questão] Título preservado",
      "> [[projects/p/tasks/x|x]]",
    })
    eq(0, m.status_num)
    eq("questão", m.raw_callout)
    eq("Título preservado", m.title, "o prefixo [!...] precisa sair mesmo com acento no tipo")
  end)

  it("lê o formato legado (negrito + [[id]] curto + status-texto + Branch:)", function()
    local m = model.parse_block({
      "> **Minha task**",
      "> [[minha-task]]",
      "> 1 - Backlog",
      "> Branch: minha-task",
      "> alguma nota",
    })
    eq(1, m.status_num)
    eq("Minha task", m.title)
    eq("minha-task", m.id)
    eq({ "alguma nota" }, m.extras, "status-texto e Branch: são consumidos, não viram nota")
  end)

  it("extrai o id do alvo do wikilink, não do alias", function()
    local m = model.parse_block({
      "> [!todo] T",
      "> [[projects/p/tasks/archived/done/x|rótulo qualquer]]",
    })
    eq("x", m.id)
  end)
end)

describe("serialize_block", function()
  it("emite o link path-qualified quando há project", function()
    eq({
      "> [!todo] Título",
      "> [[projects/p/tasks/x|x]]",
    }, model.serialize_block({ status_num = 1, title = "Título", id = "x", project = "p" }))
  end)

  it("aponta para archived/<tipo> quando a task está arquivada", function()
    eq({
      "> [!done] Título",
      "> [[projects/p/tasks/archived/done/x|x]]",
    }, model.serialize_block({ status_num = 7, title = "Título", id = "x", project = "p", archived = "done" }))
  end)

  it("omite a linha de descrição quando ela está vazia", function()
    local out = model.serialize_block({ status_num = 1, title = "T", id = "x", project = "p", desc = "  " })
    eq(2, #out, "um `>` vazio no meio do bloco promoveria a nota seguinte a descrição")
  end)

  it("preserva verbatim o tipo inválido do status 0", function()
    local out = model.serialize_block({ status_num = 0, raw_callout = "questão", title = "T", id = "x", project = "p" })
    eq("> [!questão] T", out[1])
  end)

  it("cai em `invalid` quando o status 0 não tem tipo digitado", function()
    local out = model.serialize_block({ status_num = 0, raw_callout = "", title = "T", id = "x", project = "p" })
    eq("> [!invalid] T", out[1])
  end)

  it("apara linhas em branco no fim das notas", function()
    local out = model.serialize_block({
      status_num = 1,
      title = "T",
      id = "x",
      project = "p",
      extras = { "nota", "", "  " },
    })
    eq({ "> [!todo] T", "> [[projects/p/tasks/x|x]]", "> nota" }, out)
  end)
end)

describe("round-trip parse/serialize", function()
  it("é idempotente para um bloco canônico", function()
    local original = {
      "> [!example] Título da task",
      "> [[projects/bjju-web/tasks/fazer-x|fazer-x]]",
      "> _aguardando revisão_",
      "> impedimento: VPN",
      "> segunda nota",
    }
    local m = model.parse_block(original)
    m.project = "bjju-web"
    eq(original, model.serialize_block(m))
  end)

  it("normaliza o formato legado para o atual", function()
    local m = model.parse_block({
      "> **Minha task**",
      "> [[minha-task]]",
      "> 1 - Backlog",
      "> Branch: minha-task",
    })
    m.project = "p"
    eq({ "> [!todo] Minha task", "> [[projects/p/tasks/minha-task|minha-task]]" }, model.serialize_block(m))
  end)

  it("sobrevive ao round-trip com status 0", function()
    local blk = { "> [!questão] T", "> [[projects/p/tasks/x|x]]" }
    local m = model.parse_block(blk)
    m.project = "p"
    local out = model.serialize_block(m)
    eq(blk, out)
    eq(0, model.parse_block(out).status_num, "o erro precisa reparsear como 0, não sumir")
  end)
end)

describe("first_block_range", function()
  it("acha o bloco no topo", function()
    local s, e = md.first_block_range({ "> [!todo] T", "> [[x]]", "", "corpo" })
    eq(1, s)
    eq(2, e)
  end)

  it("pula frontmatter YAML e linhas em branco", function()
    local s, e = md.first_block_range({ "---", "id: x", "---", "", "> [!todo] T", "> [[x]]", "", "corpo" })
    eq(5, s)
    eq(6, e)
  end)

  it("devolve nil quando há conteúdo não-quote antes do bloco", function()
    falsy(md.first_block_range({ "# Título", "", "> [!todo] T" }))
  end)

  it("devolve nil quando não há bloco nenhum", function()
    falsy(md.first_block_range({ "corpo", "mais corpo" }))
    falsy(md.first_block_range({}))
  end)
end)

describe("block_around", function()
  it("expande para as bordas do bloco que contém a linha", function()
    local lines = { "texto", "> a", "> b", "> c", "texto" }
    local s, e = md.block_around(lines, 3)
    eq(2, s)
    eq(4, e)
  end)

  it("devolve nil fora de um blockquote", function()
    falsy(md.block_around({ "texto", "> a" }, 1))
  end)
end)

describe("splice", function()
  it("substitui a faixa preservando o entorno", function()
    eq({ "a", "X", "Y", "d" }, md.splice({ "a", "b", "c", "d" }, 2, 3, { "X", "Y" }))
  end)

  it("aceita substituição vazia", function()
    eq({ "a", "d" }, md.splice({ "a", "b", "c", "d" }, 2, 3, {}))
  end)
end)

describe("rewrite_link", function()
  local moved_old = { id = "x", full = "projects/p/tasks/x" }
  local moved_new = { id = "x", full = "projects/p/tasks/archived/done/x" }
  local renamed_old = { id = "a", full = "projects/p/tasks/a" }
  local renamed_new = { id = "b", full = "projects/p/tasks/b" }

  it("ao mover, reescreve só o link path-qualified", function()
    eq("projects/p/tasks/archived/done/x|x", links.rewrite_link("projects/p/tasks/x|x", moved_old, moved_new))
    eq(nil, links.rewrite_link("x", moved_old, moved_new), "o Obsidian resolve o link curto por basename")
  end)

  it("ao renomear, reescreve as duas formas", function()
    eq("b", links.rewrite_link("a", renamed_old, renamed_new))
    eq("projects/p/tasks/b|b", links.rewrite_link("projects/p/tasks/a|a", renamed_old, renamed_new))
  end)

  it("preserva um alias que não era o id", function()
    eq("projects/p/tasks/b|Meu Título", links.rewrite_link("projects/p/tasks/a|Meu Título", renamed_old, renamed_new))
  end)

  it("preserva sufixo de heading e de bloco", function()
    eq("projects/p/tasks/b#Seção", links.rewrite_link("projects/p/tasks/a#Seção", renamed_old, renamed_new))
    eq("projects/p/tasks/b^bloco|b", links.rewrite_link("projects/p/tasks/a^bloco|a", renamed_old, renamed_new))
  end)

  it("tolera o sufixo .md no alvo", function()
    eq("b", links.rewrite_link("a.md", renamed_old, renamed_new))
  end)

  it("devolve nil para link que aponta para outra coisa", function()
    eq(nil, links.rewrite_link("outra-nota", renamed_old, renamed_new))
    eq(nil, links.rewrite_link("projects/outro/tasks/a|a", renamed_old, renamed_new))
  end)
end)

describe("append_history", function()
  local today = os.date("%Y-%m-%d")

  it("cria a seção quando ela não existe", function()
    local lines = { "# Task", "", "corpo", "", "" }
    md.append_history(lines, "done")
    eq({
      "# Task",
      "",
      "corpo",
      "",
      "## Histórico",
      "",
      "- " .. today .. " — arquivada em `archived/done` (feito)",
    }, lines)
  end)

  it("acrescenta ao FIM da seção existente, não do arquivo", function()
    local lines = {
      "## Histórico",
      "",
      "- 2026-01-01 — arquivada em `archived/done` (feito)",
      "",
      "## Outra seção",
      "conteúdo",
    }
    md.append_history(lines, nil)
    eq("- " .. today .. " — desarquivada, de volta ao board", lines[4])
    eq("", lines[5], "a linha em branco de separação da seção é preservada")
    eq("## Outra seção", lines[6], "a entrada nova não pode vazar para depois da próxima seção")
  end)

  it("é um log: não sobrescreve a entrada anterior", function()
    local lines = { "## Histórico", "", "- 2026-01-01 — arquivada em `archived/done` (feito)" }
    md.append_history(lines, "dropped")
    eq(4, #lines)
  end)
end)

describe("update_frontmatter_key", function()
  it("atualiza a chave quando ela já existe", function()
    local lines = { "---", "id: velho", "tags: []", "---", "corpo" }
    truthy(md.update_frontmatter_key(lines, "id", "novo"))
    eq("id: novo", lines[2])
  end)

  it("não cria frontmatter nem chave ausente", function()
    local sem = { "corpo" }
    falsy(md.update_frontmatter_key(sem, "id", "novo"))
    eq({ "corpo" }, sem)

    local sem_chave = { "---", "tags: []", "---" }
    falsy(md.update_frontmatter_key(sem_chave, "id", "novo"))
    eq({ "---", "tags: []", "---" }, sem_chave)
  end)

  it("não reporta mudança quando o valor já está certo", function()
    falsy(md.update_frontmatter_key({ "---", "id: x", "---" }, "id", "x"))
  end)
end)

describe("suggest_archive_type", function()
  it("deriva done dos callouts de conclusão", function()
    eq("done", path.suggest_archive_type(7)) -- done
    eq("done", path.suggest_archive_type(6)) -- check
  end)

  it("não deriva failed de fase nenhuma: `failed` se escolhe à mão", function()
    -- Nenhuma das seis fases significa "falhou" — `warning` é BLOQUEADA, que é
    -- outra coisa: quem travou não fracassou. Com FAILED_CALLOUTS vazio, o
    -- prompt passa a sugerir `dropped` e quem quiser `failed` digita. O teste
    -- fica como registro da escolha: se um dia uma fase de falha entrar no
    -- vocabulário, é ele que falha primeiro e cobra a atualização.
    for _, num in ipairs({ 1, 2, 3, 4, 5 }) do
      eq("dropped", path.suggest_archive_type(num))
    end
  end)

  it("cai em dropped para o resto, inclusive status 0", function()
    eq("dropped", path.suggest_archive_type(1))
    eq("dropped", path.suggest_archive_type(0))
    eq("dropped", path.suggest_archive_type(nil))
  end)
end)

describe("is_archived", function()
  it("lê o estado do CAMINHO", function()
    truthy(path.is_archived(VAULT .. "/projects/p/tasks/archived/done/x.md"))
    falsy(path.is_archived(VAULT .. "/projects/p/tasks/x.md"))
  end)
end)

describe("frontmatter", function()
  it("lê pares no primeiro nível e devolve a faixa", function()
    local map, a, b = md.frontmatter({ "---", "dono: fedora/nvim", "dominio: schema", "---", "", "> [!todo] x" })
    eq("fedora/nvim", map.dono)
    eq("schema", map.dominio)
    eq(1, a)
    eq(4, b)
  end)

  it("devolve vazio sem frontmatter e sem fechamento", function()
    eq({}, (md.frontmatter({ "> [!todo] x" })))
    eq({}, (md.frontmatter({ "---", "dono: x" })))
  end)

  it("tira as aspas do escalar", function()
    local map = md.frontmatter({ "---", 'dominio: "a: b"', "---" })
    eq("a: b", map.dominio)
  end)
end)

describe("set_frontmatter", function()
  it("cria o bloco quando não existe, com linha em branco antes do conteúdo", function()
    local lines = { "> [!todo] x" }
    truthy(md.set_frontmatter(lines, { dono = "fedora/nvim" }))
    eq({ "---", "dono: fedora/nvim", "---", "", "> [!todo] x" }, lines)
  end)

  it("não cria bloco quando só há remoções", function()
    local lines = { "> [!todo] x" }
    falsy(md.set_frontmatter(lines, { dono = false }))
    eq({ "> [!todo] x" }, lines)
  end)

  it("preserva chaves alheias e a ordem ao atualizar", function()
    local lines = { "---", "id: x", "dono: a", "tags: [p]", "---", "" }
    truthy(md.set_frontmatter(lines, { dono = "b" }))
    eq({ "---", "id: x", "dono: b", "tags: [p]", "---", "" }, lines)
  end)

  it("acrescenta chave nova no fim do bloco", function()
    local lines = { "---", "id: x", "---" }
    truthy(md.set_frontmatter(lines, { dominio = "schema" }))
    eq({ "---", "id: x", "dominio: schema", "---" }, lines)
  end)

  it("remove com false e apaga o bloco inteiro quando ele esvazia", function()
    local lines = { "---", "dono: a", "---", "", "> [!todo] x" }
    truthy(md.set_frontmatter(lines, { dono = false }))
    eq({ "> [!todo] x" }, lines)
  end)

  it("põe aspas só quando o escalar cru não sobreviveria", function()
    local lines = { "---", "id: x", "---" }
    md.set_frontmatter(lines, { a = "a: b", b = "simples com espaço", c = "[nao-lista]" })
    eq('a: "a: b"', lines[3])
    eq("b: simples com espaço", lines[4])
    eq('c: "[nao-lista]"', lines[5])
  end)

  it("sobrevive ao round-trip do que escreveu", function()
    local lines = { "> [!todo] x" }
    md.set_frontmatter(lines, { dominio = "a: b", dono = "fedora/claude-code" })
    local map = md.frontmatter(lines)
    eq("a: b", map.dominio)
    eq("fedora/claude-code", map.dono)
  end)
end)

describe("meta", function()
  it("lê só as três chaves do eixo, ignorando o resto", function()
    local mt = meta.read({ "---", "id: x", "dono: fedora/nvim", "desde: 2026-09-22T10:00:00-03:00", "---" })
    eq("fedora/nvim", mt.dono)
    eq("2026-09-22T10:00:00-03:00", mt.desde)
    eq(nil, mt.dominio)
    eq(nil, mt.id)
  end)

  it("trata campo vazio como ausente", function()
    eq(nil, meta.read({ "---", "dono:", "---" }).dono)
  end)

  it("escreve o estado INTEIRO: campo ausente é removido", function()
    local lines = { "---", "dono: a", "dominio: d", "---", "", "> x" }
    meta.write(lines, { dono = "b" })
    local mt = meta.read(lines)
    eq("b", mt.dono)
    eq(nil, mt.dominio, "dominio some porque não veio na tabela")
  end)

  it("converte ISO-8601 com offset para o mesmo instante", function()
    eq(meta.epoch("2026-09-22T00:00:00Z"), meta.epoch("2026-09-21T21:00:00-03:00"))
    eq(meta.epoch("2026-09-22T00:00:00Z"), meta.epoch("2026-09-22T03:00:00+03:00"))
    eq(3600, meta.epoch("2026-09-22T01:00:00Z") - meta.epoch("2026-09-22T00:00:00Z"))
  end)

  it("devolve nil para carimbo ilegível", function()
    eq(nil, meta.epoch("ontem"))
    eq(nil, meta.epoch(nil))
  end)

  it("emite carimbo ISO com offset de dois-pontos", function()
    truthy(meta.now():match("^%d%d%d%d%-%d%d%-%d%dT%d%d:%d%d:%d%d[+-]%d%d:%d%d$"))
  end)

  it("só acusa fantasma quando há garra velha", function()
    local now = meta.epoch("2026-09-22T12:00:00Z")
    falsy(meta.stale({}, now), "sem dono não há garra")
    falsy(meta.stale({ dono = "a", desde = "2026-09-22T11:00:00Z" }, now), "1h é garra viva")
    falsy(meta.stale({ dono = "a", desde = "2026-09-21T13:00:00Z" }, now), "23h ainda vale")
    truthy(meta.stale({ dono = "a", desde = "2026-09-21T11:00:00Z" }, now), "25h é fantasma")
    truthy(meta.stale({ dono = "a" }, now), "garra sem carimbo nunca expiraria sozinha")
  end)

  it("identifica o agente como <maquina>/<ferramenta>", function()
    truthy(meta.whoami("claude-code"):match("^[^/]+/claude%-code$"))
  end)
end)

describe("validate_new", function()
  it("recusa projeto reservado", function()
    local ok, err = task.validate_new("daily", "x")
    falsy(ok)
    truthy(err:match("reservado"))
    falsy((task.validate_new("templates", "x")))
  end)

  it("recusa projeto vazio ou com barra", function()
    falsy((task.validate_new("", "x")))
    falsy((task.validate_new("a/b", "x")))
  end)

  it("recusa id que não é kebab, sem corrigir em silêncio", function()
    -- O id é o nome do ARQUIVO e o da BRANCH git. Gravar "minha-task" quando
    -- pediram "Minha Task" faria a branch nascer com outro nome que o arquivo.
    local ok, err = task.validate_new("dotfiles", "meu id")
    falsy(ok)
    truthy(err:match("kebab"))
    truthy(err:match("meu%-id"), "a mensagem sugere o id corrigido")
    falsy((task.validate_new("dotfiles", "Maiúscula")))
    falsy((task.validate_new("dotfiles", "")))
  end)

  it("aceita kebab com dígito e acento já dobrado", function()
    truthy((task.validate_new("dotfiles", "vault-lint-2")))
    truthy((task.validate_new("dotfiles", path.kebab("Configuração da API"))))
  end)
end)

describe("template", function()
  it("é o bloco e uma linha em branco, sem estrutura fixa", function()
    -- Regressão: emitia também `## Notas Soltas` e `### [<projeto>]`, eco do
    -- formato do CURRENT.md vazado para dentro da task. Nenhuma task do vault
    -- tem essas seções.
    local out = task.template({
      status_num = 1,
      title = "Exemplo",
      id = "exemplo",
      desc = "",
      extras = {},
      project = "dotfiles",
    })
    eq({
      "> [!todo] Exemplo",
      "> [[projects/dotfiles/tasks/exemplo|exemplo]]",
      "",
    }, out)
  end)

  it("volta pelo parser com o mesmo modelo", function()
    local out = task.template({
      status_num = 1,
      title = "Exemplo",
      id = "exemplo",
      desc = "estado",
      extras = {},
      project = "dotfiles",
    })
    local back = model.parse_block({ out[1], out[2], out[3] })
    eq("Exemplo", back.title)
    eq("exemplo", back.id)
    eq("estado", back.desc)
    eq(1, back.status_num)
  end)
end)

describe("status_after_move", function()
  local DONE, CHECK, TODO = 7, 6, 1

  it("arquivar como done grava a fase done", function()
    -- Regressão: `archive <id> done` movia o arquivo e deixava o callout, e o
    -- doctor acusava callout-vs-pasta logo depois — a CLI produzindo o estado
    -- que ela mesma reclama.
    eq(DONE, task.status_after_move(TODO, "done"))
    eq(DONE, task.status_after_move(CHECK, "done"))
  end)

  it("dropped e failed não mexem na fase", function()
    -- Não têm callout: abandonar não é uma fase do trabalho, e o doctor só
    -- cobra coerência em archived/done/.
    eq(TODO, task.status_after_move(TODO, "dropped"))
    eq(TODO, task.status_after_move(TODO, "failed"))
    eq(DONE, task.status_after_move(DONE, "dropped"))
  end)

  it("desarquivar devolve done como check", function()
    -- `done` numa task ativa poria "Arquivada" no board de algo que não está.
    eq(CHECK, task.status_after_move(DONE, nil))
  end)

  it("desarquivar não mexe em fase que não seja done", function()
    eq(TODO, task.status_after_move(TODO, nil))
    eq(CHECK, task.status_after_move(CHECK, nil))
  end)
end)
