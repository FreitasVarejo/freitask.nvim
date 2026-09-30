//! O estado da tela e o que cada tecla faz.
//!
//! Puro de propósito: `handle_key` não toca disco nem processo, só devolve um
//! `Effect` que o laço de `main.rs` executa. É o que deixa o comportamento das
//! teclas testável sem terminal — e mantém num lugar só a fronteira com o
//! motor, que é quem decide de verdade (a TUI só monta os argumentos).

use crate::engine::{Data, Project, Status, Task};
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

/// Destinos de arquivamento e a glosa de cada um. Espelha `ARCHIVED_TYPES` do
/// motor só para montar a lista; quem valida é o `freitask archive`.
pub const ARCHIVE_TYPES: [(&str, &str); 3] =
    [("done", "done — feito"), ("dropped", "dropped — abandonado"), ("failed", "failed — falhou")];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pane {
    Projects,
    Tasks,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PromptKind {
    Title,
    Desc,
    Rename,
    NewProject,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PickKind {
    Status,
    Archive,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfirmKind {
    Delete,
    Unarchive,
}

/// O modal de task nova: título → id → descrição.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct NewTask {
    pub title: String,
    pub id: String,
    pub desc: String,
    /// 0 título, 1 id, 2 descrição.
    pub field: usize,
    /// O id foi digitado à mão: a sugestão derivada do título não o sobrescreve.
    pub id_touched: bool,
}

impl NewTask {
    fn field_mut(&mut self) -> &mut String {
        match self.field {
            0 => &mut self.title,
            1 => &mut self.id,
            _ => &mut self.desc,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Mode {
    Normal,
    Filter,
    NewTask(NewTask),
    Prompt { kind: PromptKind, value: String },
    Pick { kind: PickKind, options: Vec<(String, String)>, idx: usize },
    Confirm { kind: ConfirmKind },
    Help,
}

/// O que selecionar depois que a CLI rodou e os dados foram relidos.
#[derive(Debug, Clone, PartialEq)]
pub enum After {
    /// A mesma task de antes (pelo id), ou a mesma posição se ela sumiu.
    Keep,
    Task { project: String, id: String },
    /// O stdout da CLI é o id do projeto (é o que `new-project` imprime).
    ProjectFromOutput,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Effect {
    None,
    Quit,
    /// Roda `freitask <args>`; em sucesso relê tudo e mostra `ok`.
    Run { args: Vec<String>, after: After, ok: String },
    Open(String),
    /// Regenera e abre o CURRENT.md (o caminho vem do `freitask rebuild`).
    OpenCurrent,
    Reload,
    /// Pede ao motor o id derivado deste título.
    Kebab(String),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Message {
    pub text: String,
    pub error: bool,
}

pub struct App {
    pub data: Data,
    pub pane: Pane,
    pub project_idx: usize,
    pub task_idx: usize,
    pub show_archived: bool,
    pub filter: String,
    pub mode: Mode,
    pub message: Option<Message>,
    /// (caminho, conteúdo) do arquivo em preview.
    pub preview: Option<(String, String)>,
    pub preview_scroll: u16,
    /// O modal que originou o Run em voo: se a CLI recusar, ele volta com o
    /// que foi digitado, em vez de obrigar a digitar tudo de novo.
    retry: Option<Mode>,
}

fn args(v: &[&str]) -> Vec<String> {
    v.iter().map(|s| s.to_string()).collect()
}

impl App {
    pub fn new(data: Data) -> Self {
        App {
            data,
            pane: Pane::Projects,
            project_idx: 0,
            task_idx: 0,
            show_archived: false,
            filter: String::new(),
            mode: Mode::Normal,
            message: None,
            preview: None,
            preview_scroll: 0,
            retry: None,
        }
    }

    // --- leitura --------------------------------------------------------------

    pub fn current_project(&self) -> Option<&Project> {
        self.data.projects.get(self.project_idx)
    }

    /// As tasks do projeto selecionado, na ordem do `list` (ativas por fase e
    /// id, arquivadas no fim), filtradas pelo toggle de arquivadas e por `/`.
    pub fn visible_tasks(&self) -> Vec<&Task> {
        let Some(p) = self.current_project() else {
            return vec![];
        };
        let f = self.filter.to_lowercase();
        self.data
            .tasks
            .iter()
            .filter(|t| t.project == p.project)
            .filter(|t| self.show_archived || t.archived.is_none())
            .filter(|t| f.is_empty() || t.id.to_lowercase().contains(&f) || t.title.to_lowercase().contains(&f))
            .collect()
    }

    pub fn current_task(&self) -> Option<&Task> {
        self.visible_tasks().get(self.task_idx).copied()
    }

    pub fn status_of(&self, num: i64) -> Option<&Status> {
        self.data.statuses.iter().find(|s| s.num == num)
    }

    /// O caminho que o preview deveria mostrar, se ainda não mostra.
    pub fn preview_wanted(&self) -> Option<String> {
        let t = self.current_task()?;
        match &self.preview {
            Some((p, _)) if *p == t.path => None,
            _ => Some(t.path.clone()),
        }
    }

    // --- escrita vinda do laço -------------------------------------------------

    pub fn set_preview(&mut self, path: String, content: String) {
        self.preview = Some((path, content));
    }

    pub fn set_data(&mut self, data: Data, after: &After, output: &str) {
        let keep_project = self.current_project().map(|p| p.project.clone());
        let keep_task = self.current_task().map(|t| t.id.clone());
        let old_task_idx = self.task_idx;
        self.data = data;
        self.preview = None;

        let (want_project, want_task) = match after {
            After::Keep => (keep_project, keep_task),
            After::Task { project, id } => (Some(project.clone()), Some(id.clone())),
            After::ProjectFromOutput => (Some(output.trim().to_string()), None),
        };
        if let Some(i) = want_project.and_then(|w| self.data.projects.iter().position(|p| p.project == w)) {
            self.project_idx = i;
        }
        self.task_idx = want_task
            .and_then(|id| self.visible_tasks().iter().position(|t| t.id == id))
            .unwrap_or(old_task_idx);
        if matches!(after, After::ProjectFromOutput) {
            self.task_idx = 0;
            self.pane = Pane::Projects;
        }
        self.clamp();
    }

    pub fn suggest_id(&mut self, id: String) {
        if let Mode::NewTask(nt) = &mut self.mode
            && !nt.id_touched
        {
            nt.id = id;
        }
    }

    pub fn run_ok(&mut self, ok: String) {
        self.retry = None;
        self.info(ok);
    }

    pub fn run_failed(&mut self, err: String) {
        if let Some(m) = self.retry.take() {
            self.mode = m;
        }
        self.error(err);
    }

    pub fn info(&mut self, text: impl Into<String>) {
        self.message = Some(Message { text: text.into(), error: false });
    }

    pub fn error(&mut self, text: impl Into<String>) {
        self.message = Some(Message { text: text.into(), error: true });
    }

    fn clamp(&mut self) {
        self.project_idx = self.project_idx.min(self.data.projects.len().saturating_sub(1));
        self.task_idx = self.task_idx.min(self.visible_tasks().len().saturating_sub(1));
    }

    // --- teclas ---------------------------------------------------------------

    pub fn handle_key(&mut self, key: KeyEvent) -> Effect {
        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
            return Effect::Quit;
        }
        match std::mem::replace(&mut self.mode, Mode::Normal) {
            Mode::Normal => self.on_normal(key),
            Mode::Filter => self.on_filter(key),
            Mode::NewTask(nt) => self.on_new_task(nt, key),
            Mode::Prompt { kind, value } => self.on_prompt(kind, value, key),
            Mode::Pick { kind, options, idx } => self.on_pick(kind, options, idx, key),
            Mode::Confirm { kind } => self.on_confirm(kind, key),
            Mode::Help => Effect::None,
        }
    }

    fn on_normal(&mut self, key: KeyEvent) -> Effect {
        let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
        match key.code {
            KeyCode::Char('d') if ctrl => self.preview_scroll = self.preview_scroll.saturating_add(10),
            KeyCode::Char('u') if ctrl => self.preview_scroll = self.preview_scroll.saturating_sub(10),
            _ if ctrl => {}
            KeyCode::Char('q') => return Effect::Quit,
            KeyCode::Esc => {
                self.filter.clear();
                self.message = None;
                self.clamp();
            }
            KeyCode::Char('?') => self.mode = Mode::Help,
            KeyCode::Char('j') | KeyCode::Down => self.step(1),
            KeyCode::Char('k') | KeyCode::Up => self.step(-1),
            KeyCode::Char('g') | KeyCode::Home => self.step(isize::MIN),
            KeyCode::Char('G') | KeyCode::End => self.step(isize::MAX),
            KeyCode::Char('h') | KeyCode::Left => self.pane = Pane::Projects,
            KeyCode::Char('l') | KeyCode::Right => self.pane = Pane::Tasks,
            KeyCode::Tab | KeyCode::BackTab => {
                self.pane = match self.pane {
                    Pane::Projects => Pane::Tasks,
                    Pane::Tasks => Pane::Projects,
                }
            }
            KeyCode::Enter if self.pane == Pane::Projects => self.pane = Pane::Tasks,
            KeyCode::Enter | KeyCode::Char('e') => return self.open_current_task(),
            KeyCode::Char('/') => {
                self.pane = Pane::Tasks;
                self.mode = Mode::Filter;
            }
            KeyCode::Char('n') => {
                if self.current_project().is_some() {
                    self.mode = Mode::NewTask(NewTask::default());
                } else {
                    self.error("crie um projeto antes (N)");
                }
            }
            KeyCode::Char('N') => self.mode = Mode::Prompt { kind: PromptKind::NewProject, value: String::new() },
            KeyCode::Char('A') => {
                self.show_archived = !self.show_archived;
                self.preview = None;
                self.clamp();
            }
            KeyCode::Char('c') => return Effect::OpenCurrent,
            KeyCode::Char('R') => return Effect::Reload,
            KeyCode::Char(c @ ('d' | 'a' | 's' | 'r' | 't' | 'i')) => self.on_task_key(c),
            _ => {}
        }
        Effect::None
    }

    /// Teclas que agem sobre a task selecionada: abrem o modal certo.
    fn on_task_key(&mut self, c: char) {
        let Some(t) = self.current_task().cloned() else {
            self.error("nenhuma task selecionada");
            return;
        };
        self.mode = match c {
            'd' => Mode::Confirm { kind: ConfirmKind::Delete },
            'a' if t.archived.is_some() => Mode::Confirm { kind: ConfirmKind::Unarchive },
            'a' => Mode::Pick {
                kind: PickKind::Archive,
                options: ARCHIVE_TYPES.iter().map(|(v, l)| (v.to_string(), l.to_string())).collect(),
                idx: ARCHIVE_TYPES.iter().position(|(v, _)| *v == t.suggested_archive).unwrap_or(1),
            },
            's' => {
                // `done` e o sentinela 0 ficam fora: o primeiro é o que `archive`
                // grava, o segundo não é fase que se escolhe. O motor recusaria
                // os dois de qualquer jeito; aqui só não se oferece o que falha.
                let options: Vec<(String, String)> = self
                    .data
                    .statuses
                    .iter()
                    .filter(|s| s.num > 0 && s.callout != "done")
                    .map(|s| (s.callout.clone(), format!("{} {}", s.icon, s.title)))
                    .collect();
                let idx = options.iter().position(|(c, _)| *c == t.callout).unwrap_or(0);
                Mode::Pick { kind: PickKind::Status, options, idx }
            }
            'r' => Mode::Prompt { kind: PromptKind::Rename, value: t.id },
            't' => Mode::Prompt { kind: PromptKind::Title, value: t.title },
            'i' => Mode::Prompt { kind: PromptKind::Desc, value: t.desc },
            _ => Mode::Normal,
        };
    }

    fn open_current_task(&mut self) -> Effect {
        match self.current_task() {
            Some(t) => Effect::Open(t.path.clone()),
            None => Effect::None,
        }
    }

    fn step(&mut self, delta: isize) {
        let (idx, len) = match self.pane {
            Pane::Projects => (self.project_idx, self.data.projects.len()),
            Pane::Tasks => (self.task_idx, self.visible_tasks().len()),
        };
        if len == 0 {
            return;
        }
        let next = (idx as isize).saturating_add(delta).clamp(0, len as isize - 1) as usize;
        match self.pane {
            Pane::Projects => {
                if next != self.project_idx {
                    self.project_idx = next;
                    self.task_idx = 0;
                }
            }
            Pane::Tasks => self.task_idx = next,
        }
        self.preview_scroll = 0;
    }

    fn on_filter(&mut self, key: KeyEvent) -> Effect {
        match key.code {
            KeyCode::Esc => {
                self.filter.clear();
                self.task_idx = 0;
            }
            KeyCode::Enter => {}
            KeyCode::Down => {
                self.step(1);
                self.mode = Mode::Filter;
            }
            KeyCode::Up => {
                self.step(-1);
                self.mode = Mode::Filter;
            }
            KeyCode::Backspace => {
                self.filter.pop();
                self.task_idx = 0;
                self.mode = Mode::Filter;
            }
            KeyCode::Char(c) => {
                self.filter.push(c);
                self.task_idx = 0;
                self.mode = Mode::Filter;
            }
            _ => self.mode = Mode::Filter,
        }
        Effect::None
    }

    fn on_new_task(&mut self, mut nt: NewTask, key: KeyEvent) -> Effect {
        let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
        let mut effect = Effect::None;
        match key.code {
            KeyCode::Esc => return Effect::None,
            KeyCode::Enter if nt.field == 2 => return self.submit_new_task(nt),
            KeyCode::Enter | KeyCode::Tab | KeyCode::Down => {
                let from = nt.field;
                nt.field = (nt.field + 1) % 3;
                if from == 0 && !nt.id_touched && !nt.title.trim().is_empty() {
                    effect = Effect::Kebab(nt.title.trim().to_string());
                }
            }
            KeyCode::BackTab | KeyCode::Up => nt.field = (nt.field + 2) % 3,
            KeyCode::Char('u') if ctrl => {
                nt.field_mut().clear();
                if nt.field == 1 {
                    nt.id_touched = false;
                }
            }
            KeyCode::Backspace => {
                nt.field_mut().pop();
                if nt.field == 1 {
                    nt.id_touched = !nt.id.is_empty();
                }
            }
            KeyCode::Char(c) if !ctrl => {
                nt.field_mut().push(c);
                if nt.field == 1 {
                    nt.id_touched = true;
                }
            }
            _ => {}
        }
        self.mode = Mode::NewTask(nt);
        effect
    }

    fn submit_new_task(&mut self, mut nt: NewTask) -> Effect {
        let Some(project) = self.current_project().map(|p| p.project.clone()) else {
            return Effect::None;
        };
        let (title, id, desc) = (nt.title.trim().to_string(), nt.id.trim().to_string(), nt.desc.trim().to_string());
        if title.is_empty() || id.is_empty() {
            nt.field = if title.is_empty() { 0 } else { 1 };
            self.error(if title.is_empty() { "falta o título" } else { "falta o id" });
            self.mode = Mode::NewTask(nt);
            return Effect::None;
        }
        let mut a = args(&["new", &project, &id, &title]);
        if !desc.is_empty() {
            a.extend(args(&["--desc", &desc]));
        }
        self.retry = Some(Mode::NewTask(nt));
        Effect::Run { args: a, after: After::Task { project, id: id.clone() }, ok: format!("criada: {id}") }
    }

    fn on_prompt(&mut self, kind: PromptKind, mut value: String, key: KeyEvent) -> Effect {
        let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
        match key.code {
            KeyCode::Esc => return Effect::None,
            KeyCode::Enter => return self.submit_prompt(kind, value),
            KeyCode::Char('u') if ctrl => value.clear(),
            KeyCode::Backspace => {
                value.pop();
            }
            KeyCode::Char(c) if !ctrl => value.push(c),
            _ => {}
        }
        self.mode = Mode::Prompt { kind, value };
        Effect::None
    }

    fn submit_prompt(&mut self, kind: PromptKind, value: String) -> Effect {
        let v = value.trim().to_string();
        if v.is_empty() && kind != PromptKind::Desc {
            self.error("não pode ficar vazio");
            self.mode = Mode::Prompt { kind, value };
            return Effect::None;
        }
        if kind == PromptKind::NewProject {
            self.retry = Some(Mode::Prompt { kind, value });
            return Effect::Run {
                args: args(&["new-project", &v]),
                after: After::ProjectFromOutput,
                ok: "projeto criado".into(),
            };
        }
        let Some(t) = self.current_task().cloned() else {
            return Effect::None;
        };
        let (a, after, ok) = match kind {
            PromptKind::Title if v == t.title => return Effect::None,
            PromptKind::Desc if v == t.desc => return Effect::None,
            PromptKind::Rename if v == t.id => return Effect::None,
            PromptKind::Title => (args(&["set", &t.path, "--title", &v]), After::Keep, "título atualizado".to_string()),
            PromptKind::Desc => (args(&["set", &t.path, "--desc", &v]), After::Keep, "descrição atualizada".to_string()),
            PromptKind::Rename => (
                args(&["rename", &t.path, &v]),
                After::Task { project: t.project.clone(), id: v.clone() },
                format!("renomeada: {} → {v}", t.id),
            ),
            PromptKind::NewProject => unreachable!(),
        };
        self.retry = Some(Mode::Prompt { kind, value });
        Effect::Run { args: a, after, ok }
    }

    fn on_pick(&mut self, kind: PickKind, options: Vec<(String, String)>, mut idx: usize, key: KeyEvent) -> Effect {
        match key.code {
            KeyCode::Esc | KeyCode::Char('q') => return Effect::None,
            KeyCode::Char('j') | KeyCode::Down => idx = (idx + 1).min(options.len().saturating_sub(1)),
            KeyCode::Char('k') | KeyCode::Up => idx = idx.saturating_sub(1),
            KeyCode::Enter => {
                let (Some(t), Some((value, label))) = (self.current_task().cloned(), options.get(idx)) else {
                    return Effect::None;
                };
                return match kind {
                    PickKind::Status if *value == t.callout => Effect::None,
                    PickKind::Status => Effect::Run {
                        args: args(&["set", &t.path, "--status", value]),
                        after: After::Keep,
                        ok: format!("fase: {label}"),
                    },
                    PickKind::Archive => Effect::Run {
                        args: args(&["archive", &t.path, value]),
                        after: After::Keep,
                        ok: format!("arquivada como {value}: {}", t.id),
                    },
                };
            }
            _ => {}
        }
        self.mode = Mode::Pick { kind, options, idx };
        Effect::None
    }

    fn on_confirm(&mut self, kind: ConfirmKind, key: KeyEvent) -> Effect {
        if !matches!(key.code, KeyCode::Char('y') | KeyCode::Char('Y')) {
            return Effect::None;
        }
        let Some(t) = self.current_task().cloned() else {
            return Effect::None;
        };
        match kind {
            ConfirmKind::Delete => Effect::Run {
                args: args(&["delete", &t.path]),
                after: After::Keep,
                ok: format!("apagada: {}", t.id),
            },
            ConfirmKind::Unarchive => Effect::Run {
                args: args(&["unarchive", &t.path]),
                after: After::Task { project: t.project.clone(), id: t.id.clone() },
                ok: format!("desarquivada: {}", t.id),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn task(project: &str, id: &str, callout: &str, num: i64, archived: Option<&str>) -> Task {
        let path = match archived {
            Some(a) => format!("/v/projects/{project}/tasks/archived/{a}/{id}.md"),
            None => format!("/v/projects/{project}/tasks/{id}.md"),
        };
        Task {
            project: project.into(),
            id: id.into(),
            status: String::new(),
            status_num: num,
            callout: callout.into(),
            title: format!("Título {id}"),
            desc: String::new(),
            archived: archived.map(Into::into),
            suggested_archive: if callout == "check" { "done" } else { "dropped" }.into(),
            path,
        }
    }

    fn status(num: i64, callout: &str) -> Status {
        Status { num, callout: callout.into(), title: callout.into(), icon: "*".into(), hl_group: String::new() }
    }

    fn data() -> Data {
        Data {
            statuses: vec![
                status(0, "invalid"),
                status(1, "todo"),
                status(3, "example"),
                status(6, "check"),
                status(7, "done"),
            ],
            projects: vec![Project { project: "alfa".into(), active: 2 }, Project { project: "beta".into(), active: 1 }],
            tasks: vec![
                task("alfa", "a1", "todo", 1, None),
                task("alfa", "a2", "check", 6, None),
                task("alfa", "a0", "done", 7, Some("done")),
                task("beta", "b1", "example", 3, None),
            ],
        }
    }

    fn key(c: char) -> KeyEvent {
        KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE)
    }

    fn code(k: KeyCode) -> KeyEvent {
        KeyEvent::new(k, KeyModifiers::NONE)
    }

    fn typ(app: &mut App, s: &str) {
        for c in s.chars() {
            app.handle_key(key(c));
        }
    }

    fn run_args(e: Effect) -> Vec<String> {
        match e {
            Effect::Run { args, .. } => args,
            other => panic!("esperava Run, veio {other:?}"),
        }
    }

    #[test]
    fn arquivadas_ficam_escondidas_ate_o_toggle() {
        let mut app = App::new(data());
        assert_eq!(app.visible_tasks().len(), 2);
        app.handle_key(key('A'));
        assert_eq!(app.visible_tasks().len(), 3);
    }

    #[test]
    fn trocar_de_projeto_volta_a_task_para_o_topo() {
        let mut app = App::new(data());
        app.pane = Pane::Tasks;
        app.handle_key(key('j'));
        assert_eq!(app.current_task().unwrap().id, "a2");
        app.handle_key(key('h'));
        app.handle_key(key('j'));
        assert_eq!(app.current_project().unwrap().project, "beta");
        assert_eq!(app.current_task().unwrap().id, "b1");
    }

    #[test]
    fn filtro_casa_id_e_titulo_e_esc_limpa() {
        let mut app = App::new(data());
        app.handle_key(key('/'));
        typ(&mut app, "a2");
        assert_eq!(app.visible_tasks().len(), 1);
        app.handle_key(code(KeyCode::Enter));
        assert_eq!(app.mode, Mode::Normal);
        assert_eq!(app.filter, "a2");
        app.handle_key(code(KeyCode::Esc));
        assert_eq!(app.visible_tasks().len(), 2);
    }

    #[test]
    fn nova_task_pede_o_id_ao_motor_e_monta_o_new() {
        let mut app = App::new(data());
        app.handle_key(key('n'));
        typ(&mut app, "Ávida nova");
        assert_eq!(app.handle_key(code(KeyCode::Tab)), Effect::Kebab("Ávida nova".into()));
        app.suggest_id("avida-nova".into());
        app.handle_key(code(KeyCode::Tab));
        typ(&mut app, "estado");
        let e = app.handle_key(code(KeyCode::Enter));
        assert_eq!(
            e,
            Effect::Run {
                args: args(&["new", "alfa", "avida-nova", "Ávida nova", "--desc", "estado"]),
                after: After::Task { project: "alfa".into(), id: "avida-nova".into() },
                ok: "criada: avida-nova".into(),
            }
        );
    }

    #[test]
    fn id_digitado_a_mao_nao_e_sobrescrito_pela_sugestao() {
        let mut app = App::new(data());
        app.handle_key(key('n'));
        app.handle_key(code(KeyCode::Tab)); // título vazio: nada a sugerir
        typ(&mut app, "meu-id");
        app.suggest_id("outro".into());
        let Mode::NewTask(nt) = &app.mode else { panic!() };
        assert_eq!(nt.id, "meu-id");
    }

    #[test]
    fn recusa_do_motor_devolve_o_modal_preenchido() {
        let mut app = App::new(data());
        app.handle_key(key('n'));
        typ(&mut app, "X");
        app.handle_key(code(KeyCode::Tab));
        typ(&mut app, "Id Ruim");
        app.handle_key(code(KeyCode::Tab));
        run_args(app.handle_key(code(KeyCode::Enter)));
        app.run_failed("id precisa ser kebab-case".into());
        let Mode::NewTask(nt) = &app.mode else { panic!("modal não voltou") };
        assert_eq!(nt.id, "Id Ruim");
        assert!(app.message.as_ref().unwrap().error);
    }

    #[test]
    fn fase_oferece_so_o_que_o_set_aceita() {
        let mut app = App::new(data());
        app.handle_key(key('s'));
        let Mode::Pick { options, idx, .. } = &app.mode else { panic!() };
        let callouts: Vec<&str> = options.iter().map(|(c, _)| c.as_str()).collect();
        assert_eq!(callouts, ["todo", "example", "check"]);
        assert_eq!(*idx, 0); // a1 está em todo
        app.handle_key(key('j'));
        assert_eq!(
            run_args(app.handle_key(code(KeyCode::Enter))),
            args(&["set", "/v/projects/alfa/tasks/a1.md", "--status", "example"])
        );
    }

    #[test]
    fn arquivar_parte_da_sugestao_do_motor() {
        let mut app = App::new(data());
        app.pane = Pane::Tasks;
        app.handle_key(key('j')); // a2, em check → sugere done
        app.handle_key(key('a'));
        let Mode::Pick { idx, .. } = &app.mode else { panic!() };
        assert_eq!(*idx, 0);
        assert_eq!(
            run_args(app.handle_key(code(KeyCode::Enter))),
            args(&["archive", "/v/projects/alfa/tasks/a2.md", "done"])
        );
    }

    #[test]
    fn a_numa_arquivada_confirma_e_desarquiva() {
        let mut app = App::new(data());
        app.handle_key(key('A'));
        app.pane = Pane::Tasks;
        app.handle_key(key('G'));
        assert_eq!(app.current_task().unwrap().id, "a0");
        app.handle_key(key('a'));
        assert_eq!(app.mode, Mode::Confirm { kind: ConfirmKind::Unarchive });
        assert_eq!(
            run_args(app.handle_key(key('y'))),
            args(&["unarchive", "/v/projects/alfa/tasks/archived/done/a0.md"])
        );
    }

    #[test]
    fn apagar_so_com_y() {
        let mut app = App::new(data());
        app.handle_key(key('d'));
        assert_eq!(app.handle_key(key('n')), Effect::None);
        assert_eq!(app.mode, Mode::Normal);
        app.handle_key(key('d'));
        assert_eq!(run_args(app.handle_key(key('y'))), args(&["delete", "/v/projects/alfa/tasks/a1.md"]));
    }

    #[test]
    fn prompt_sem_mudanca_nao_chama_o_motor() {
        let mut app = App::new(data());
        app.handle_key(key('t'));
        assert_eq!(app.handle_key(code(KeyCode::Enter)), Effect::None);
        app.handle_key(key('r'));
        app.handle_key(code(KeyCode::Backspace));
        typ(&mut app, "9");
        assert_eq!(run_args(app.handle_key(code(KeyCode::Enter))), args(&["rename", "/v/projects/alfa/tasks/a1.md", "a9"]));
    }

    #[test]
    fn descricao_vazia_e_permitida_titulo_vazio_nao() {
        let mut app = App::new(data());
        app.handle_key(key('i'));
        // a descrição já é vazia: nada muda
        assert_eq!(app.handle_key(code(KeyCode::Enter)), Effect::None);
        app.handle_key(key('t'));
        app.handle_key(KeyEvent::new(KeyCode::Char('u'), KeyModifiers::CONTROL));
        assert_eq!(app.handle_key(code(KeyCode::Enter)), Effect::None);
        assert!(matches!(app.mode, Mode::Prompt { kind: PromptKind::Title, .. }));
    }

    #[test]
    fn releitura_mantem_a_task_pelo_id() {
        let mut app = App::new(data());
        app.pane = Pane::Tasks;
        app.handle_key(key('j'));
        let mut d = data();
        d.tasks.retain(|t| t.id != "a1"); // a task de cima sumiu
        app.set_data(d, &After::Keep, "");
        assert_eq!(app.current_task().unwrap().id, "a2");
    }

    #[test]
    fn releitura_seleciona_a_task_nova() {
        let mut app = App::new(data());
        let mut d = data();
        d.tasks.push(task("beta", "b2", "todo", 1, None));
        app.set_data(d, &After::Task { project: "beta".into(), id: "b2".into() }, "");
        assert_eq!(app.current_project().unwrap().project, "beta");
        assert_eq!(app.current_task().unwrap().id, "b2");
    }

    #[test]
    fn novo_projeto_e_selecionado_pelo_stdout() {
        let mut app = App::new(data());
        let mut d = data();
        d.projects.push(Project { project: "gama".into(), active: 0 });
        app.set_data(d, &After::ProjectFromOutput, "gama\n");
        assert_eq!(app.current_project().unwrap().project, "gama");
        assert!(app.current_task().is_none());
        assert_eq!(app.handle_key(key('d')), Effect::None);
        assert!(app.message.as_ref().unwrap().error);
    }

    #[test]
    fn ctrl_c_sai_de_qualquer_modo() {
        let mut app = App::new(data());
        app.handle_key(key('n'));
        assert_eq!(app.handle_key(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL)), Effect::Quit);
    }
}
