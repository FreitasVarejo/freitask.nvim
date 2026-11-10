//! Dependências entre tasks (spec §10.1). A dependência mora só na task que
//! espera (`bloqueada-por: [a, b]`); o lado reverso (`bloqueia`) e o
//! "bloqueada" são derivados na leitura, numa função só, usada pela CLI, pelo
//! JSON e pela TUI. Semântica: bloqueadora arquivada está resolvida;
//! bloqueadora que não existe não bloqueia (vira achado do doctor);
//! bloqueadora escondida bloqueia normalmente.

use crate::index::{Entry, Indice};
use crate::meta;
use crate::trim;

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
/// `None`. Segue só bloqueadoras ativas (as arquivadas estão resolvidas). O id
/// é o nó: com o mesmo id em dois projetos, as arestas de todas as entradas
/// ativas com esse id contam, porque a lista cita o id e não sabe qual das
/// duas quer.
pub fn ciclo(indice: &Indice, inicio: &str) -> Option<Vec<String>> {
    /// `visitados` não se desfaz no recuo: com o alvo fixo, um nó já
    /// explorado sem chegar a ele não chega por outro caminho. Sem isso a
    /// busca percorre todos os caminhos simples, exponencial num grafo denso,
    /// e o `doctor` a roda para cada task ativa.
    fn anda(
        indice: &Indice,
        atual: &str,
        alvo: &str,
        caminho: &mut Vec<String>,
        visitados: &mut std::collections::BTreeSet<String>,
    ) -> bool {
        let arestas = indice
            .entradas
            .iter()
            .filter(|e| e.task_id == atual && e.archived.is_none())
            .flat_map(|e| e.meta.bloqueada_por.iter());
        for dep in arestas {
            if dep == alvo {
                caminho.push(dep.clone());
                return true;
            }
            if visitados.contains(dep) || !ativa(indice, dep) {
                continue;
            }
            visitados.insert(dep.clone());
            caminho.push(dep.clone());
            if anda(indice, dep, alvo, caminho, visitados) {
                return true;
            }
            caminho.pop();
        }
        false
    }
    let mut caminho = vec![inicio.to_string()];
    let mut visitados = std::collections::BTreeSet::from([inicio.to_string()]);
    if anda(indice, inicio, inicio, &mut caminho, &mut visitados) {
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

/// Valida o texto de `set --bloqueada-por` contra o índice e devolve a lista a
/// gravar: ids aparados, sem vazios nem repetidos, na ordem em que vieram. `""`
/// dá a lista vazia, que é sempre aceita (limpar não cria ciclo, e é como se
/// corta a aresta que o `doctor` acusa). Recusa, sem gravar nada, o id da
/// própria task, o que não existe entre ativas e arquivadas, o que existe em
/// mais de um arquivo e a lista que fecha um ciclo.
pub fn validar_bloqueada_por(
    indice: &Indice,
    alvo: &Entry,
    texto: &str,
) -> Result<Vec<String>, String> {
    let mut ids: Vec<String> = Vec::new();
    for id in texto.split(',').map(trim).filter(|s| !s.is_empty()) {
        if !ids.iter().any(|i| i == id) {
            ids.push(id.to_string());
        }
    }
    for id in &ids {
        if *id == alvo.task_id {
            return Err(format!("a task não pode ser bloqueada por si mesma: {id}"));
        }
        match indice.por_id(id).len() {
            0 => return Err(format!("task não encontrada: {id}")),
            1 => {}
            n => {
                return Err(format!(
                    "id \"{id}\" é ambíguo ({n} arquivos); passe o caminho"
                ));
            }
        }
    }
    // Limpar sempre se aceita: é como se corta o ciclo que o `doctor` acusa,
    // e com o id do alvo repetido a outra cópia pode seguir no ciclo.
    if ids.is_empty() {
        return Ok(ids);
    }
    // O ciclo se procura no índice com a lista nova no lugar da do alvo.
    let mut novo = indice.clone();
    if let Some(e) = novo.entradas.iter_mut().find(|e| e.rel == alvo.rel) {
        e.meta.bloqueada_por = ids.clone();
    }
    if let Some(c) = ciclo(&novo, &alvo.task_id) {
        return Err(format!("ciclo de dependência: {}", c.join(" → ")));
    }
    Ok(ids)
}

/// Troca o id `velho` por `novo` na linha `bloqueada-por` das `lines` (o
/// `rename` arrasta a dependência junto com os wikilinks). Só o elemento igual
/// ao id conta (`a-2` não é `a`), repetidos saem mantendo a ordem, e só a
/// linha é regravada. Devolve quantos ids foram trocados.
pub fn trocar_id(lines: &mut Vec<String>, velho: &str, novo: &str) -> usize {
    let lista = meta::read(lines).bloqueada_por;
    let trocados = lista.iter().filter(|i| *i == velho).count();
    if trocados == 0 {
        return 0;
    }
    let mut nova: Vec<String> = Vec::new();
    for id in lista {
        let id = if id == velho { novo.to_string() } else { id };
        if !nova.contains(&id) {
            nova.push(id);
        }
    }
    meta::write_bloqueada_por(lines, &nova);
    trocados
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

    #[test]
    fn ciclo_trata_o_id_como_no_e_une_as_arestas_dos_dois_projetos() {
        // `a` existe em `p` (sem dependência) e em `q` (espera `b`); `b`
        // espera `a`. O ciclo só aparece se as arestas das duas cópias contam.
        let i = idx(&[
            ("projects/p/tasks/a.md", &[]),
            ("projects/q/tasks/a.md", &["b"]),
            ("projects/p/tasks/b.md", &["a"]),
        ]);
        assert_eq!(ciclo(&i, "a").unwrap(), vec!["a", "b", "a"]);
        assert_eq!(ciclo(&i, "b").unwrap(), vec!["b", "a", "b"]);
    }

    fn lista(i: &Indice, alvo: &str, texto: &str) -> Result<Vec<String>, String> {
        validar_bloqueada_por(i, i.por_id(alvo)[0], texto)
    }

    #[test]
    fn validar_apara_tira_vazios_e_repetidos_mantendo_a_ordem() {
        let i = idx(&[
            ("projects/p/tasks/a.md", &[]),
            ("projects/p/tasks/b.md", &[]),
            ("projects/p/tasks/c.md", &[]),
        ]);
        assert_eq!(lista(&i, "a", " c , b,,c ,").unwrap(), vec!["c", "b"]);
        assert!(lista(&i, "a", "").unwrap().is_empty());
        assert!(lista(&i, "a", " , ,").unwrap().is_empty());
    }

    #[test]
    fn validar_aceita_limpar_mesmo_com_a_outra_copia_do_id_em_ciclo() {
        // `a` de `p` espera `b`, que espera `a`; `a` de `q` não espera nada.
        // Limpar a de `q` não desfaz o ciclo (o nó é o id), mas se aceita.
        let i = idx(&[
            ("projects/q/tasks/a.md", &[]),
            ("projects/p/tasks/a.md", &["b"]),
            ("projects/p/tasks/b.md", &["a"]),
        ]);
        assert_eq!(i.por_id("a")[0].rel, "projects/q/tasks/a.md");
        assert!(lista(&i, "a", "").unwrap().is_empty());
        assert!(lista(&i, "a", " , ").unwrap().is_empty());
    }

    #[test]
    fn validar_aceita_arquivada_e_de_outro_projeto() {
        let i = idx(&[
            ("projects/p/tasks/a.md", &[]),
            ("projects/p/tasks/archived/done/z.md", &[]),
            ("projects/q/tasks/k.md", &[]),
        ]);
        assert_eq!(lista(&i, "a", "z,k").unwrap(), vec!["z", "k"]);
    }

    #[test]
    fn validar_recusa_a_propria_a_inexistente_e_a_ambigua() {
        let i = idx(&[
            ("projects/p/tasks/a.md", &[]),
            ("projects/p/tasks/b.md", &[]),
            ("projects/p/tasks/d.md", &[]),
            ("projects/q/tasks/d.md", &[]),
            ("projects/q/tasks/archived/done/e.md", &[]),
            ("projects/p/tasks/e.md", &[]),
        ]);
        assert_eq!(
            lista(&i, "a", "b,a").unwrap_err(),
            "a task não pode ser bloqueada por si mesma: a"
        );
        assert_eq!(
            lista(&i, "a", "b,zzz").unwrap_err(),
            "task não encontrada: zzz"
        );
        assert_eq!(
            lista(&i, "a", "d").unwrap_err(),
            "id \"d\" é ambíguo (2 arquivos); passe o caminho"
        );
        assert_eq!(
            lista(&i, "a", "e").unwrap_err(),
            "id \"e\" é ambíguo (2 arquivos); passe o caminho",
            "ativa e arquivada com o mesmo id também são duas"
        );
    }

    #[test]
    fn validar_recusa_ciclo_com_o_caminho_e_aceita_limpar_task_em_ciclo() {
        let i = idx(&[
            ("projects/p/tasks/a.md", &[]),
            ("projects/p/tasks/b.md", &["c"]),
            ("projects/p/tasks/c.md", &["a"]),
            ("projects/p/tasks/w.md", &[]),
        ]);
        assert_eq!(
            lista(&i, "a", "w,b").unwrap_err(),
            "ciclo de dependência: a → b → c → a"
        );
        assert_eq!(lista(&i, "a", "w").unwrap(), vec!["w"]);
        // Uma lista que já era ciclo à mão: limpar é sempre aceito.
        let em_ciclo = idx(&[
            ("projects/p/tasks/a.md", &["b"]),
            ("projects/p/tasks/b.md", &["a"]),
        ]);
        assert!(lista(&em_ciclo, "a", "").unwrap().is_empty());
        // Arquivada não fecha ciclo: está resolvida.
        let arq = idx(&[
            ("projects/p/tasks/a.md", &[]),
            ("projects/p/tasks/archived/done/z.md", &["a"]),
        ]);
        assert_eq!(lista(&arq, "a", "z").unwrap(), vec!["z"]);
    }

    fn linhas(xs: &[&str]) -> Vec<String> {
        xs.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn trocar_id_troca_so_o_elemento_igual_e_tira_repetidos() {
        let mut l = linhas(&["---", "tags: [x]", "bloqueada-por: [a-2, a, aa, b]", "---"]);
        assert_eq!(trocar_id(&mut l, "a", "novo"), 1);
        assert_eq!(l[2], "bloqueada-por: [a-2, novo, aa, b]");
        assert_eq!((l[1].as_str(), l.len()), ("tags: [x]", 4));
        // O id novo já na lista: sai um só, na primeira posição.
        let mut l = linhas(&["---", "bloqueada-por: [a, b, novo]", "---"]);
        assert_eq!(trocar_id(&mut l, "a", "novo"), 1);
        assert_eq!(l[1], "bloqueada-por: [novo, b]");
        // Sem o id, ou sem lista, nada muda.
        let mut l = linhas(&["---", "bloqueada-por: [b]", "---"]);
        assert_eq!(trocar_id(&mut l, "a", "novo"), 0);
        assert_eq!(l[1], "bloqueada-por: [b]");
        let mut l = linhas(&["> [!todo] T"]);
        assert_eq!(trocar_id(&mut l, "a", "novo"), 0);
        assert_eq!(l, linhas(&["> [!todo] T"]));
    }
}
