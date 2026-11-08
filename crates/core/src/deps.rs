//! Dependências entre tasks (spec §10.1). A dependência mora só na task que
//! espera (`bloqueada-por: [a, b]`); o lado reverso (`bloqueia`) e o
//! "bloqueada" são derivados na leitura, numa função só, usada pela CLI, pelo
//! JSON e pela TUI. Semântica: bloqueadora arquivada está resolvida;
//! bloqueadora que não existe não bloqueia (vira achado do doctor);
//! bloqueadora escondida bloqueia normalmente.

use crate::index::{Entry, Indice};

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Dependencias {
    /// Ao menos uma bloqueadora ATIVA.
    pub bloqueada: bool,
    /// Os ids ativos que listam esta task em `bloqueada-por`, em ordem.
    pub bloqueia: Vec<String>,
}

/// Uma bloqueadora ativa: existe entre as tasks ativas do índice.
fn ativa(indice: &Indice, id: &str) -> bool {
    indice
        .entradas
        .iter()
        .any(|e| e.task_id == id && e.archived.is_none())
}

pub fn dependencias(indice: &Indice, e: &Entry) -> Dependencias {
    let bloqueada = e.meta.bloqueada_por.iter().any(|id| ativa(indice, id));
    let mut bloqueia: Vec<String> = indice
        .entradas
        .iter()
        .filter(|o| o.archived.is_none() && o.meta.bloqueada_por.contains(&e.task_id))
        .map(|o| o.task_id.clone())
        .collect();
    bloqueia.sort();
    bloqueia.dedup();
    Dependencias {
        bloqueada,
        bloqueia,
    }
}

/// Um ciclo que passa por `inicio`, como a lista de ids `a → b → c → a`, ou
/// `None`. Segue só bloqueadoras ativas (as arquivadas estão resolvidas).
pub fn ciclo(indice: &Indice, inicio: &str) -> Option<Vec<String>> {
    fn anda(indice: &Indice, atual: &str, alvo: &str, caminho: &mut Vec<String>) -> bool {
        let Some(e) = indice
            .entradas
            .iter()
            .find(|e| e.task_id == atual && e.archived.is_none())
        else {
            return false;
        };
        for dep in &e.meta.bloqueada_por {
            if dep == alvo {
                caminho.push(dep.clone());
                return true;
            }
            if caminho.contains(dep) || !ativa(indice, dep) {
                continue;
            }
            caminho.push(dep.clone());
            if anda(indice, dep, alvo, caminho) {
                return true;
            }
            caminho.pop();
        }
        false
    }
    let mut caminho = vec![inicio.to_string()];
    if anda(indice, inicio, inicio, &mut caminho) {
        Some(caminho)
    } else {
        None
    }
}

/// Todos os ciclos do índice, um por task que está num ciclo, sem repetir o
/// mesmo conjunto (para o `doctor`).
pub fn ciclos(indice: &Indice) -> Vec<Vec<String>> {
    let mut vistos: Vec<std::collections::BTreeSet<String>> = Vec::new();
    let mut out = Vec::new();
    for e in indice.entradas.iter().filter(|e| e.archived.is_none()) {
        if let Some(c) = ciclo(indice, &e.task_id) {
            let conjunto: std::collections::BTreeSet<String> = c.iter().cloned().collect();
            if !vistos.contains(&conjunto) {
                vistos.push(conjunto);
                out.push(c);
            }
        }
    }
    out
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use crate::index::scan_task;
    use crate::status::Vocab;

    fn idx(tasks: &[(&str, &[&str])]) -> Indice {
        let v = Vocab::default();
        let entradas = tasks
            .iter()
            .map(|(rel, deps)| {
                let fm = if deps.is_empty() {
                    String::new()
                } else {
                    format!("---\nbloqueada-por: [{}]\n---\n", deps.join(", "))
                };
                scan_task(&v, rel, format!("{fm}> [!todo] T\n> [[x]]\n").as_bytes()).unwrap()
            })
            .collect();
        Indice::new(vec!["p".into()], entradas)
    }

    #[test]
    fn bloqueada_so_por_bloqueadora_ativa_e_bloqueia_e_o_inverso() {
        let i = idx(&[
            ("projects/p/tasks/a.md", &["b", "z", "nao-existe"]),
            ("projects/p/tasks/b.md", &[]),
            ("projects/p/tasks/archived/done/z.md", &[]),
            ("projects/p/tasks/c.md", &["b"]),
        ]);
        let a = i.por_id("a")[0];
        let d = dependencias(&i, a);
        assert!(d.bloqueada);
        assert!(d.bloqueia.is_empty());
        let b = i.por_id("b")[0];
        assert_eq!(dependencias(&i, b).bloqueia, vec!["a", "c"]);
        assert!(!dependencias(&i, b).bloqueada);
        let so_arquivada = idx(&[
            ("projects/p/tasks/a.md", &["z"]),
            ("projects/p/tasks/archived/done/z.md", &[]),
        ]);
        assert!(
            !dependencias(&so_arquivada, so_arquivada.por_id("a")[0]).bloqueada,
            "arquivada está resolvida"
        );
    }

    #[test]
    fn ciclo_acha_o_caminho_e_ignora_arquivadas() {
        let i = idx(&[
            ("projects/p/tasks/a.md", &["b"]),
            ("projects/p/tasks/b.md", &["c"]),
            ("projects/p/tasks/c.md", &["a"]),
            ("projects/p/tasks/d.md", &["a"]),
        ]);
        assert_eq!(ciclo(&i, "a").unwrap(), vec!["a", "b", "c", "a"]);
        assert!(ciclo(&i, "d").is_none());
        assert_eq!(ciclos(&i).len(), 1);
        let sem = idx(&[
            ("projects/p/tasks/a.md", &["z"]),
            ("projects/p/tasks/archived/done/z.md", &["a"]),
        ]);
        assert!(ciclo(&sem, "a").is_none());
        let auto = idx(&[("projects/p/tasks/a.md", &["a"])]);
        assert_eq!(ciclo(&auto, "a").unwrap(), vec!["a", "a"]);
    }
}
