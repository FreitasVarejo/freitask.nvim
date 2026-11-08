//! Aceite da fatia 2 — Leitura (spec §5): `vault` varre e lê; `list` (com
//! `notes`, `historico`, `checklist`), `show`, `projects`, `statuses`,
//! `rebuild` (o painel), `version`, `help`.
//!
//! Fontes: `~/ObsidianVault/projects/workflow-ia/reescrita-do-freitask-spec.md`
//! (§1.5, §1.6, §2.1, §2.2, §2.4, I6, D6, D7, D9) e `docs/freitask.md`
//! ("Modelo mental", "Formato do bloco", "Prioridade", "Volta em", "Status 0",
//! "CURRENT.md", "Arquivar / desarquivar", "Encerrar / reabrir", "CLI").
//! Nenhum valor esperado aqui foi gerado da implementação. O dia fixo é
//! `FREITASK_TODAY=2026-11-08`, um domingo.
//!
//! Decisões tomadas aqui, onde a spec e o doc não respondem (listadas também
//! no relatório do rascunho):
//! - task sem callout no `list`: aparece, com `status_num` 0, `status`
//!   "Sem status", `title` "", `desc` "" e `hidden` false; `callout` não é
//!   afirmado nesse caso (não há o que ter sido digitado);
//! - `rebuild` só é afirmado pelo `CURRENT.md` (e `daily/`): nada se afirma
//!   sobre `projects/` nem `.freitask/status.json`, porque `rebuild` é escrita
//!   e a spec só lista as leituras como puras;
//! - `show` em texto imprime exatamente as linhas `> …` do bloco, cada uma com
//!   `\n`, e nada mais: nem frontmatter, nem corpo, nem linha em branco;
//! - o painel termina com a linha em branco que segue o último bloco (ou o
//!   `_(sem tasks)_`), isto é, os dois últimos bytes são `\n\n`;
//! - `desc` ausente é `""`, como o JSON de hoje emite (spec §2.2: forma
//!   idêntica); só `prioridade` e `volta_em` ausentes saem `null`.

mod util;

use std::fs;
use std::path::Path;
use std::thread::sleep;
use std::time::Duration;

use serde_json::{Value, json};
use util::*;

// ---------------------------------------------------------------------------
// Fixtures: blocos e arquivos de task escritos à mão.
// ---------------------------------------------------------------------------

/// Um bloco de task ativa: linha 1 e 2 obrigatórias, depois as `extras`
/// (descrição em itálico e/ou notas), cada uma como `> texto`.
fn bloco(callout: &str, titulo: &str, projeto: &str, id: &str, extras: &[&str]) -> String {
    let mut s = format!("> [!{callout}] {titulo}\n> [[projects/{projeto}/tasks/{id}|{id}]]\n");
    for e in extras {
        s.push_str(&format!("> {e}\n"));
    }
    s
}

/// Um bloco de task arquivada: o wikilink da linha 2 aponta para dentro de
/// `archived/<tipo>/`.
fn bloco_arquivado(
    callout: &str,
    titulo: &str,
    projeto: &str,
    tipo: &str,
    id: &str,
    extras: &[&str],
) -> String {
    let mut s = format!(
        "> [!{callout}] {titulo}\n> [[projects/{projeto}/tasks/archived/{tipo}/{id}|{id}]]\n"
    );
    for e in extras {
        s.push_str(&format!("> {e}\n"));
    }
    s
}

/// Frontmatter YAML com as linhas dadas, seguido de linha em branco.
fn frontmatter(linhas: &[&str]) -> String {
    let mut s = String::from("---\n");
    for l in linhas {
        s.push_str(l);
        s.push('\n');
    }
    s.push_str("---\n\n");
    s
}

/// `projects/<p>/tasks/<id>.md`.
fn rel_ativa(projeto: &str, id: &str) -> String {
    format!("projects/{projeto}/tasks/{id}.md")
}

/// `projects/<p>/tasks/archived/<tipo>/<id>.md`.
fn rel_arquivada(projeto: &str, tipo: &str, id: &str) -> String {
    format!("projects/{projeto}/tasks/archived/{tipo}/{id}.md")
}

/// Uma linha esperada do `list --json` / `show --json`, com todas as 21
/// chaves da tabela da spec §2.2 (as 13 de hoje mais `notes`, `historico`,
/// `checklist`, `bloqueada_por`, `bloqueada`, `bloqueia`, `prazo`,
/// `dias_para_o_prazo`). Os campos das fatias 9 e 10 são fixos nesta fatia:
/// `bloqueada_por: []`, `bloqueada: false`, `bloqueia: []`, `prazo: null`,
/// `dias_para_o_prazo: null`; `ciclo: null` no `show`.
#[derive(Clone, Debug)]
struct Linha {
    project: String,
    id: String,
    rel: String,
    status: String,
    status_num: u64,
    callout: String,
    title: String,
    desc: String,
    archived: Option<String>,
    suggested_archive: String,
    hidden: bool,
    prioridade: Option<String>,
    volta_em: Option<String>,
    notes: Vec<String>,
    historico: Vec<String>,
    feitos: u64,
    total: u64,
}

impl Linha {
    /// Task ativa, sem descrição (`desc` ""), sem notas, sem frontmatter, visível,
    /// `suggested_archive` "dropped" (o teste troca o que for diferente).
    fn ativa(
        project: &str,
        id: &str,
        callout: &str,
        status_num: u64,
        status: &str,
        title: &str,
    ) -> Self {
        Linha {
            project: project.into(),
            id: id.into(),
            rel: rel_ativa(project, id),
            status: status.into(),
            status_num,
            callout: callout.into(),
            title: title.into(),
            desc: String::new(),
            archived: None,
            suggested_archive: "dropped".into(),
            hidden: false,
            prioridade: None,
            volta_em: None,
            notes: vec![],
            historico: vec![],
            feitos: 0,
            total: 0,
        }
    }

    /// Task arquivada em `archived/<tipo>/`: `archived` é o tipo, `hidden`
    /// false, `prioridade` e `volta_em` null (docs/freitask.md, "Volta em":
    /// "arquivada não é adiada").
    fn arquivada(
        project: &str,
        tipo: &str,
        id: &str,
        callout: &str,
        status_num: u64,
        status: &str,
        title: &str,
    ) -> Self {
        let mut l = Linha::ativa(project, id, callout, status_num, status, title);
        l.rel = rel_arquivada(project, tipo, id);
        l.archived = Some(tipo.into());
        l
    }

    fn desc(mut self, d: &str) -> Self {
        self.desc = d.into();
        self
    }
    fn notas(mut self, n: &[&str]) -> Self {
        self.notes = n.iter().map(|s| s.to_string()).collect();
        self
    }
    fn historico(mut self, h: &[&str]) -> Self {
        self.historico = h.iter().map(|s| s.to_string()).collect();
        self
    }
    fn checklist(mut self, feitos: u64, total: u64) -> Self {
        self.feitos = feitos;
        self.total = total;
        self
    }
    fn escondida(mut self) -> Self {
        self.hidden = true;
        self
    }
    fn alta(mut self) -> Self {
        self.prioridade = Some("alta".into());
        self
    }
    fn volta_em(mut self, d: &str) -> Self {
        self.volta_em = Some(d.into());
        self
    }
    fn sugerida(mut self, s: &str) -> Self {
        self.suggested_archive = s.into();
        self
    }

    /// A linha como o `list --json` a devolve.
    fn json(&self, home: &Path) -> Value {
        json!({
            "archived": self.archived,
            "bloqueada": false,
            "bloqueada_por": [],
            "bloqueia": [],
            "callout": self.callout,
            "checklist": {"feitos": self.feitos, "total": self.total},
            "desc": self.desc,
            "dias_para_o_prazo": null,
            "hidden": self.hidden,
            "historico": self.historico,
            "id": self.id,
            "notes": self.notes,
            "path": abs(home, &self.rel),
            "prazo": null,
            "prioridade": self.prioridade,
            "project": self.project,
            "status": self.status,
            "status_num": self.status_num,
            "suggested_archive": self.suggested_archive,
            "title": self.title,
            "volta_em": self.volta_em,
        })
    }

    /// O objeto como o `show --json` o devolve: a linha mais `ciclo`.
    fn show_json(&self, home: &Path) -> Value {
        let mut v = self.json(home);
        v["ciclo"] = Value::Null;
        v
    }
}

/// As 21 chaves de uma linha do `list --json`, em ordem alfabética.
const CHAVES_LIST: [&str; 21] = [
    "archived",
    "bloqueada",
    "bloqueada_por",
    "bloqueia",
    "callout",
    "checklist",
    "desc",
    "dias_para_o_prazo",
    "hidden",
    "historico",
    "id",
    "notes",
    "path",
    "prazo",
    "prioridade",
    "project",
    "status",
    "status_num",
    "suggested_archive",
    "title",
    "volta_em",
];

/// Um `status.json` próprio, com títulos diferentes do default, para provar
/// que `status` vem do vault e não do default embutido.
const STATUS_JSON_PROPRIO: &str = r#"{
  "1": { "title": "Fazer", "callout": "todo", "icon": "A", "hl_group": "Normal" },
  "2": { "title": "Fazendo", "callout": "example", "icon": "B", "hl_group": "Normal" },
  "3": { "title": "Feita", "callout": "check", "icon": "C", "hl_group": "DiagnosticOk" }
}"#;

/// Um vault com de tudo um pouco, para as leituras puras e os schemas: dois
/// projetos com task, um vazio, um encerrado, arquivadas dos três tipos,
/// escondidas, adiada, frontmatter alheio, corpo com checklist e histórico,
/// `CURRENT.md` e `daily/` já existentes, `status.json` default por escrito.
fn home_completo() -> tempfile::TempDir {
    let home = home_com(&[
        (".freitask/status.json", gerador::STATUS_JSON),
        (
            "projects/alfa/tasks/x.md",
            &(frontmatter(&["id: x", "aliases: [xis]", "tags: [a, b]"])
                + &bloco(
                    "todo",
                    "Xis",
                    "alfa",
                    "x",
                    &["_em curso_", "nota um", "nota dois"],
                )
                + "\n## Notas\n\n- [x] feito\n- [ ] aberto\n- [ ] outro\n"),
        ),
        (
            "projects/alfa/tasks/escondida.md",
            &bloco("quote", "Depois", "alfa", "escondida", &[]),
        ),
        (
            "projects/alfa/tasks/adiada.md",
            &(frontmatter(&["volta-em: 2026-11-09"])
                + &bloco("example", "Adiada", "alfa", "adiada", &[])),
        ),
        (
            "projects/alfa/tasks/archived/done/feita.md",
            &(bloco_arquivado("done", "Feita", "alfa", "done", "feita", &["_acabou_"])
                + "\n## Histórico\n\n- 2026-08-30 — arquivada em `archived/done` (feito)\n"),
        ),
        (
            "projects/alfa/tasks/archived/dropped/largada.md",
            &bloco_arquivado("example", "Largada", "alfa", "dropped", "largada", &[]),
        ),
        (
            "projects/alfa/tasks/archived/failed/falhou.md",
            &bloco_arquivado("warning", "Falhou", "alfa", "failed", "falhou", &[]),
        ),
        (
            "projects/beta/tasks/y.md",
            &(frontmatter(&["prioridade: alta"]) + &bloco("check", "Ypsilon", "beta", "y", &[])),
        ),
        (
            "projects/beta/tasks/z.md",
            &bloco("questão", "Zê", "beta", "z", &[]),
        ),
        (
            "projects/_encerrados/omega/tasks/fim.md",
            &bloco("todo", "Fim", "omega", "fim", &[]),
        ),
        ("daily/2026-11-06.md", "# painel velho\n"),
        (
            "CURRENT.md",
            "---\ndate: 2026-11-07\nweekday: Saturday\n---\n\n# Planning diário 07/11/26\n\n## Notas Avulsas\n\n- nota\n\n## alfa\n\n_(sem tasks)_\n\n",
        ),
        ("notas/ref.md", "ver [[projects/alfa/tasks/x|x]]\n"),
    ]);
    pasta(home.path(), "projects/gama/tasks");
    home
}

// ---------------------------------------------------------------------------
// `freitask list --json`: forma
// ---------------------------------------------------------------------------

#[test]
fn list_json_num_vault_sem_task_devolve_array_vazio() {
    let home = home_com_vault_vazio();
    pasta(home.path(), "projects");
    let saida = roda(home.path(), &["list", "--json"]);
    ok(&saida);
    assert_eq!(json(&saida), json!([]));
}

#[test]
fn list_json_num_home_sem_vault_devolve_array_vazio_e_sai_0() {
    let home = home_sem_vault();
    let saida = roda(home.path(), &["list", "--json"]);
    ok(&saida);
    assert_eq!(json(&saida), json!([]));
}

#[test]
fn list_json_num_vault_so_com_projeto_vazio_devolve_array_vazio() {
    let home = home_com_vault_vazio();
    pasta(home.path(), "projects/alfa/tasks");
    let saida = roda(home.path(), &["list", "--json"]);
    ok(&saida);
    assert_eq!(json(&saida), json!([]));
}

#[test]
fn list_json_cada_linha_tem_exatamente_as_21_chaves() {
    let home = home_completo();
    let saida = roda(home.path(), &["list", "--json", "--archived"]);
    ok(&saida);
    let linhas = json(&saida);
    assert!(!linhas.as_array().expect("array").is_empty());
    for l in linhas.as_array().expect("array") {
        assert_eq!(chaves(l), CHAVES_LIST.to_vec(), "linha {l}");
    }
}

#[test]
fn list_json_task_minima_sai_com_todos_os_campos_no_default() {
    let home = home_com(&[(
        "projects/p/tasks/t1.md",
        &bloco("todo", "Primeira", "p", "t1", &[]),
    )]);
    let saida = roda(home.path(), &["list", "--json"]);
    ok(&saida);
    let esperado = Linha::ativa("p", "t1", "todo", 1, "Não iniciada", "Primeira");
    assert_eq!(json(&saida), json!([esperado.json(home.path())]));
}

#[test]
fn list_json_path_e_absoluto_dentro_do_home() {
    let home = home_com(&[(
        "projects/p/tasks/t1.md",
        &bloco("todo", "Primeira", "p", "t1", &[]),
    )]);
    let saida = roda(home.path(), &["list", "--json"]);
    ok(&saida);
    let linhas = json(&saida);
    let path = linhas[0]["path"].as_str().expect("path é string");
    assert_eq!(path, abs(home.path(), "projects/p/tasks/t1.md"));
    assert!(Path::new(path).is_absolute());
}

#[test]
fn list_json_le_desc_em_italico_e_as_notas_sem_o_prefixo() {
    let home = home_com(&[(
        "projects/p/tasks/t1.md",
        &bloco(
            "example",
            "Com notas",
            "p",
            "t1",
            &[
                "_aguardando revisão_",
                "impedimento: falta a VPN",
                "_itálico depois de nota é nota_",
            ],
        ),
    )]);
    let saida = roda(home.path(), &["list", "--json"]);
    ok(&saida);
    let esperado = Linha::ativa("p", "t1", "example", 4, "Em implementação", "Com notas")
        .desc("aguardando revisão")
        .notas(&[
            "impedimento: falta a VPN",
            "_itálico depois de nota é nota_",
        ]);
    assert_eq!(json(&saida), json!([esperado.json(home.path())]));
}

#[test]
fn list_json_linha_3_sem_italico_e_nota_e_desc_fica_vazia() {
    let home = home_com(&[(
        "projects/p/tasks/t1.md",
        &bloco("todo", "Sem desc", "p", "t1", &["só uma nota"]),
    )]);
    let saida = roda(home.path(), &["list", "--json"]);
    ok(&saida);
    let esperado =
        Linha::ativa("p", "t1", "todo", 1, "Não iniciada", "Sem desc").notas(&["só uma nota"]);
    assert_eq!(json(&saida), json!([esperado.json(home.path())]));
}

#[test]
fn list_json_conta_o_checklist_do_corpo() {
    let home = home_com(&[(
        "projects/p/tasks/t1.md",
        &(bloco("todo", "Com checklist", "p", "t1", &[])
            + "\n## Passos\n\n- [x] um\n- [ ] dois\n- [x] três\n- [ ] quatro\n- [ ] cinco\n\ntexto solto\n"),
    )]);
    let saida = roda(home.path(), &["list", "--json"]);
    ok(&saida);
    assert_eq!(
        json(&saida)[0]["checklist"],
        json!({"feitos": 2, "total": 5})
    );
}

#[test]
fn list_json_checklist_nao_conta_item_dentro_do_bloco() {
    let home = home_com(&[(
        "projects/p/tasks/t1.md",
        &(bloco("todo", "Item no bloco", "p", "t1", &["- [ ] isto é nota"])
            + "\n- [x] isto conta\n"),
    )]);
    let saida = roda(home.path(), &["list", "--json"]);
    ok(&saida);
    let l = json(&saida);
    assert_eq!(l[0]["checklist"], json!({"feitos": 1, "total": 1}));
    assert_eq!(l[0]["notes"], json!(["- [ ] isto é nota"]));
}

#[test]
fn list_json_checklist_zerado_sem_corpo() {
    let home = home_com(&[(
        "projects/p/tasks/t1.md",
        &bloco("todo", "Sem corpo", "p", "t1", &[]),
    )]);
    let saida = roda(home.path(), &["list", "--json"]);
    ok(&saida);
    assert_eq!(
        json(&saida)[0]["checklist"],
        json!({"feitos": 0, "total": 0})
    );
}

#[test]
fn list_json_le_o_historico_na_ordem_do_arquivo_com_a_linha_inteira() {
    let home = home_com(&[(
        "projects/p/tasks/t1.md",
        &(bloco("todo", "Com histórico", "p", "t1", &[])
            + "\n## Histórico\n\n- 2026-08-09 — arquivada em `archived/dropped` (abandonado)\n- 2026-08-11 — desarquivada, de volta ao board\n- 2026-09-05 — renomeada de `foo` para `t1`\n"),
    )]);
    let saida = roda(home.path(), &["list", "--json"]);
    ok(&saida);
    assert_eq!(
        json(&saida)[0]["historico"],
        json!([
            "- 2026-08-09 — arquivada em `archived/dropped` (abandonado)",
            "- 2026-08-11 — desarquivada, de volta ao board",
            "- 2026-09-05 — renomeada de `foo` para `t1`",
        ])
    );
}

#[test]
fn list_json_historico_so_le_a_secao_historico() {
    // Uma linha datada fora de `## Histórico` não é histórico; a seção acaba
    // na próxima `## seção` (docs/freitask.md, "Rodapé ## Histórico").
    let home = home_com(&[(
        "projects/p/tasks/t1.md",
        &(bloco("todo", "Seções", "p", "t1", &[])
            + "\n## Notas\n\n- 2026-01-01 — isto não é histórico\n\n## Histórico\n\n- 2026-09-02 — movida de `q` para `p`\n\n## Depois\n\n- 2026-10-10 — nem isto\n"),
    )]);
    let saida = roda(home.path(), &["list", "--json"]);
    ok(&saida);
    assert_eq!(
        json(&saida)[0]["historico"],
        json!(["- 2026-09-02 — movida de `q` para `p`"])
    );
}

#[test]
fn list_json_historico_vazio_sem_a_secao() {
    let home = home_com(&[(
        "projects/p/tasks/t1.md",
        &(bloco("todo", "Sem histórico", "p", "t1", &[]) + "\n## Notas\n\ntexto\n"),
    )]);
    let saida = roda(home.path(), &["list", "--json"]);
    ok(&saida);
    assert_eq!(json(&saida)[0]["historico"], json!([]));
}

#[test]
fn list_json_preserva_task_com_frontmatter_de_chaves_alheias() {
    let home = home_com(&[(
        "projects/p/tasks/t1.md",
        &(frontmatter(&[
            "id: t1",
            "aliases: [primeira]",
            "tags: [x, y]",
            "created: 2026-01-01",
        ]) + &bloco(
            "tip",
            "Com frontmatter",
            "p",
            "t1",
            &["_pronta para codar_"],
        )),
    )]);
    let saida = roda(home.path(), &["list", "--json"]);
    ok(&saida);
    let esperado =
        Linha::ativa("p", "t1", "tip", 3, "Refinada", "Com frontmatter").desc("pronta para codar");
    assert_eq!(json(&saida), json!([esperado.json(home.path())]));
}

#[test]
fn list_json_status_e_o_titulo_da_fase_no_status_json_do_vault() {
    let home = home_com(&[
        (".freitask/status.json", STATUS_JSON_PROPRIO),
        (
            "projects/p/tasks/t1.md",
            &bloco("example", "Um", "p", "t1", &[]),
        ),
    ]);
    let saida = roda(home.path(), &["list", "--json"]);
    ok(&saida);
    let l = json(&saida);
    assert_eq!(l[0]["status"], json!("Fazendo"));
    assert_eq!(l[0]["status_num"], json!(2));
    assert_eq!(l[0]["callout"], json!("example"));
}

// ---------------------------------------------------------------------------
// `freitask list --json`: fases, status 0, prioridade, suggested_archive
// ---------------------------------------------------------------------------

#[test]
fn list_json_cada_fase_ativa_tem_seu_status_num_e_titulo() {
    let fases = [
        ("todo", 1, "Não iniciada"),
        ("abstract", 2, "Refinando"),
        ("tip", 3, "Refinada"),
        ("example", 4, "Em implementação"),
        ("question", 5, "Em homologação"),
        ("warning", 6, "Bloqueada"),
        ("check", 7, "Pronta"),
        ("quote", 8, "Algum dia"),
    ];
    let arquivos: Vec<(String, String)> = fases
        .iter()
        .map(|(c, _, _)| {
            (
                rel_ativa("p", c),
                bloco(c, &format!("Fase {c}"), "p", c, &[]),
            )
        })
        .collect();
    let refs: Vec<(&str, &str)> = arquivos
        .iter()
        .map(|(a, b)| (a.as_str(), b.as_str()))
        .collect();
    let home = home_com(&refs);
    let saida = roda(home.path(), &["list", "--json"]);
    ok(&saida);
    let linhas = json(&saida);
    for (callout, num, titulo) in fases {
        let l = linha(&linhas, callout);
        assert_eq!(l["status_num"], json!(num), "{callout}");
        assert_eq!(l["status"], json!(titulo), "{callout}");
        assert_eq!(l["callout"], json!(callout), "{callout}");
    }
}

#[test]
fn list_json_callout_desconhecido_e_status_0_com_o_tipo_preservado() {
    let home = home_com(&[(
        "projects/p/tasks/t1.md",
        &bloco("questão", "Com typo", "p", "t1", &["_estado_"]),
    )]);
    let saida = roda(home.path(), &["list", "--json"]);
    ok(&saida);
    let esperado = Linha::ativa("p", "t1", "questão", 0, "Sem status", "Com typo").desc("estado");
    assert_eq!(json(&saida), json!([esperado.json(home.path())]));
}

#[test]
fn list_json_task_sem_callout_aparece_com_status_0_e_titulo_vazio() {
    // Decisão deste rascunho: o arquivo em `tasks/` é task (o `doctor` acusa
    // `sem-callout`, mas o `list` não o esconde); sem bloco não há título.
    let home = home_com(&[
        (
            "projects/p/tasks/sem-callout.md",
            "# Só um título\n\nSem bloco nenhum.\n",
        ),
        (
            "projects/p/tasks/t1.md",
            &bloco("todo", "Normal", "p", "t1", &[]),
        ),
    ]);
    let saida = roda(home.path(), &["list", "--json"]);
    ok(&saida);
    let linhas = json(&saida);
    assert_eq!(ids(&linhas), vec!["sem-callout", "t1"]);
    let l = linha(&linhas, "sem-callout");
    assert_eq!(l["status_num"], json!(0));
    assert_eq!(l["status"], json!("Sem status"));
    assert_eq!(l["title"], json!(""));
    assert_eq!(l["desc"], json!(""));
    assert_eq!(l["hidden"], json!(false));
    assert_eq!(l["archived"], Value::Null);
    assert_eq!(l["notes"], json!([]));
    assert_eq!(
        l["path"],
        json!(abs(home.path(), "projects/p/tasks/sem-callout.md"))
    );
}

#[test]
fn list_json_prioridade_alta_e_lida_do_frontmatter() {
    let home = home_com(&[(
        "projects/p/tasks/t1.md",
        &(frontmatter(&["prioridade: alta"]) + &bloco("todo", "Alta", "p", "t1", &[])),
    )]);
    let saida = roda(home.path(), &["list", "--json"]);
    ok(&saida);
    let esperado = Linha::ativa("p", "t1", "todo", 1, "Não iniciada", "Alta").alta();
    assert_eq!(json(&saida), json!([esperado.json(home.path())]));
}

#[test]
fn list_json_prioridade_com_outro_valor_e_null() {
    // Só `alta` conta; `Alta` é "normal" (docs/freitask.md, "Prioridade").
    let home = home_com(&[
        (
            "projects/p/tasks/maiuscula.md",
            &(frontmatter(&["prioridade: Alta"]) + &bloco("todo", "A", "p", "maiuscula", &[])),
        ),
        (
            "projects/p/tasks/urgente.md",
            &(frontmatter(&["prioridade: urgente"]) + &bloco("todo", "B", "p", "urgente", &[])),
        ),
    ]);
    let saida = roda(home.path(), &["list", "--json"]);
    ok(&saida);
    let linhas = json(&saida);
    assert_eq!(linha(&linhas, "maiuscula")["prioridade"], Value::Null);
    assert_eq!(linha(&linhas, "urgente")["prioridade"], Value::Null);
}

#[test]
fn list_json_suggested_archive_e_done_para_check_e_dropped_para_o_resto() {
    let home = home_com(&[
        (
            "projects/p/tasks/pronta.md",
            &bloco("check", "P", "p", "pronta", &[]),
        ),
        (
            "projects/p/tasks/nova.md",
            &bloco("todo", "N", "p", "nova", &[]),
        ),
        (
            "projects/p/tasks/travada.md",
            &bloco("warning", "T", "p", "travada", &[]),
        ),
        (
            "projects/p/tasks/depois.md",
            &bloco("quote", "D", "p", "depois", &[]),
        ),
        (
            "projects/p/tasks/zero.md",
            &bloco("xyz", "Z", "p", "zero", &[]),
        ),
    ]);
    let saida = roda(home.path(), &["list", "--json"]);
    ok(&saida);
    let linhas = json(&saida);
    assert_eq!(linha(&linhas, "pronta")["suggested_archive"], json!("done"));
    assert_eq!(
        linha(&linhas, "nova")["suggested_archive"],
        json!("dropped")
    );
    assert_eq!(
        linha(&linhas, "travada")["suggested_archive"],
        json!("dropped")
    );
    assert_eq!(
        linha(&linhas, "depois")["suggested_archive"],
        json!("dropped")
    );
    assert_eq!(
        linha(&linhas, "zero")["suggested_archive"],
        json!("dropped")
    );
}

// ---------------------------------------------------------------------------
// `freitask list --json`: hidden e volta_em
// ---------------------------------------------------------------------------

#[test]
fn list_json_algum_dia_e_hidden_e_segue_na_lista() {
    let home = home_com(&[(
        "projects/p/tasks/t1.md",
        &bloco("quote", "Um dia", "p", "t1", &[]),
    )]);
    let saida = roda(home.path(), &["list", "--json"]);
    ok(&saida);
    let esperado = Linha::ativa("p", "t1", "quote", 8, "Algum dia", "Um dia").escondida();
    assert_eq!(json(&saida), json!([esperado.json(home.path())]));
}

#[test]
fn list_json_volta_em_futura_esconde_e_informa_a_data() {
    let home = home_com(&[(
        "projects/p/tasks/t1.md",
        &(frontmatter(&["volta-em: 2026-11-09"]) + &bloco("todo", "Amanhã", "p", "t1", &[])),
    )]);
    let saida = roda(home.path(), &["list", "--json"]);
    ok(&saida);
    let esperado = Linha::ativa("p", "t1", "todo", 1, "Não iniciada", "Amanhã")
        .escondida()
        .volta_em("2026-11-09");
    assert_eq!(json(&saida), json!([esperado.json(home.path())]));
}

#[test]
fn list_json_volta_em_de_hoje_ja_mostra_a_task() {
    // "É o primeiro dia visível": no dia da data ela já aparece.
    let home = home_com(&[(
        "projects/p/tasks/t1.md",
        &(frontmatter(&["volta-em: 2026-11-08"]) + &bloco("todo", "Hoje", "p", "t1", &[])),
    )]);
    let saida = roda(home.path(), &["list", "--json"]);
    ok(&saida);
    let esperado = Linha::ativa("p", "t1", "todo", 1, "Não iniciada", "Hoje");
    assert_eq!(json(&saida), json!([esperado.json(home.path())]));
}

#[test]
fn list_json_volta_em_passada_nao_esconde_e_volta_em_e_null() {
    let home = home_com(&[(
        "projects/p/tasks/t1.md",
        &(frontmatter(&["volta-em: 2026-11-07"]) + &bloco("todo", "Ontem", "p", "t1", &[])),
    )]);
    let saida = roda(home.path(), &["list", "--json"]);
    ok(&saida);
    let esperado = Linha::ativa("p", "t1", "todo", 1, "Não iniciada", "Ontem");
    assert_eq!(json(&saida), json!([esperado.json(home.path())]));
}

#[test]
fn list_json_volta_em_invalida_nao_esconde() {
    // `2026-12-1`, `01/12/2026` e `2026-02-30` não são datas: a task aparece
    // (docs/freitask.md, "A data é uma data").
    let home = home_com(&[
        (
            "projects/p/tasks/curta.md",
            &(frontmatter(&["volta-em: 2026-12-1"]) + &bloco("todo", "A", "p", "curta", &[])),
        ),
        (
            "projects/p/tasks/barras.md",
            &(frontmatter(&["volta-em: 01/12/2026"]) + &bloco("todo", "B", "p", "barras", &[])),
        ),
        (
            "projects/p/tasks/fevereiro.md",
            &(frontmatter(&["volta-em: 2026-02-30"]) + &bloco("todo", "C", "p", "fevereiro", &[])),
        ),
    ]);
    let saida = roda(home.path(), &["list", "--json"]);
    ok(&saida);
    let linhas = json(&saida);
    for id in ["curta", "barras", "fevereiro"] {
        let l = linha(&linhas, id);
        assert_eq!(l["hidden"], json!(false), "{id}");
        assert_eq!(l["volta_em"], Value::Null, "{id}");
    }
}

#[test]
fn list_json_volta_em_vazio_conta_como_sem_data() {
    let home = home_com(&[(
        "projects/p/tasks/t1.md",
        &(frontmatter(&["volta-em:"]) + &bloco("todo", "Vazio", "p", "t1", &[])),
    )]);
    let saida = roda(home.path(), &["list", "--json"]);
    ok(&saida);
    let esperado = Linha::ativa("p", "t1", "todo", 1, "Não iniciada", "Vazio");
    assert_eq!(json(&saida), json!([esperado.json(home.path())]));
}

#[test]
fn list_json_adiada_de_prioridade_alta_fica_escondida_e_mantem_a_prioridade() {
    let home = home_com(&[(
        "projects/p/tasks/t1.md",
        &(frontmatter(&["prioridade: alta", "volta-em: 2026-12-01"])
            + &bloco("example", "Alta adiada", "p", "t1", &[])),
    )]);
    let saida = roda(home.path(), &["list", "--json"]);
    ok(&saida);
    let esperado = Linha::ativa("p", "t1", "example", 4, "Em implementação", "Alta adiada")
        .alta()
        .escondida()
        .volta_em("2026-12-01");
    assert_eq!(json(&saida), json!([esperado.json(home.path())]));
}

#[test]
fn list_json_arquivada_com_volta_em_futura_nao_e_hidden_nem_adiada() {
    let home = home_com(&[(
        "projects/p/tasks/archived/dropped/t0.md",
        &(frontmatter(&["volta-em: 2026-12-01", "prioridade: alta"])
            + &bloco_arquivado("todo", "Velha", "p", "dropped", "t0", &[])),
    )]);
    let saida = roda(home.path(), &["list", "--json", "--archived"]);
    ok(&saida);
    let esperado = Linha::arquivada("p", "dropped", "t0", "todo", 1, "Não iniciada", "Velha");
    assert_eq!(json(&saida), json!([esperado.json(home.path())]));
}

// ---------------------------------------------------------------------------
// `freitask list --json`: o que entra e o que não entra
// ---------------------------------------------------------------------------

#[test]
fn list_json_sem_archived_nao_lista_as_arquivadas() {
    let home = home_com(&[
        (
            "projects/p/tasks/t1.md",
            &bloco("todo", "Ativa", "p", "t1", &[]),
        ),
        (
            "projects/p/tasks/archived/done/t0.md",
            &bloco_arquivado("done", "Feita", "p", "done", "t0", &[]),
        ),
        (
            "projects/p/tasks/archived/dropped/t2.md",
            &bloco_arquivado("todo", "Largada", "p", "dropped", "t2", &[]),
        ),
        (
            "projects/p/tasks/archived/failed/t3.md",
            &bloco_arquivado("warning", "Falhou", "p", "failed", "t3", &[]),
        ),
    ]);
    let saida = roda(home.path(), &["list", "--json"]);
    ok(&saida);
    assert_eq!(ids(&json(&saida)), vec!["t1"]);
}

#[test]
fn list_json_archived_lista_as_arquivadas_depois_das_ativas_do_projeto() {
    let home = home_com(&[
        (
            "projects/p/tasks/t1.md",
            &bloco("todo", "Ativa", "p", "t1", &[]),
        ),
        (
            "projects/p/tasks/archived/done/t0.md",
            &(bloco_arquivado("done", "Feita", "p", "done", "t0", &["_acabou_"])
                + "\n## Histórico\n\n- 2026-08-30 — arquivada em `archived/done` (feito)\n"),
        ),
        (
            "projects/p/tasks/archived/dropped/t2.md",
            &bloco_arquivado("example", "Largada", "p", "dropped", "t2", &[]),
        ),
        (
            "projects/p/tasks/archived/failed/t3.md",
            &bloco_arquivado("warning", "Falhou", "p", "failed", "t3", &[]),
        ),
    ]);
    let saida = roda(home.path(), &["list", "--json", "--archived"]);
    ok(&saida);
    let esperado = vec![
        Linha::ativa("p", "t1", "todo", 1, "Não iniciada", "Ativa").json(home.path()),
        Linha::arquivada("p", "done", "t0", "done", 9, "Arquivada", "Feita")
            .desc("acabou")
            .sugerida("done")
            .historico(&["- 2026-08-30 — arquivada em `archived/done` (feito)"])
            .json(home.path()),
        Linha::arquivada(
            "p",
            "dropped",
            "t2",
            "example",
            4,
            "Em implementação",
            "Largada",
        )
        .json(home.path()),
        Linha::arquivada("p", "failed", "t3", "warning", 6, "Bloqueada", "Falhou")
            .json(home.path()),
    ];
    assert_eq!(json(&saida), Value::Array(esperado));
}

#[test]
fn list_json_archived_ordena_por_tipo_e_depois_por_id() {
    let home = home_com(&[
        (
            "projects/p/tasks/archived/failed/a-f.md",
            &bloco_arquivado("todo", "A", "p", "failed", "a-f", &[]),
        ),
        (
            "projects/p/tasks/archived/done/z-d.md",
            &bloco_arquivado("done", "Z", "p", "done", "z-d", &[]),
        ),
        (
            "projects/p/tasks/archived/dropped/m-x.md",
            &bloco_arquivado("todo", "M", "p", "dropped", "m-x", &[]),
        ),
        (
            "projects/p/tasks/archived/done/a-d.md",
            &bloco_arquivado("done", "A", "p", "done", "a-d", &[]),
        ),
    ]);
    let saida = roda(home.path(), &["list", "--json", "--archived"]);
    ok(&saida);
    assert_eq!(ids(&json(&saida)), vec!["a-d", "z-d", "m-x", "a-f"]);
}

#[test]
fn list_json_archived_intercala_por_projeto_ativas_e_depois_arquivadas() {
    let home = home_com(&[
        (
            "projects/a/tasks/a1.md",
            &bloco("todo", "A1", "a", "a1", &[]),
        ),
        (
            "projects/a/tasks/archived/done/a0.md",
            &bloco_arquivado("done", "A0", "a", "done", "a0", &[]),
        ),
        (
            "projects/b/tasks/b1.md",
            &bloco("todo", "B1", "b", "b1", &[]),
        ),
        (
            "projects/b/tasks/archived/dropped/b0.md",
            &bloco_arquivado("todo", "B0", "b", "dropped", "b0", &[]),
        ),
    ]);
    let saida = roda(home.path(), &["list", "--json", "--archived"]);
    ok(&saida);
    assert_eq!(ids(&json(&saida)), vec!["a1", "a0", "b1", "b0"]);
}

#[test]
fn list_json_ignora_projetos_reservados() {
    let home = home_com(&[
        (
            "projects/daily/tasks/d.md",
            &bloco("todo", "D", "daily", "d", &[]),
        ),
        (
            "projects/templates/tasks/t.md",
            &bloco("todo", "T", "templates", "t", &[]),
        ),
        (
            "projects/_encerrados/tasks/e.md",
            &bloco("todo", "E", "_encerrados", "e", &[]),
        ),
        (
            "projects/p/tasks/t1.md",
            &bloco("todo", "Normal", "p", "t1", &[]),
        ),
    ]);
    let saida = roda(home.path(), &["list", "--json"]);
    ok(&saida);
    assert_eq!(ids(&json(&saida)), vec!["t1"]);
}

#[test]
fn list_json_ignora_projeto_encerrado() {
    let home = home_com(&[
        (
            "projects/_encerrados/omega/tasks/fim.md",
            &bloco("todo", "Fim", "omega", "fim", &[]),
        ),
        (
            "projects/_encerrados/omega/tasks/archived/done/feita.md",
            &bloco_arquivado("done", "Feita", "omega", "done", "feita", &[]),
        ),
        (
            "projects/p/tasks/t1.md",
            &bloco("todo", "Normal", "p", "t1", &[]),
        ),
    ]);
    let saida = roda(home.path(), &["list", "--json", "--archived"]);
    ok(&saida);
    assert_eq!(ids(&json(&saida)), vec!["t1"]);
}

#[test]
fn list_json_ignora_pasta_de_projeto_sem_tasks() {
    // Projeto é subdiretório de `projects/` com `tasks/` dentro; sem `tasks/`,
    // a pasta não é projeto (docs/freitask.md, "Modelo mental").
    let home = home_com(&[
        ("projects/notas/moc.md", "# MOC\n"),
        (
            "projects/notas/ideia.md",
            &bloco("todo", "Ideia", "notas", "ideia", &[]),
        ),
        (
            "projects/p/tasks/t1.md",
            &bloco("todo", "Normal", "p", "t1", &[]),
        ),
    ]);
    let saida = roda(home.path(), &["list", "--json"]);
    ok(&saida);
    assert_eq!(ids(&json(&saida)), vec!["t1"]);
}

#[test]
fn list_json_ignora_sync_conflict() {
    let home = home_com(&[
        (
            "projects/p/tasks/t1.md",
            &bloco("todo", "Normal", "p", "t1", &[]),
        ),
        (
            "projects/p/tasks/t1.sync-conflict-20260809-123456-ABCDEFG.md",
            &bloco("todo", "Conflito", "p", "t1", &[]),
        ),
        (
            "projects/p/tasks/archived/done/t0.sync-conflict-20260809-123456-ABCDEFG.md",
            &bloco_arquivado("done", "Conflito", "p", "done", "t0", &[]),
        ),
    ]);
    let saida = roda(home.path(), &["list", "--json", "--archived"]);
    ok(&saida);
    assert_eq!(ids(&json(&saida)), vec!["t1"]);
}

#[test]
fn list_json_ignora_arquivo_em_subpasta_de_tasks() {
    let home = home_com(&[
        (
            "projects/p/tasks/t1.md",
            &bloco("todo", "Normal", "p", "t1", &[]),
        ),
        (
            "projects/p/tasks/rascunho/ideia.md",
            &bloco("todo", "Ideia", "p", "ideia", &[]),
        ),
        (
            "projects/p/tasks/archived/solta.md",
            &bloco("todo", "Solta", "p", "solta", &[]),
        ),
    ]);
    let saida = roda(home.path(), &["list", "--json", "--archived"]);
    ok(&saida);
    assert_eq!(ids(&json(&saida)), vec!["t1"]);
}

#[test]
fn list_json_ignora_arquivo_que_nao_e_md() {
    let home = home_com(&[
        (
            "projects/p/tasks/t1.md",
            &bloco("todo", "Normal", "p", "t1", &[]),
        ),
        ("projects/p/tasks/anexo.txt", "texto\n"),
        ("projects/p/tasks/.keep", ""),
    ]);
    let saida = roda(home.path(), &["list", "--json"]);
    ok(&saida);
    assert_eq!(ids(&json(&saida)), vec!["t1"]);
}

#[test]
fn list_json_mostra_as_duas_tasks_de_um_id_duplicado_em_dois_projetos() {
    let home = home_com(&[
        (
            "projects/a/tasks/dup.md",
            &bloco("todo", "Em a", "a", "dup", &[]),
        ),
        (
            "projects/b/tasks/dup.md",
            &bloco("example", "Em b", "b", "dup", &[]),
        ),
    ]);
    let saida = roda(home.path(), &["list", "--json"]);
    ok(&saida);
    let esperado = vec![
        Linha::ativa("a", "dup", "todo", 1, "Não iniciada", "Em a").json(home.path()),
        Linha::ativa("b", "dup", "example", 4, "Em implementação", "Em b").json(home.path()),
    ];
    assert_eq!(json(&saida), Value::Array(esperado));
}

// ---------------------------------------------------------------------------
// `freitask list --json`: ordem
// ---------------------------------------------------------------------------

#[test]
fn list_json_ordena_projetos_em_ordem_alfabetica() {
    let home = home_com(&[
        (
            "projects/zeta/tasks/z.md",
            &bloco("todo", "Z", "zeta", "z", &[]),
        ),
        (
            "projects/alfa/tasks/a.md",
            &bloco("todo", "A", "alfa", "a", &[]),
        ),
        (
            "projects/meio/tasks/m.md",
            &bloco("todo", "M", "meio", "m", &[]),
        ),
    ]);
    let saida = roda(home.path(), &["list", "--json"]);
    ok(&saida);
    assert_eq!(ids(&json(&saida)), vec!["a", "m", "z"]);
}

#[test]
fn list_json_dentro_do_projeto_alta_primeiro_depois_fase_depois_id() {
    let home = home_com(&[
        (
            "projects/p/tasks/z-alta-pronta.md",
            &(frontmatter(&["prioridade: alta"]) + &bloco("check", "Z", "p", "z-alta-pronta", &[])),
        ),
        (
            "projects/p/tasks/y-alta-nova.md",
            &(frontmatter(&["prioridade: alta"]) + &bloco("todo", "Y", "p", "y-alta-nova", &[])),
        ),
        (
            "projects/p/tasks/b-nova.md",
            &bloco("todo", "B", "p", "b-nova", &[]),
        ),
        (
            "projects/p/tasks/a-nova.md",
            &bloco("todo", "A", "p", "a-nova", &[]),
        ),
        (
            "projects/p/tasks/a-implementando.md",
            &bloco("example", "AI", "p", "a-implementando", &[]),
        ),
        (
            "projects/p/tasks/c-zero.md",
            &bloco("questão", "C", "p", "c-zero", &[]),
        ),
        (
            "projects/p/tasks/d-depois.md",
            &bloco("quote", "D", "p", "d-depois", &[]),
        ),
    ]);
    let saida = roda(home.path(), &["list", "--json"]);
    ok(&saida);
    assert_eq!(
        ids(&json(&saida)),
        vec![
            "y-alta-nova",
            "z-alta-pronta",
            "c-zero",
            "a-nova",
            "b-nova",
            "a-implementando",
            "d-depois",
        ]
    );
}

#[test]
fn list_json_adiada_fica_na_posicao_da_sua_fase() {
    // `hidden` não reordena o `list`: a adiada segue na lista, no lugar da
    // fase dela.
    let home = home_com(&[
        ("projects/p/tasks/a.md", &bloco("todo", "A", "p", "a", &[])),
        (
            "projects/p/tasks/b.md",
            &(frontmatter(&["volta-em: 2026-12-01"]) + &bloco("todo", "B", "p", "b", &[])),
        ),
        ("projects/p/tasks/c.md", &bloco("todo", "C", "p", "c", &[])),
    ]);
    let saida = roda(home.path(), &["list", "--json"]);
    ok(&saida);
    assert_eq!(ids(&json(&saida)), vec!["a", "b", "c"]);
}

// ---------------------------------------------------------------------------
// `freitask list` em texto
// ---------------------------------------------------------------------------

/// O vault do `list` em texto: `alfa` com `x` (todo) e `y` (check),
/// `meu-projeto` com `t1` (example), uma arquivada `done` e uma `dropped`.
fn home_list_texto() -> tempfile::TempDir {
    home_com(&[
        (
            "projects/alfa/tasks/x.md",
            &bloco("todo", "Xis", "alfa", "x", &[]),
        ),
        (
            "projects/alfa/tasks/y.md",
            &bloco("check", "Ypsilon", "alfa", "y", &[]),
        ),
        (
            "projects/meu-projeto/tasks/t1.md",
            &bloco("example", "Primeira", "meu-projeto", "t1", &["_em curso_"]),
        ),
        (
            "projects/meu-projeto/tasks/archived/done/t0.md",
            &bloco_arquivado("done", "Zero", "meu-projeto", "done", "t0", &[]),
        ),
        (
            "projects/meu-projeto/tasks/archived/dropped/largada.md",
            &bloco_arquivado(
                "example",
                "Largada",
                "meu-projeto",
                "dropped",
                "largada",
                &[],
            ),
        ),
    ])
}

/// `string.format("%-14s %-34s %-12s %s", project, id, status, "")`, com o
/// padding contado em bytes: "Não iniciada" (13 bytes) e "Em implementação"
/// (18 bytes) não ganham padding, e o quarto campo vazio deixa o espaço final.
const LIST_TEXTO: &str = concat!(
    "alfa           x                                  Não iniciada \n",
    "alfa           y                                  Pronta       \n",
    "meu-projeto    t1                                 Em implementação \n",
);

const LIST_TEXTO_ARCHIVED: &str = concat!(
    "alfa           x                                  Não iniciada \n",
    "alfa           y                                  Pronta       \n",
    "meu-projeto    t1                                 Em implementação \n",
    "meu-projeto    t0                                 Arquivada    [done]\n",
    "meu-projeto    largada                            Em implementação [dropped]\n",
);

#[test]
fn list_texto_uma_linha_por_task_com_padding_em_bytes() {
    let home = home_list_texto();
    let saida = roda(home.path(), &["list"]);
    ok(&saida);
    assert_eq!(saida.stdout, LIST_TEXTO);
}

#[test]
fn list_texto_archived_acrescenta_o_tipo_entre_colchetes() {
    let home = home_list_texto();
    let saida = roda(home.path(), &["list", "--archived"]);
    ok(&saida);
    assert_eq!(saida.stdout, LIST_TEXTO_ARCHIVED);
}

#[test]
fn list_texto_status_0_imprime_sem_status() {
    let home = home_com(&[(
        "projects/beta/tasks/z.md",
        &bloco("questão", "Z", "beta", "z", &[]),
    )]);
    let saida = roda(home.path(), &["list"]);
    ok(&saida);
    assert_eq!(
        saida.stdout,
        "beta           z                                  Sem status   \n"
    );
}

#[test]
fn list_texto_campo_maior_que_a_coluna_nao_e_cortado() {
    let home = home_com(&[(
        "projects/projeto-com-nome-bem-longo/tasks/id-tambem-bem-longo-para-passar-de-34-bytes.md",
        &bloco(
            "tip",
            "Longa",
            "projeto-com-nome-bem-longo",
            "id-tambem-bem-longo-para-passar-de-34-bytes",
            &[],
        ),
    )]);
    let saida = roda(home.path(), &["list"]);
    ok(&saida);
    assert_eq!(
        saida.stdout,
        "projeto-com-nome-bem-longo id-tambem-bem-longo-para-passar-de-34-bytes Refinada     \n"
    );
}

#[test]
fn list_texto_inclui_as_escondidas() {
    let home = home_com(&[
        (
            "projects/p/tasks/depois.md",
            &bloco("quote", "D", "p", "depois", &[]),
        ),
        (
            "projects/p/tasks/adiada.md",
            &(frontmatter(&["volta-em: 2026-12-01"]) + &bloco("todo", "A", "p", "adiada", &[])),
        ),
    ]);
    let saida = roda(home.path(), &["list"]);
    ok(&saida);
    assert_eq!(
        saida.stdout,
        concat!(
            "p              adiada                             Não iniciada \n",
            "p              depois                             Algum dia    \n",
        )
    );
}

#[test]
fn list_texto_num_vault_sem_task_nao_imprime_nada() {
    let home = home_com_vault_vazio();
    let saida = roda(home.path(), &["list"]);
    ok(&saida);
    assert_eq!(saida.stdout, "");
}

#[test]
fn list_texto_num_home_sem_vault_nao_imprime_nada_e_sai_0() {
    let home = home_sem_vault();
    let saida = roda(home.path(), &["list"]);
    ok(&saida);
    assert_eq!(saida.stdout, "");
}

// ---------------------------------------------------------------------------
// `freitask show <id|caminho> [--json]`
// ---------------------------------------------------------------------------

/// O vault do `show`: `p` com `t1` (frontmatter, desc, notas, corpo com
/// checklist e histórico), `t2` simples, uma arquivada `done`; `q` com `t2`
/// também (id duplicado); `omega` encerrado com `fim`.
fn home_show() -> tempfile::TempDir {
    home_com(&[
        (
            "projects/p/tasks/t1.md",
            &(frontmatter(&["id: t1", "prioridade: alta", "tags: [x]"])
                + &bloco(
                    "example",
                    "Primeira",
                    "p",
                    "t1",
                    &["_em curso_", "nota um", "nota dois"],
                )
                + "\n## Passos\n\n- [x] um\n- [ ] dois\n\n## Histórico\n\n- 2026-09-02 — movida de `q` para `p`\n"),
        ),
        (
            "projects/p/tasks/t2.md",
            &bloco("todo", "Segunda em p", "p", "t2", &[]),
        ),
        (
            "projects/p/tasks/archived/done/t0.md",
            &(bloco_arquivado("done", "Zero", "p", "done", "t0", &[])
                + "\n## Histórico\n\n- 2026-08-30 — arquivada em `archived/done` (feito)\n"),
        ),
        (
            "projects/q/tasks/t2.md",
            &bloco("tip", "Segunda em q", "q", "t2", &[]),
        ),
        (
            "projects/_encerrados/omega/tasks/fim.md",
            &bloco("todo", "Fim", "omega", "fim", &[]),
        ),
    ])
}

fn t1_esperada() -> Linha {
    Linha::ativa("p", "t1", "example", 4, "Em implementação", "Primeira")
        .alta()
        .desc("em curso")
        .notas(&["nota um", "nota dois"])
        .checklist(1, 2)
        .historico(&["- 2026-09-02 — movida de `q` para `p`"])
}

const T1_BLOCO: &str = concat!(
    "> [!example] Primeira\n",
    "> [[projects/p/tasks/t1|t1]]\n",
    "> _em curso_\n",
    "> nota um\n",
    "> nota dois\n",
);

#[test]
fn show_json_por_id_devolve_a_linha_do_list_mais_ciclo_null() {
    let home = home_show();
    let saida = roda(home.path(), &["show", "t1", "--json"]);
    ok(&saida);
    assert_eq!(json(&saida), t1_esperada().show_json(home.path()));
}

#[test]
fn show_json_tem_exatamente_as_22_chaves() {
    let home = home_show();
    let saida = roda(home.path(), &["show", "t1", "--json"]);
    ok(&saida);
    let mut esperadas: Vec<String> = CHAVES_LIST.iter().map(|s| s.to_string()).collect();
    esperadas.push("ciclo".into());
    esperadas.sort();
    assert_eq!(chaves(&json(&saida)), esperadas);
}

#[test]
fn show_json_e_igual_a_linha_correspondente_do_list_json() {
    let home = home_show();
    let lista = roda(home.path(), &["list", "--json"]);
    ok(&lista);
    let mostra = roda(home.path(), &["show", "t1", "--json"]);
    ok(&mostra);
    let mut da_lista = linha(&json(&lista), "t1").clone();
    da_lista["ciclo"] = Value::Null;
    assert_eq!(json(&mostra), da_lista);
}

#[test]
fn show_json_resolve_id_de_task_arquivada() {
    let home = home_show();
    let saida = roda(home.path(), &["show", "t0", "--json"]);
    ok(&saida);
    let esperado = Linha::arquivada("p", "done", "t0", "done", 9, "Arquivada", "Zero")
        .sugerida("done")
        .historico(&["- 2026-08-30 — arquivada em `archived/done` (feito)"]);
    assert_eq!(json(&saida), esperado.show_json(home.path()));
}

#[test]
fn show_json_aceita_caminho_relativo_ao_cwd() {
    // O cwd é o HOME, então o caminho relativo começa em `ObsidianVault/`.
    let home = home_show();
    let saida = roda(
        home.path(),
        &["show", "ObsidianVault/projects/p/tasks/t1.md", "--json"],
    );
    ok(&saida);
    assert_eq!(json(&saida), t1_esperada().show_json(home.path()));
}

#[test]
fn show_json_aceita_caminho_absoluto() {
    let home = home_show();
    let caminho = abs(home.path(), "projects/p/tasks/t1.md");
    let saida = roda(home.path(), &["show", &caminho, "--json"]);
    ok(&saida);
    assert_eq!(json(&saida), t1_esperada().show_json(home.path()));
}

#[test]
fn show_json_por_caminho_desambigua_id_duplicado() {
    let home = home_show();
    let em_q = roda(
        home.path(),
        &["show", "ObsidianVault/projects/q/tasks/t2.md", "--json"],
    );
    ok(&em_q);
    assert_eq!(
        json(&em_q),
        Linha::ativa("q", "t2", "tip", 3, "Refinada", "Segunda em q").show_json(home.path())
    );
    let em_p = roda(
        home.path(),
        &["show", "ObsidianVault/projects/p/tasks/t2.md", "--json"],
    );
    ok(&em_p);
    assert_eq!(
        json(&em_p),
        Linha::ativa("p", "t2", "todo", 1, "Não iniciada", "Segunda em p").show_json(home.path())
    );
}

#[test]
fn show_texto_imprime_o_bloco_como_esta_no_arquivo_e_nada_mais() {
    let home = home_show();
    let saida = roda(home.path(), &["show", "t1"]);
    ok(&saida);
    assert_eq!(saida.stdout, T1_BLOCO);
}

#[test]
fn show_texto_de_task_arquivada_imprime_o_bloco_dela() {
    let home = home_show();
    let saida = roda(home.path(), &["show", "t0"]);
    ok(&saida);
    assert_eq!(
        saida.stdout,
        "> [!done] Zero\n> [[projects/p/tasks/archived/done/t0|t0]]\n"
    );
}

#[test]
fn show_texto_por_caminho_imprime_o_bloco() {
    let home = home_show();
    let saida = roda(
        home.path(),
        &["show", "ObsidianVault/projects/q/tasks/t2.md"],
    );
    ok(&saida);
    assert_eq!(
        saida.stdout,
        "> [!tip] Segunda em q\n> [[projects/q/tasks/t2|t2]]\n"
    );
}

#[test]
fn show_id_inexistente_sai_1_e_diz_task_nao_encontrada() {
    let home = home_show();
    let saida = roda(home.path(), &["show", "nao-existe"]);
    assert_eq!(saida.codigo, Some(1));
    assert_eq!(saida.stdout, "");
    assert_eq!(saida.stderr, "freitask: task não encontrada: nao-existe\n");
}

#[test]
fn show_json_id_inexistente_tambem_sai_1_sem_json() {
    let home = home_show();
    let saida = roda(home.path(), &["show", "nao-existe", "--json"]);
    assert_eq!(saida.codigo, Some(1));
    assert_eq!(saida.stdout, "");
    assert_eq!(saida.stderr, "freitask: task não encontrada: nao-existe\n");
}

#[test]
fn show_nao_acha_task_de_projeto_encerrado() {
    let home = home_show();
    let saida = roda(home.path(), &["show", "fim"]);
    assert_eq!(saida.codigo, Some(1));
    assert_eq!(saida.stdout, "");
    assert_eq!(saida.stderr, "freitask: task não encontrada: fim\n");
}

#[test]
fn show_id_ambiguo_sai_1_e_manda_passar_o_caminho() {
    let home = home_show();
    let saida = roda(home.path(), &["show", "t2"]);
    assert_eq!(saida.codigo, Some(1));
    assert_eq!(saida.stdout, "");
    assert_eq!(
        saida.stderr,
        "freitask: id \"t2\" é ambíguo (2 arquivos); passe o caminho\n"
    );
}

#[test]
fn show_id_ambiguo_entre_ativa_e_arquivada_de_outro_projeto_tambem_recusa() {
    let home = home_com(&[
        (
            "projects/a/tasks/dup.md",
            &bloco("todo", "Ativa", "a", "dup", &[]),
        ),
        (
            "projects/b/tasks/archived/done/dup.md",
            &bloco_arquivado("done", "Feita", "b", "done", "dup", &[]),
        ),
    ]);
    let saida = roda(home.path(), &["show", "dup", "--json"]);
    assert_eq!(saida.codigo, Some(1));
    assert_eq!(saida.stdout, "");
    assert_eq!(
        saida.stderr,
        "freitask: id \"dup\" é ambíguo (2 arquivos); passe o caminho\n"
    );
}

#[test]
fn show_sem_argumento_sai_2_e_diz_que_falta_o_id() {
    let home = home_show();
    let saida = roda(home.path(), &["show"]);
    assert_eq!(saida.codigo, Some(2));
    assert_eq!(saida.stdout, "");
    assert_eq!(saida.stderr, "freitask: falta o id ou caminho da task\n");
}

#[test]
fn show_json_sem_argumento_sai_2_e_diz_que_falta_o_id() {
    let home = home_show();
    let saida = roda(home.path(), &["show", "--json"]);
    assert_eq!(saida.codigo, Some(2));
    assert_eq!(saida.stdout, "");
    assert_eq!(saida.stderr, "freitask: falta o id ou caminho da task\n");
}

// ---------------------------------------------------------------------------
// `freitask projects [--json]`
// ---------------------------------------------------------------------------

/// O vault do `projects`: `alfa` com 3 ativas visíveis, 1 Algum dia, 1 adiada
/// e 2 arquivadas; `beta` com 1 (status 0); `gama` com `tasks/` vazio; `zeta`
/// com 1 adiada que já voltou; `omega` encerrado; `templates`, `daily` e uma
/// pasta sem `tasks/`.
fn home_projects() -> tempfile::TempDir {
    let home = home_com(&[
        (
            "projects/alfa/tasks/a1.md",
            &bloco("todo", "A1", "alfa", "a1", &[]),
        ),
        (
            "projects/alfa/tasks/a2.md",
            &(frontmatter(&["prioridade: alta"]) + &bloco("example", "A2", "alfa", "a2", &[])),
        ),
        (
            "projects/alfa/tasks/a3.md",
            &bloco("check", "A3", "alfa", "a3", &[]),
        ),
        (
            "projects/alfa/tasks/depois.md",
            &bloco("quote", "D", "alfa", "depois", &[]),
        ),
        (
            "projects/alfa/tasks/adiada.md",
            &(frontmatter(&["volta-em: 2026-11-09"]) + &bloco("todo", "Ad", "alfa", "adiada", &[])),
        ),
        (
            "projects/alfa/tasks/archived/done/a0.md",
            &bloco_arquivado("done", "A0", "alfa", "done", "a0", &[]),
        ),
        (
            "projects/alfa/tasks/archived/dropped/a9.md",
            &bloco_arquivado("todo", "A9", "alfa", "dropped", "a9", &[]),
        ),
        (
            "projects/beta/tasks/z.md",
            &bloco("questão", "Z", "beta", "z", &[]),
        ),
        (
            "projects/zeta/tasks/voltou.md",
            &(frontmatter(&["volta-em: 2026-11-08"]) + &bloco("todo", "V", "zeta", "voltou", &[])),
        ),
        (
            "projects/_encerrados/omega/tasks/fim.md",
            &bloco("todo", "Fim", "omega", "fim", &[]),
        ),
        (
            "projects/templates/tasks/t.md",
            &bloco("todo", "T", "templates", "t", &[]),
        ),
        (
            "projects/daily/tasks/d.md",
            &bloco("todo", "D", "daily", "d", &[]),
        ),
        ("projects/notas/moc.md", "# MOC sem tasks/\n"),
    ]);
    pasta(home.path(), "projects/gama/tasks");
    home
}

#[test]
fn projects_json_conta_so_as_ativas_que_o_painel_mostra() {
    let home = home_projects();
    let saida = roda(home.path(), &["projects", "--json"]);
    ok(&saida);
    assert_eq!(
        json(&saida),
        json!([
            {"active": 3, "project": "alfa"},
            {"active": 1, "project": "beta"},
            {"active": 0, "project": "gama"},
            {"active": 1, "project": "zeta"},
        ])
    );
}

#[test]
fn projects_json_cada_objeto_tem_exatamente_as_duas_chaves() {
    let home = home_projects();
    let saida = roda(home.path(), &["projects", "--json"]);
    ok(&saida);
    for o in json(&saida).as_array().expect("array") {
        assert_eq!(chaves(o), vec!["active", "project"]);
    }
}

#[test]
fn projects_json_inclui_projeto_com_tasks_vazio() {
    let home = home_com_vault_vazio();
    pasta(home.path(), "projects/sozinho/tasks");
    let saida = roda(home.path(), &["projects", "--json"]);
    ok(&saida);
    assert_eq!(json(&saida), json!([{"active": 0, "project": "sozinho"}]));
}

#[test]
fn projects_json_exclui_encerrados_reservados_e_pasta_sem_tasks() {
    let home = home_com(&[
        (
            "projects/_encerrados/omega/tasks/fim.md",
            &bloco("todo", "Fim", "omega", "fim", &[]),
        ),
        (
            "projects/templates/tasks/t.md",
            &bloco("todo", "T", "templates", "t", &[]),
        ),
        (
            "projects/daily/tasks/d.md",
            &bloco("todo", "D", "daily", "d", &[]),
        ),
        ("projects/notas/moc.md", "# MOC\n"),
    ]);
    let saida = roda(home.path(), &["projects", "--json"]);
    ok(&saida);
    assert_eq!(json(&saida), json!([]));
}

#[test]
fn projects_json_num_vault_sem_projects_devolve_array_vazio() {
    let home = home_com_vault_vazio();
    let saida = roda(home.path(), &["projects", "--json"]);
    ok(&saida);
    assert_eq!(json(&saida), json!([]));
}

#[test]
fn projects_json_num_home_sem_vault_devolve_array_vazio_e_sai_0() {
    let home = home_sem_vault();
    let saida = roda(home.path(), &["projects", "--json"]);
    ok(&saida);
    assert_eq!(json(&saida), json!([]));
}

#[test]
fn projects_texto_e_nome_com_padding_de_14_bytes_e_a_contagem() {
    // `string.format("%-14s %d", project, active)`.
    let home = home_projects();
    let saida = roda(home.path(), &["projects"]);
    ok(&saida);
    assert_eq!(
        saida.stdout,
        concat!(
            "alfa           3\n",
            "beta           1\n",
            "gama           0\n",
            "zeta           1\n",
        )
    );
}

#[test]
fn projects_texto_nome_maior_que_a_coluna_nao_e_cortado() {
    let home = home_com_vault_vazio();
    pasta(home.path(), "projects/projeto-com-nome-bem-longo/tasks");
    let saida = roda(home.path(), &["projects"]);
    ok(&saida);
    assert_eq!(saida.stdout, "projeto-com-nome-bem-longo 0\n");
}

#[test]
fn projects_texto_num_home_sem_vault_nao_imprime_nada_e_sai_0() {
    let home = home_sem_vault();
    let saida = roda(home.path(), &["projects"]);
    ok(&saida);
    assert_eq!(saida.stdout, "");
}

// ---------------------------------------------------------------------------
// `freitask rebuild`: o painel
// ---------------------------------------------------------------------------

/// O vault do painel: `alfa` com `x` (todo, com desc e nota; corpo com
/// checklist), uma Algum dia, uma adiada e uma arquivada; `beta` com `y`
/// (alta, check); `gama` vazio; `omega` encerrado; `templates`.
fn home_painel() -> tempfile::TempDir {
    let home = home_com(&[
        (
            "projects/alfa/tasks/x.md",
            &(bloco("todo", "Xis", "alfa", "x", &["_em curso_", "nota"])
                + "\n## Notas\n\n- [ ] a\n"),
        ),
        (
            "projects/alfa/tasks/escondida.md",
            &bloco("quote", "Depois", "alfa", "escondida", &[]),
        ),
        (
            "projects/alfa/tasks/adiada.md",
            &(frontmatter(&["volta-em: 2026-11-09"])
                + &bloco("example", "Adiada", "alfa", "adiada", &[])),
        ),
        (
            "projects/alfa/tasks/archived/done/velha.md",
            &bloco_arquivado("done", "Velha", "alfa", "done", "velha", &[]),
        ),
        (
            "projects/beta/tasks/y.md",
            &(frontmatter(&["prioridade: alta"]) + &bloco("check", "Ypsilon", "beta", "y", &[])),
        ),
        (
            "projects/_encerrados/omega/tasks/z.md",
            &bloco("todo", "Z", "omega", "z", &[]),
        ),
        (
            "projects/templates/tasks/t.md",
            &bloco("todo", "T", "templates", "t", &[]),
        ),
    ]);
    pasta(home.path(), "projects/gama/tasks");
    home
}

/// O painel de `home_painel()` em 2026-11-08, byte a byte (docs/freitask.md,
/// "CURRENT.md — o painel diário"): 2 tasks mostradas (a Algum dia, a adiada
/// e a arquivada ficam de fora), 3 projetos (o `gama` vazio conta; o
/// encerrado e o `templates` não).
const PAINEL: &str = concat!(
    "---\n",
    "date: 2026-11-08\n",
    "weekday: Sunday\n",
    "---\n",
    "\n",
    "# Planning diário 08/11/26\n",
    "\n",
    "> Painel gerado automaticamente — 2 tasks em 3 projetos.\n",
    "\n",
    "## Notas Avulsas\n",
    "\n",
    "- \n",
    "\n",
    "## alfa\n",
    "\n",
    "> [!todo] Xis\n",
    "> [[projects/alfa/tasks/x|x]]\n",
    "> _em curso_\n",
    "> nota\n",
    "\n",
    "## beta\n",
    "\n",
    "> [!check] Ypsilon\n",
    "> [[projects/beta/tasks/y|y]]\n",
    "\n",
    "## gama\n",
    "\n",
    "_(sem tasks)_\n",
    "\n",
);

#[test]
fn rebuild_grava_o_painel_e_imprime_o_caminho_dele() {
    let home = home_painel();
    let saida = roda(home.path(), &["rebuild"]);
    ok(&saida);
    assert_eq!(
        saida.stdout,
        format!("{}\n", abs(home.path(), "CURRENT.md"))
    );
    assert_eq!(le(home.path(), "CURRENT.md"), PAINEL);
}

#[test]
fn rebuild_sem_current_anterior_nao_cria_daily() {
    let home = home_painel();
    ok(&roda(home.path(), &["rebuild"]));
    assert!(!vault(home.path()).join("daily").exists());
}

#[test]
fn rebuild_painel_de_vault_so_com_projeto_vazio() {
    let home = home_com_vault_vazio();
    pasta(home.path(), "projects/alfa/tasks");
    pasta(home.path(), "projects/beta/tasks");
    let saida = roda(home.path(), &["rebuild"]);
    ok(&saida);
    assert_eq!(
        le(home.path(), "CURRENT.md"),
        concat!(
            "---\n",
            "date: 2026-11-08\n",
            "weekday: Sunday\n",
            "---\n",
            "\n",
            "# Planning diário 08/11/26\n",
            "\n",
            "> Painel gerado automaticamente — 0 tasks em 2 projetos.\n",
            "\n",
            "## Notas Avulsas\n",
            "\n",
            "- \n",
            "\n",
            "## alfa\n",
            "\n",
            "_(sem tasks)_\n",
            "\n",
            "## beta\n",
            "\n",
            "_(sem tasks)_\n",
            "\n",
        )
    );
}

#[test]
fn rebuild_num_vault_sem_freitask_e_sem_projects_cria_o_current() {
    // Só o CURRENT.md é afirmado: `rebuild` é escrita, e o que ele faz com
    // `projects/` e `.freitask/` não é contrato desta fatia.
    let home = home_com_vault_vazio();
    let saida = roda(home.path(), &["rebuild"]);
    ok(&saida);
    assert_eq!(
        saida.stdout,
        format!("{}\n", abs(home.path(), "CURRENT.md"))
    );
    let painel = le(home.path(), "CURRENT.md");
    assert!(
        painel.starts_with(concat!(
            "---\n",
            "date: 2026-11-08\n",
            "weekday: Sunday\n",
            "---\n",
            "\n",
            "# Planning diário 08/11/26\n",
            "\n",
        )),
        "painel:\n{painel}"
    );
    assert!(
        painel.contains("\n## Notas Avulsas\n\n- \n"),
        "painel:\n{painel}"
    );
    assert!(painel.ends_with('\n'));
}

#[test]
fn rebuild_usa_o_dia_de_freitask_today_no_cabecalho() {
    let home = home_com_vault_vazio();
    pasta(home.path(), "projects/alfa/tasks");
    let mut cmd = comando(home.path());
    cmd.env("FREITASK_TODAY", "2026-12-25").args(["rebuild"]);
    let saida = cmd.output().expect("rodar");
    assert_eq!(saida.status.code(), Some(0));
    let painel = le(home.path(), "CURRENT.md");
    assert!(
        painel.starts_with(concat!(
            "---\n",
            "date: 2026-12-25\n",
            "weekday: Friday\n",
            "---\n",
            "\n",
            "# Planning diário 25/12/26\n",
            "\n",
        )),
        "painel:\n{painel}"
    );
}

#[test]
fn rebuild_duas_vezes_da_os_mesmos_bytes_e_nao_toca_o_mtime() {
    // D6 / I4: escrita sem mudança não grava.
    let home = home_painel();
    ok(&roda(home.path(), &["rebuild"]));
    let caminho = vault(home.path()).join("CURRENT.md");
    let bytes_1 = fs::read(&caminho).expect("ler");
    let mtime_1 = fs::metadata(&caminho)
        .expect("metadata")
        .modified()
        .expect("mtime");

    sleep(Duration::from_millis(1100));

    ok(&roda(home.path(), &["rebuild"]));
    let bytes_2 = fs::read(&caminho).expect("ler");
    let mtime_2 = fs::metadata(&caminho)
        .expect("metadata")
        .modified()
        .expect("mtime");
    assert_eq!(bytes_1, bytes_2);
    assert_eq!(bytes_2, PAINEL.as_bytes());
    assert_eq!(
        mtime_1, mtime_2,
        "o segundo rebuild não pode regravar o painel"
    );
}

#[test]
fn rebuild_reflete_mudanca_na_task() {
    let home = home_painel();
    ok(&roda(home.path(), &["rebuild"]));
    arquivo(
        home.path(),
        "projects/beta/tasks/y.md",
        &(frontmatter(&["prioridade: alta"])
            + &bloco("example", "Ypsilon mudou", "beta", "y", &[])),
    );
    ok(&roda(home.path(), &["rebuild"]));
    let painel = le(home.path(), "CURRENT.md");
    assert!(
        painel.contains("## beta\n\n> [!example] Ypsilon mudou\n> [[projects/beta/tasks/y|y]]\n\n"),
        "painel:\n{painel}"
    );
    assert!(!painel.contains("[!check] Ypsilon"));
}

#[test]
fn rebuild_nao_toca_arquivo_de_task_nenhum() {
    let home = home_painel();
    let antes = fotografa(&vault(home.path()).join("projects"));
    ok(&roda(home.path(), &["rebuild"]));
    assert_eq!(fotografa(&vault(home.path()).join("projects")), antes);
}

#[test]
fn rebuild_ordena_as_tasks_do_projeto_como_o_list() {
    let home = home_com(&[
        (
            "projects/p/tasks/z-alta.md",
            &(frontmatter(&["prioridade: alta"]) + &bloco("check", "Z", "p", "z-alta", &[])),
        ),
        ("projects/p/tasks/b.md", &bloco("todo", "B", "p", "b", &[])),
        ("projects/p/tasks/a.md", &bloco("todo", "A", "p", "a", &[])),
        (
            "projects/p/tasks/c.md",
            &bloco("questão", "C", "p", "c", &[]),
        ),
        (
            "projects/p/tasks/d.md",
            &bloco("example", "D", "p", "d", &[]),
        ),
    ]);
    ok(&roda(home.path(), &["rebuild"]));
    let painel = le(home.path(), "CURRENT.md");
    assert!(
        painel.ends_with(concat!(
            "## p\n",
            "\n",
            "> [!check] Z\n",
            "> [[projects/p/tasks/z-alta|z-alta]]\n",
            "\n",
            "> [!questão] C\n",
            "> [[projects/p/tasks/c|c]]\n",
            "\n",
            "> [!todo] A\n",
            "> [[projects/p/tasks/a|a]]\n",
            "\n",
            "> [!todo] B\n",
            "> [[projects/p/tasks/b|b]]\n",
            "\n",
            "> [!example] D\n",
            "> [[projects/p/tasks/d|d]]\n",
            "\n",
        )),
        "painel:\n{painel}"
    );
}

#[test]
fn rebuild_nao_mostra_nem_conta_algum_dia_e_adiada() {
    let home = home_com(&[
        ("projects/p/tasks/a.md", &bloco("todo", "A", "p", "a", &[])),
        (
            "projects/p/tasks/depois.md",
            &bloco("quote", "D", "p", "depois", &[]),
        ),
        (
            "projects/p/tasks/adiada.md",
            &(frontmatter(&["volta-em: 2026-11-09"]) + &bloco("todo", "Ad", "p", "adiada", &[])),
        ),
        (
            "projects/p/tasks/voltou.md",
            &(frontmatter(&["volta-em: 2026-11-08"]) + &bloco("todo", "V", "p", "voltou", &[])),
        ),
        ("projects/q/tasks/b.md", &bloco("todo", "B", "q", "b", &[])),
    ]);
    ok(&roda(home.path(), &["rebuild"]));
    let painel = le(home.path(), "CURRENT.md");
    assert!(
        painel.contains("— 3 tasks em 2 projetos.\n"),
        "painel:\n{painel}"
    );
    assert!(painel.contains("[[projects/p/tasks/a|a]]"));
    assert!(painel.contains("[[projects/p/tasks/voltou|voltou]]"));
    assert!(painel.contains("[[projects/q/tasks/b|b]]"));
    assert!(!painel.contains("depois"), "painel:\n{painel}");
    assert!(!painel.contains("adiada"), "painel:\n{painel}");
}

#[test]
fn rebuild_projeto_so_com_escondidas_imprime_sem_tasks() {
    let home = home_com(&[
        (
            "projects/p/tasks/depois.md",
            &bloco("quote", "D", "p", "depois", &[]),
        ),
        ("projects/q/tasks/b.md", &bloco("todo", "B", "q", "b", &[])),
    ]);
    ok(&roda(home.path(), &["rebuild"]));
    let painel = le(home.path(), "CURRENT.md");
    assert!(
        painel.contains("## p\n\n_(sem tasks)_\n\n## q\n\n> [!todo] B\n"),
        "painel:\n{painel}"
    );
}

// ---------------------------------------------------------------------------
// `freitask rebuild`: o CURRENT.md anterior, o daily e as Notas Avulsas
// ---------------------------------------------------------------------------

/// Um CURRENT.md de ontem, com notas escritas à mão.
const CURRENT_DE_ONTEM: &str = concat!(
    "---\n",
    "date: 2026-11-07\n",
    "weekday: Saturday\n",
    "---\n",
    "\n",
    "# Planning diário 07/11/26\n",
    "\n",
    "> Painel gerado automaticamente — 1 tasks em 1 projetos.\n",
    "\n",
    "## Notas Avulsas\n",
    "\n",
    "- lembrar de X\n",
    "- ligar para Y\n",
    "\n",
    "## alfa\n",
    "\n",
    "> [!todo] Velha\n",
    "> [[projects/alfa/tasks/velha|velha]]\n",
    "\n",
);

/// O painel de `home_painel()` depois de preservar as notas de
/// `CURRENT_DE_ONTEM`.
const PAINEL_COM_NOTAS: &str = concat!(
    "---\n",
    "date: 2026-11-08\n",
    "weekday: Sunday\n",
    "---\n",
    "\n",
    "# Planning diário 08/11/26\n",
    "\n",
    "> Painel gerado automaticamente — 2 tasks em 3 projetos.\n",
    "\n",
    "## Notas Avulsas\n",
    "\n",
    "- lembrar de X\n",
    "- ligar para Y\n",
    "\n",
    "## alfa\n",
    "\n",
    "> [!todo] Xis\n",
    "> [[projects/alfa/tasks/x|x]]\n",
    "> _em curso_\n",
    "> nota\n",
    "\n",
    "## beta\n",
    "\n",
    "> [!check] Ypsilon\n",
    "> [[projects/beta/tasks/y|y]]\n",
    "\n",
    "## gama\n",
    "\n",
    "_(sem tasks)_\n",
    "\n",
);

#[test]
fn rebuild_copia_o_current_de_ontem_para_daily_antes_de_regravar() {
    let home = home_painel();
    arquivo(home.path(), "CURRENT.md", CURRENT_DE_ONTEM);
    let saida = roda(home.path(), &["rebuild"]);
    ok(&saida);
    assert_eq!(le(home.path(), "daily/2026-11-07.md"), CURRENT_DE_ONTEM);
    assert_eq!(le(home.path(), "CURRENT.md"), PAINEL_COM_NOTAS);
}

#[test]
fn rebuild_preserva_as_notas_avulsas_e_regenera_o_resto() {
    let home = home_painel();
    arquivo(home.path(), "CURRENT.md", CURRENT_DE_ONTEM);
    ok(&roda(home.path(), &["rebuild"]));
    let painel = le(home.path(), "CURRENT.md");
    assert!(
        painel.contains("## Notas Avulsas\n\n- lembrar de X\n- ligar para Y\n\n## alfa\n"),
        "painel:\n{painel}"
    );
    assert!(
        !painel.contains("Velha"),
        "a seção de projeto antiga não sobrevive"
    );
    assert!(!painel.contains("2026-11-07"));
}

#[test]
fn rebuild_preserva_as_notas_avulsas_no_mesmo_dia_sem_copiar_para_daily() {
    let home = home_painel();
    arquivo(
        home.path(),
        "CURRENT.md",
        &CURRENT_DE_ONTEM
            .replace("date: 2026-11-07", "date: 2026-11-08")
            .replace("weekday: Saturday", "weekday: Sunday")
            .replace("07/11/26", "08/11/26"),
    );
    ok(&roda(home.path(), &["rebuild"]));
    assert!(
        !vault(home.path()).join("daily").exists(),
        "o painel de hoje não vira snapshot"
    );
    assert_eq!(le(home.path(), "CURRENT.md"), PAINEL_COM_NOTAS);
}

#[test]
fn rebuild_usa_a_data_do_current_anterior_no_nome_do_daily() {
    let home = home_painel();
    let antigo = CURRENT_DE_ONTEM
        .replace("date: 2026-11-07", "date: 2026-10-30")
        .replace("weekday: Saturday", "weekday: Friday")
        .replace("07/11/26", "30/10/26");
    arquivo(home.path(), "CURRENT.md", &antigo);
    ok(&roda(home.path(), &["rebuild"]));
    assert_eq!(le(home.path(), "daily/2026-10-30.md"), antigo);
    assert!(!vault(home.path()).join("daily/2026-11-07.md").exists());
    assert_eq!(le(home.path(), "CURRENT.md"), PAINEL_COM_NOTAS);
}

#[test]
fn rebuild_nao_sobrescreve_daily_que_ja_existe() {
    let home = home_painel();
    arquivo(home.path(), "CURRENT.md", CURRENT_DE_ONTEM);
    arquivo(home.path(), "daily/2026-11-07.md", "já existia\n");
    ok(&roda(home.path(), &["rebuild"]));
    assert_eq!(le(home.path(), "daily/2026-11-07.md"), "já existia\n");
    assert_eq!(le(home.path(), "CURRENT.md"), PAINEL_COM_NOTAS);
}

#[test]
fn rebuild_mantem_o_placeholder_quando_o_anterior_nao_tinha_notas() {
    let home = home_painel();
    arquivo(
        home.path(),
        "CURRENT.md",
        &CURRENT_DE_ONTEM.replace("- lembrar de X\n- ligar para Y\n", "- \n"),
    );
    ok(&roda(home.path(), &["rebuild"]));
    assert_eq!(le(home.path(), "CURRENT.md"), PAINEL);
}

#[test]
fn rebuild_nao_toca_os_outros_daily() {
    let home = home_painel();
    arquivo(home.path(), "CURRENT.md", CURRENT_DE_ONTEM);
    arquivo(home.path(), "daily/2026-11-01.md", "# outro dia\n");
    let antes = fotografa(&vault(home.path()).join("daily"));
    ok(&roda(home.path(), &["rebuild"]));
    let depois = fotografa(&vault(home.path()).join("daily"));
    assert_eq!(
        depois.get(Path::new("2026-11-01.md")),
        antes.get(Path::new("2026-11-01.md"))
    );
    assert_eq!(depois.len(), 2, "só entrou o snapshot de ontem: {depois:?}");
}

// ---------------------------------------------------------------------------
// Leitura pura (I6, D7)
// ---------------------------------------------------------------------------

/// As leituras desta fatia, como listas de argumentos.
const LEITURAS: &[&[&str]] = &[
    &["list"],
    &["list", "--json"],
    &["list", "--archived"],
    &["list", "--json", "--archived"],
    &["show", "x"],
    &["show", "x", "--json"],
    &["projects"],
    &["projects", "--json"],
    &["statuses"],
    &["statuses", "--json"],
    &["help"],
    &["version"],
    &["version", "--json"],
];

#[test]
fn leituras_nao_mudam_byte_nem_mtime_de_nada_sob_o_home() {
    let home = home_completo();
    let antes = fotografa(home.path());
    assert!(antes.contains_key(Path::new("ObsidianVault/CURRENT.md")));
    assert!(antes.contains_key(Path::new("ObsidianVault/.freitask/status.json")));
    for args in LEITURAS {
        let saida = roda(home.path(), args);
        assert_eq!(saida.codigo, Some(0), "freitask {args:?}: {}", saida.stderr);
        assert_eq!(saida.stderr, "", "freitask {args:?}");
        assert_eq!(
            fotografa(home.path()),
            antes,
            "freitask {args:?} mudou algo"
        );
    }
}

#[test]
fn leituras_num_vault_vazio_nao_criam_projects_nem_status_json() {
    let home = home_com_vault_vazio();
    for args in LEITURAS {
        if args[0] == "show" {
            continue; // não há task para mostrar; o caso dele está abaixo
        }
        let saida = roda(home.path(), args);
        assert_eq!(saida.codigo, Some(0), "freitask {args:?}: {}", saida.stderr);
        assert_eq!(
            entradas(&vault(home.path())),
            Vec::<String>::new(),
            "freitask {args:?} criou algo em ObsidianVault/"
        );
    }
    assert!(!vault(home.path()).join("projects").exists());
    assert!(!vault(home.path()).join(".freitask/status.json").exists());
}

#[test]
fn show_recusado_num_vault_vazio_tambem_nao_cria_nada() {
    let home = home_com_vault_vazio();
    let saida = roda(home.path(), &["show", "x"]);
    assert_eq!(saida.codigo, Some(1));
    assert_eq!(saida.stderr, "freitask: task não encontrada: x\n");
    assert_eq!(entradas(&vault(home.path())), Vec::<String>::new());
}

#[test]
fn leituras_num_home_sem_vault_saem_0_e_nao_criam_o_vault() {
    let home = home_sem_vault();
    for args in LEITURAS {
        if args[0] == "show" {
            continue;
        }
        let saida = roda(home.path(), args);
        assert_eq!(saida.codigo, Some(0), "freitask {args:?}: {}", saida.stderr);
        assert_eq!(saida.stderr, "", "freitask {args:?}");
    }
    assert!(
        !vault(home.path()).exists(),
        "ObsidianVault/ não foi criado"
    );
    assert_eq!(entradas(home.path()), Vec::<String>::new());
}

#[test]
fn list_json_e_projects_json_num_home_sem_vault_sao_arrays_vazios() {
    let home = home_sem_vault();
    let lista = roda(home.path(), &["list", "--json"]);
    ok(&lista);
    assert_eq!(json(&lista), json!([]));
    let projetos = roda(home.path(), &["projects", "--json"]);
    ok(&projetos);
    assert_eq!(json(&projetos), json!([]));
    assert!(!vault(home.path()).exists());
}

#[test]
fn list_nao_regenera_o_current() {
    // Só `rebuild` (e as escritas) regeneram o painel; o `list` não.
    let home = home_completo();
    let antes = le(home.path(), "CURRENT.md");
    ok(&roda(home.path(), &["list", "--json"]));
    ok(&roda(home.path(), &["list"]));
    assert_eq!(le(home.path(), "CURRENT.md"), antes);
    assert!(!vault(home.path()).join("daily/2026-11-07.md").exists());
}

// ---------------------------------------------------------------------------
// `freitask help`, sem argumentos, comando desconhecido
// ---------------------------------------------------------------------------

const PRIMEIRA_LINHA_DO_USAGE: &str = "freitask — gestão de tasks do vault do Obsidian\n";

#[test]
fn help_imprime_o_usage_e_sai_0() {
    let home = home_sem_vault();
    let saida = roda(home.path(), &["help"]);
    ok(&saida);
    assert!(
        saida.stdout.starts_with(PRIMEIRA_LINHA_DO_USAGE),
        "stdout:\n{}",
        saida.stdout
    );
    assert!(saida.stdout.ends_with('\n'));
}

#[test]
fn help_lista_cada_subcomando_da_fatia() {
    let home = home_sem_vault();
    let saida = roda(home.path(), &["help"]);
    ok(&saida);
    for trecho in [
        "freitask list [--json] [--archived]",
        "freitask projects [--json]",
        "freitask statuses [--json]",
        "freitask rebuild",
    ] {
        assert!(saida.stdout.contains(trecho), "falta {trecho:?} no usage");
    }
}

#[test]
fn sem_argumentos_fora_de_terminal_imprime_o_usage_em_stdout_e_sai_2() {
    let home = home_sem_vault();
    let ajuda = roda(home.path(), &["help"]);
    ok(&ajuda);
    // `Command::output()` não herda o stdin: o processo não tem terminal.
    let saida = roda(home.path(), &[]);
    assert_eq!(saida.codigo, Some(2));
    assert_eq!(saida.stdout, ajuda.stdout);
}

#[test]
fn sem_argumentos_fora_de_terminal_nao_cria_nada() {
    let home = home_sem_vault();
    let saida = roda(home.path(), &[]);
    assert_eq!(saida.codigo, Some(2));
    assert!(!vault(home.path()).exists());
    assert_eq!(entradas(home.path()), Vec::<String>::new());
}

#[test]
fn comando_desconhecido_sai_2_e_diz_qual_e() {
    let home = home_sem_vault();
    let saida = roda(home.path(), &["comando-que-nao-existe"]);
    assert_eq!(saida.codigo, Some(2));
    assert!(
        saida
            .stderr
            .starts_with("freitask: comando desconhecido: comando-que-nao-existe"),
        "stderr:\n{}",
        saida.stderr
    );
}

#[test]
fn comando_desconhecido_nao_cria_nada() {
    let home = home_com_vault_vazio();
    let saida = roda(home.path(), &["comando-que-nao-existe"]);
    assert_eq!(saida.codigo, Some(2));
    assert_eq!(entradas(&vault(home.path())), Vec::<String>::new());
}

// ---------------------------------------------------------------------------
// JSON Schema (spec §3.3): `<FREITASK_ACEITE_RAIZ>/schema/<comando>.json`
// ---------------------------------------------------------------------------

fn aceite_raiz() -> String {
    std::env::var("FREITASK_ACEITE_RAIZ")
        .unwrap_or_else(|_| format!("{}/tests/aceite", raiz_do_repo()))
}

fn schema(comando: &str) -> Value {
    let caminho = format!("{}/schema/{comando}.json", aceite_raiz());
    let texto = fs::read_to_string(&caminho)
        .unwrap_or_else(|e| panic!("não consegui ler o schema {caminho}: {e}"));
    serde_json::from_str(&texto).unwrap_or_else(|e| panic!("schema {caminho} não é JSON: {e}"))
}

/// Valida `instancia` contra o schema do comando; pânico com a lista de
/// erros quando não passa.
fn valida(comando: &str, instancia: &Value) {
    let esquema = schema(comando);
    assert_eq!(
        esquema["$schema"],
        json!("https://json-schema.org/draft/2020-12/schema"),
        "schema/{comando}.json tem de ser draft 2020-12"
    );
    let validador = jsonschema::validator_for(&esquema)
        .unwrap_or_else(|e| panic!("schema/{comando}.json inválido: {e}"));
    let erros: Vec<String> = validador
        .iter_errors(instancia)
        .map(|e| format!("{} em {}", e, e.instance_path()))
        .collect();
    assert!(
        erros.is_empty(),
        "a saída de `{comando}` não passa em schema/{comando}.json:\n{}\n\nsaída:\n{instancia}",
        erros.join("\n")
    );
}

#[test]
fn schema_list_valida_a_saida_de_list_json() {
    let home = home_completo();
    let saida = roda(home.path(), &["list", "--json"]);
    ok(&saida);
    let v = json(&saida);
    assert!(
        v.as_array().expect("array").len() >= 4,
        "o vault completo tem tasks"
    );
    valida("list", &v);
}

#[test]
fn schema_list_valida_a_saida_de_list_json_archived() {
    let home = home_completo();
    let saida = roda(home.path(), &["list", "--json", "--archived"]);
    ok(&saida);
    let v = json(&saida);
    assert!(
        v.as_array().expect("array").len() >= 7,
        "ativas e arquivadas"
    );
    valida("list", &v);
}

#[test]
fn schema_list_valida_o_array_vazio() {
    let home = home_sem_vault();
    let saida = roda(home.path(), &["list", "--json"]);
    ok(&saida);
    valida("list", &json(&saida));
}

#[test]
fn schema_list_recusa_linha_com_chave_a_mais_ou_a_menos() {
    // O schema é fechado: `additionalProperties: false` e `required` com as 21.
    let home = home_com(&[(
        "projects/p/tasks/t1.md",
        &bloco("todo", "T", "p", "t1", &[]),
    )]);
    let esquema = schema("list");
    let validador = jsonschema::validator_for(&esquema).expect("schema válido");
    let certa = Linha::ativa("p", "t1", "todo", 1, "Não iniciada", "T").json(home.path());
    assert!(validador.is_valid(&json!([certa])));

    let mut a_mais = certa.clone();
    a_mais["extra"] = json!(1);
    assert!(
        !validador.is_valid(&json!([a_mais])),
        "chave a mais tem de falhar"
    );

    let mut a_menos = certa.clone();
    a_menos.as_object_mut().expect("objeto").remove("checklist");
    assert!(
        !validador.is_valid(&json!([a_menos])),
        "chave a menos tem de falhar"
    );

    let mut tipo_errado = certa;
    tipo_errado["status_num"] = json!("1");
    assert!(
        !validador.is_valid(&json!([tipo_errado])),
        "tipo errado tem de falhar"
    );
}

#[test]
fn schema_show_valida_a_saida_de_show_json() {
    let home = home_completo();
    for id in ["x", "escondida", "adiada", "feita", "y", "z"] {
        let saida = roda(home.path(), &["show", id, "--json"]);
        ok(&saida);
        valida("show", &json(&saida));
    }
}

#[test]
fn schema_show_recusa_objeto_sem_ciclo() {
    let home = home_com(&[(
        "projects/p/tasks/t1.md",
        &bloco("todo", "T", "p", "t1", &[]),
    )]);
    let esquema = schema("show");
    let validador = jsonschema::validator_for(&esquema).expect("schema válido");
    let certa = Linha::ativa("p", "t1", "todo", 1, "Não iniciada", "T");
    assert!(validador.is_valid(&certa.show_json(home.path())));
    assert!(
        !validador.is_valid(&certa.json(home.path())),
        "sem `ciclo` tem de falhar"
    );
}

#[test]
fn schema_projects_valida_a_saida_de_projects_json() {
    let home = home_completo();
    let saida = roda(home.path(), &["projects", "--json"]);
    ok(&saida);
    let v = json(&saida);
    assert!(!v.as_array().expect("array").is_empty());
    valida("projects", &v);
    let vazio = roda(home_sem_vault().path(), &["projects", "--json"]);
    ok(&vazio);
    valida("projects", &json(&vazio));
}

#[test]
fn schema_statuses_valida_a_saida_de_statuses_json() {
    let home = home_completo();
    let saida = roda(home.path(), &["statuses", "--json"]);
    ok(&saida);
    valida("statuses", &json(&saida));
    let proprio = home_com(&[(".freitask/status.json", STATUS_JSON_PROPRIO)]);
    let saida = roda(proprio.path(), &["statuses", "--json"]);
    ok(&saida);
    valida("statuses", &json(&saida));
}

#[test]
fn schema_version_valida_a_saida_de_version_json() {
    let home = home_sem_vault();
    let saida = roda(home.path(), &["version", "--json"]);
    ok(&saida);
    valida("version", &json(&saida));
}
