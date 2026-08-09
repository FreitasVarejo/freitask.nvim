-- Testes das funções PURAS do util.freitask.
--
-- Só entra aqui o que não toca disco: parser/serializer do bloco, derivação de
-- id, decomposição de caminho, reescrita de wikilink, manipulação de linhas.
-- É a fatia que erra em silêncio — um link reescrito errado só se manifesta
-- semanas depois, quando você clica nele — e é a que o refactor vai mover de
-- arquivo, então precisa de rede antes.
--
-- Os testes fixam o status.json DEFAULT em vez de deixar o módulo ler o do
-- vault: senão a suíte passaria ou falharia conforme o estado da máquina.

local T = require("util.freitask")
local X = T.__test

T.status = vim.json.decode(X.DEFAULT_STATUS_JSON)

---Impede que qualquer teste caia no caminho de I/O que releria o status.json
---do vault e reintroduziria a dependência de máquina.
T.load_status = function() end

local VAULT = vim.fn.expand("~/ObsidianVault")

describe("kebab", function()
  it("dobra acentos multibyte", function()
    eq("configuracao-da-api", X.kebab("Configuração da API"))
    eq("avida-tarefa", X.kebab("Ávida Tarefa"))
  end)

  it("descarta pontuação e colapsa separadores", function()
    eq("fix-the-thing", X.kebab("Fix: the thing!"))
    eq("foo-bar", X.kebab("  foo   bar  "))
    eq("foo-bar", X.kebab("--foo--bar--"))
  end)

  it("come o underscore em vez de virar dash", function()
    -- CARACTERIZAÇÃO, não aprovação: `%w` do Lua não inclui `_`, então o
    -- primeiro gsub o remove e a regra `[%s_]+ → -` nunca chega a ver um.
    -- "foo_bar" deveria virar "foo-bar". Corrigido no passo seguinte.
    eq("foobar", X.kebab("foo_bar"))
  end)

  it("devolve string vazia quando não sobra nada", function()
    eq("", X.kebab("!!!"))
    eq("", X.kebab(""))
  end)
end)

describe("split_task_path", function()
  it("decompõe uma task ativa", function()
    local p, id, arch = T.split_task_path(VAULT .. "/tasks/bjju-web/fazer-x.md")
    eq("bjju-web", p)
    eq("fazer-x", id)
    eq(nil, arch)
  end)

  it("decompõe uma task arquivada", function()
    local p, id, arch = T.split_task_path(VAULT .. "/tasks/bjju-web/archived/done/fazer-x.md")
    eq("bjju-web", p)
    eq("fazer-x", id)
    eq("done", arch)
  end)

  it("recusa tipo de arquivamento desconhecido", function()
    falsy(T.split_task_path(VAULT .. "/tasks/bjju-web/archived/talvez/fazer-x.md"))
  end)

  it("recusa diretórios reservados", function()
    falsy(T.split_task_path(VAULT .. "/tasks/daily/2026-08-09.md"))
    falsy(T.split_task_path(VAULT .. "/tasks/templates/task.md"))
  end)

  it("recusa arquivo de conflito do Syncthing", function()
    falsy(T.split_task_path(VAULT .. "/tasks/bjju-web/x.sync-conflict-20260809-123456-ABCDEFG.md"))
  end)

  it("recusa subdiretório que não é archived/<tipo>", function()
    falsy(T.split_task_path(VAULT .. "/tasks/bjju-web/notas/x.md"))
  end)
end)

describe("status_meta", function()
  it("resolve um status válido", function()
    eq("todo", T.status_meta(1).callout)
    eq("Todo", T.status_meta(1).title)
  end)

  it("cai no sentinela para 0 e nil", function()
    eq("invalid", T.status_meta(0).callout)
    eq("invalid", T.status_meta(nil).callout)
    eq("invalid", T.status_meta(999).callout)
  end)
end)

describe("is_status_text", function()
  it("reconhece um título vigente do status.json", function()
    local ok, num = X.is_status_text("1 - Todo")
    truthy(ok)
    eq(1, num)
  end)

  it("reconhece o vocabulário legado que já não existe", function()
    local ok, num = X.is_status_text("1 - Backlog")
    truthy(ok)
    eq(1, num)
    local ok2, num2 = X.is_status_text("3 - Blocked")
    truthy(ok2)
    eq(13, num2, "Blocked legado mapeia para o callout warning de hoje")
  end)

  it("usa a chave ATUAL do título, não o dígito escrito na linha", function()
    -- "99 - Done" tem dígito inválido, mas o título casa a entrada 25.
    local ok, num = X.is_status_text("99 - Done")
    truthy(ok)
    eq(25, num)
  end)

  it("não come uma nota que só parece status", function()
    falsy(X.is_status_text("3 - comprar leite"))
    falsy(X.is_status_text("nota qualquer"))
  end)
end)

describe("parse_block", function()
  it("lê o formato atual completo", function()
    local m = T.parse_block({
      "> [!todo] Título da task",
      "> [[tasks/bjju-web/fazer-x|fazer-x]]",
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
    local m = T.parse_block({
      "> [!todo] T",
      "> [[tasks/p/x|x]]",
      "> uma nota qualquer",
    })
    eq("", m.desc)
    eq({ "uma nota qualquer" }, m.extras)
  end)

  it("não promove itálico a descrição depois de já haver nota", function()
    local m = T.parse_block({
      "> [!todo] T",
      "> [[tasks/p/x|x]]",
      "> nota primeiro",
      "> _isso continua nota_",
    })
    eq("", m.desc)
    eq({ "nota primeiro", "_isso continua nota_" }, m.extras)
  end)

  it("dá status 0 e preserva o tipo digitado quando o callout é desconhecido", function()
    local m = T.parse_block({
      "> [!questão] Título preservado",
      "> [[tasks/p/x|x]]",
    })
    eq(0, m.status_num)
    eq("questão", m.raw_callout)
    eq("Título preservado", m.title, "o prefixo [!...] precisa sair mesmo com acento no tipo")
  end)

  it("lê o formato legado (negrito + [[id]] curto + status-texto + Branch:)", function()
    local m = T.parse_block({
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
    local m = T.parse_block({
      "> [!todo] T",
      "> [[tasks/p/archived/done/x|rótulo qualquer]]",
    })
    eq("x", m.id)
  end)
end)

describe("serialize_block", function()
  it("emite o link path-qualified quando há project", function()
    eq({
      "> [!todo] Título",
      "> [[tasks/p/x|x]]",
    }, T.serialize_block({ status_num = 1, title = "Título", id = "x", project = "p" }))
  end)

  it("aponta para archived/<tipo> quando a task está arquivada", function()
    eq({
      "> [!done] Título",
      "> [[tasks/p/archived/done/x|x]]",
    }, T.serialize_block({ status_num = 25, title = "Título", id = "x", project = "p", archived = "done" }))
  end)

  it("omite a linha de descrição quando ela está vazia", function()
    local out = T.serialize_block({ status_num = 1, title = "T", id = "x", project = "p", desc = "  " })
    eq(2, #out, "um `>` vazio no meio do bloco promoveria a nota seguinte a descrição")
  end)

  it("preserva verbatim o tipo inválido do status 0", function()
    local out = T.serialize_block({ status_num = 0, raw_callout = "questão", title = "T", id = "x", project = "p" })
    eq("> [!questão] T", out[1])
  end)

  it("cai em `invalid` quando o status 0 não tem tipo digitado", function()
    local out = T.serialize_block({ status_num = 0, raw_callout = "", title = "T", id = "x", project = "p" })
    eq("> [!invalid] T", out[1])
  end)

  it("apara linhas em branco no fim das notas", function()
    local out = T.serialize_block({
      status_num = 1,
      title = "T",
      id = "x",
      project = "p",
      extras = { "nota", "", "  " },
    })
    eq({ "> [!todo] T", "> [[tasks/p/x|x]]", "> nota" }, out)
  end)
end)

describe("round-trip parse/serialize", function()
  it("é idempotente para um bloco canônico", function()
    local original = {
      "> [!example] Título da task",
      "> [[tasks/bjju-web/fazer-x|fazer-x]]",
      "> _aguardando revisão_",
      "> impedimento: VPN",
      "> segunda nota",
    }
    local m = T.parse_block(original)
    m.project = "bjju-web"
    eq(original, T.serialize_block(m))
  end)

  it("normaliza o formato legado para o atual", function()
    local m = T.parse_block({
      "> **Minha task**",
      "> [[minha-task]]",
      "> 1 - Backlog",
      "> Branch: minha-task",
    })
    m.project = "p"
    eq({ "> [!todo] Minha task", "> [[tasks/p/minha-task|minha-task]]" }, T.serialize_block(m))
  end)

  it("sobrevive ao round-trip com status 0", function()
    local blk = { "> [!questão] T", "> [[tasks/p/x|x]]" }
    local m = T.parse_block(blk)
    m.project = "p"
    local out = T.serialize_block(m)
    eq(blk, out)
    eq(0, T.parse_block(out).status_num, "o erro precisa reparsear como 0, não sumir")
  end)
end)

describe("first_block_range", function()
  it("acha o bloco no topo", function()
    local s, e = X.first_block_range({ "> [!todo] T", "> [[x]]", "", "corpo" })
    eq(1, s)
    eq(2, e)
  end)

  it("pula frontmatter YAML e linhas em branco", function()
    local s, e = X.first_block_range({ "---", "id: x", "---", "", "> [!todo] T", "> [[x]]", "", "corpo" })
    eq(5, s)
    eq(6, e)
  end)

  it("devolve nil quando há conteúdo não-quote antes do bloco", function()
    falsy(X.first_block_range({ "# Título", "", "> [!todo] T" }))
  end)

  it("devolve nil quando não há bloco nenhum", function()
    falsy(X.first_block_range({ "corpo", "mais corpo" }))
    falsy(X.first_block_range({}))
  end)
end)

describe("block_around", function()
  it("expande para as bordas do bloco que contém a linha", function()
    local lines = { "texto", "> a", "> b", "> c", "texto" }
    local s, e = X.block_around(lines, 3)
    eq(2, s)
    eq(4, e)
  end)

  it("devolve nil fora de um blockquote", function()
    falsy(X.block_around({ "texto", "> a" }, 1))
  end)
end)

describe("splice", function()
  it("substitui a faixa preservando o entorno", function()
    eq({ "a", "X", "Y", "d" }, X.splice({ "a", "b", "c", "d" }, 2, 3, { "X", "Y" }))
  end)

  it("aceita substituição vazia", function()
    eq({ "a", "d" }, X.splice({ "a", "b", "c", "d" }, 2, 3, {}))
  end)
end)

describe("rewrite_link", function()
  local moved_old = { id = "x", full = "tasks/p/x" }
  local moved_new = { id = "x", full = "tasks/p/archived/done/x" }
  local renamed_old = { id = "a", full = "tasks/p/a" }
  local renamed_new = { id = "b", full = "tasks/p/b" }

  it("ao mover, reescreve só o link path-qualified", function()
    eq("tasks/p/archived/done/x|x", X.rewrite_link("tasks/p/x|x", moved_old, moved_new))
    eq(nil, X.rewrite_link("x", moved_old, moved_new), "o Obsidian resolve o link curto por basename")
  end)

  it("ao renomear, reescreve as duas formas", function()
    eq("b", X.rewrite_link("a", renamed_old, renamed_new))
    eq("tasks/p/b|b", X.rewrite_link("tasks/p/a|a", renamed_old, renamed_new))
  end)

  it("preserva um alias que não era o id", function()
    eq("tasks/p/b|Meu Título", X.rewrite_link("tasks/p/a|Meu Título", renamed_old, renamed_new))
  end)

  it("preserva sufixo de heading e de bloco", function()
    eq("tasks/p/b#Seção", X.rewrite_link("tasks/p/a#Seção", renamed_old, renamed_new))
    eq("tasks/p/b^bloco|b", X.rewrite_link("tasks/p/a^bloco|a", renamed_old, renamed_new))
  end)

  it("tolera o sufixo .md no alvo", function()
    eq("b", X.rewrite_link("a.md", renamed_old, renamed_new))
  end)

  it("devolve nil para link que aponta para outra coisa", function()
    eq(nil, X.rewrite_link("outra-nota", renamed_old, renamed_new))
    eq(nil, X.rewrite_link("tasks/outro/a|a", renamed_old, renamed_new))
  end)
end)

describe("append_history", function()
  local today = os.date("%Y-%m-%d")

  it("cria a seção quando ela não existe", function()
    local lines = { "# Task", "", "corpo", "", "" }
    X.append_history(lines, "done")
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
    X.append_history(lines, nil)
    eq("- " .. today .. " — desarquivada, de volta ao board", lines[4])
    eq("", lines[5], "a linha em branco de separação da seção é preservada")
    eq("## Outra seção", lines[6], "a entrada nova não pode vazar para depois da próxima seção")
  end)

  it("é um log: não sobrescreve a entrada anterior", function()
    local lines = { "## Histórico", "", "- 2026-01-01 — arquivada em `archived/done` (feito)" }
    X.append_history(lines, "dropped")
    eq(4, #lines)
  end)
end)

describe("update_frontmatter_key", function()
  it("atualiza a chave quando ela já existe", function()
    local lines = { "---", "id: velho", "tags: []", "---", "corpo" }
    truthy(X.update_frontmatter_key(lines, "id", "novo"))
    eq("id: novo", lines[2])
  end)

  it("não cria frontmatter nem chave ausente", function()
    local sem = { "corpo" }
    falsy(X.update_frontmatter_key(sem, "id", "novo"))
    eq({ "corpo" }, sem)

    local sem_chave = { "---", "tags: []", "---" }
    falsy(X.update_frontmatter_key(sem_chave, "id", "novo"))
    eq({ "---", "tags: []", "---" }, sem_chave)
  end)

  it("não reporta mudança quando o valor já está certo", function()
    falsy(X.update_frontmatter_key({ "---", "id: x", "---" }, "id", "x"))
  end)
end)

describe("suggest_archive_type", function()
  it("deriva done dos callouts de conclusão", function()
    eq("done", T.suggest_archive_type(25))
    eq("done", T.suggest_archive_type(23))
  end)

  it("deriva failed dos callouts de erro", function()
    eq("failed", T.suggest_archive_type(16))
    eq("failed", T.suggest_archive_type(15))
  end)

  it("cai em dropped para o resto, inclusive status 0", function()
    eq("dropped", T.suggest_archive_type(1))
    eq("dropped", T.suggest_archive_type(0))
    eq("dropped", T.suggest_archive_type(nil))
  end)
end)

describe("is_archived", function()
  it("lê o estado do CAMINHO", function()
    truthy(T.is_archived(VAULT .. "/tasks/p/archived/done/x.md"))
    falsy(T.is_archived(VAULT .. "/tasks/p/x.md"))
  end)
end)
