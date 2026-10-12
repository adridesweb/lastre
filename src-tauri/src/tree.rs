use std::cmp::Reverse;
use std::collections::{BinaryHeap, HashMap};
use std::ops::Range;
use std::path::PathBuf;

pub const NO_PARENT: u32 = u32::MAX;

/// Nodo intermedio que produce el escáner antes de aplanar el árbol.
pub struct TempNode {
    pub name: Box<str>,
    pub size: u64,
    pub files: u32,
    pub is_dir: bool,
    /// Ocupa en disco menos que su tamaño lógico (disperso, comprimido o solo en la nube),
    /// o contiene algo que lo hace.
    pub shrunk: bool,
    pub children: Vec<TempNode>,
}

pub struct Node {
    pub name: Box<str>,
    pub parent: u32,
    /// Los hijos de un nodo son contiguos en la arena.
    pub child_start: u32,
    pub child_count: u32,
    pub size: u64,
    pub files: u32,
    /// Índice en `Tree::exts`; 0 = sin extensión (y directorios).
    pub ext: u16,
    pub is_dir: bool,
    pub deleted: bool,
    /// Ver `TempNode::shrunk`.
    pub shrunk: bool,
}

pub struct Tree {
    pub nodes: Vec<Node>,
    pub exts: Vec<String>,
    pub root_path: PathBuf,
}

fn ext_of(name: &str) -> Option<String> {
    let idx = name.rfind('.')?;
    if idx == 0 {
        return None;
    }
    let ext = &name[idx + 1..];
    if ext.is_empty() || ext.len() > 10 || !ext.chars().all(|c| c.is_alphanumeric()) {
        return None;
    }
    Some(ext.to_lowercase())
}

impl Tree {
    /// Aplana el resultado del escaneo en una arena, con los hijos de cada
    /// directorio ordenados por tamaño descendente.
    pub fn build(root_path: PathBuf, children: Vec<TempNode>) -> Tree {
        let mut nodes = Vec::new();
        let mut exts = vec![String::new()];
        let mut ext_ids: HashMap<String, u16> = HashMap::new();

        nodes.push(Node {
            name: root_path.to_string_lossy().into(),
            parent: NO_PARENT,
            child_start: 0,
            child_count: 0,
            size: children.iter().map(|c| c.size).sum(),
            files: children.iter().map(|c| c.files).sum(),
            ext: 0,
            is_dir: true,
            deleted: false,
            shrunk: children.iter().any(|c| c.shrunk),
        });

        let mut stack = vec![(0u32, children)];
        while let Some((id, mut ch)) = stack.pop() {
            ch.sort_unstable_by(|a, b| b.size.cmp(&a.size));
            let start = nodes.len() as u32;
            nodes[id as usize].child_start = start;
            nodes[id as usize].child_count = ch.len() as u32;
            for (i, c) in ch.into_iter().enumerate() {
                let ext = if c.is_dir {
                    0
                } else {
                    match ext_of(&c.name) {
                        None => 0,
                        Some(e) => match ext_ids.get(&e) {
                            Some(&id) => id,
                            None if exts.len() < u16::MAX as usize => {
                                let id = exts.len() as u16;
                                exts.push(e.clone());
                                ext_ids.insert(e, id);
                                id
                            }
                            None => 0,
                        },
                    }
                };
                nodes.push(Node {
                    name: c.name,
                    parent: id,
                    child_start: 0,
                    child_count: 0,
                    size: c.size,
                    files: c.files,
                    ext,
                    is_dir: c.is_dir,
                    deleted: false,
                    shrunk: c.shrunk,
                });
                if !c.children.is_empty() {
                    stack.push((start + i as u32, c.children));
                }
            }
        }

        Tree { nodes, exts, root_path }
    }

    pub fn get(&self, id: u32) -> Option<&Node> {
        self.nodes.get(id as usize).filter(|n| !n.deleted)
    }

    pub fn children(&self, id: u32) -> Range<u32> {
        let n = &self.nodes[id as usize];
        n.child_start..n.child_start + n.child_count
    }

    /// Ids desde la raíz hasta `id`, ambos incluidos.
    pub fn ancestors(&self, id: u32) -> Vec<u32> {
        let mut out = vec![id];
        let mut p = self.nodes[id as usize].parent;
        while p != NO_PARENT {
            out.push(p);
            p = self.nodes[p as usize].parent;
        }
        out.reverse();
        out
    }

    pub fn path(&self, id: u32) -> PathBuf {
        let mut path = self.root_path.clone();
        for a in self.ancestors(id).into_iter().skip(1) {
            path.push(&*self.nodes[a as usize].name);
        }
        path
    }

    /// Los `limit` archivos más grandes bajo `root`, de mayor a menor;
    /// con `ext`, solo los de esa extensión.
    pub fn largest_files(&self, root: u32, ext: Option<u16>, limit: usize) -> Vec<u32> {
        let limit = limit.max(1);
        let mut heap: BinaryHeap<Reverse<(u64, u32)>> = BinaryHeap::with_capacity(limit + 1);
        let mut stack = vec![root];
        while let Some(id) = stack.pop() {
            let n = &self.nodes[id as usize];
            if n.deleted {
                continue;
            }
            if n.is_dir {
                // Un directorio que no supera al menor de los elegidos no puede aportar nada.
                let full = heap.len() == limit;
                if full && heap.peek().is_some_and(|min| n.size <= min.0.0) {
                    continue;
                }
                stack.extend(self.children(id));
            } else if ext.is_none_or(|e| e == n.ext) {
                heap.push(Reverse((n.size, id)));
                if heap.len() > limit {
                    heap.pop();
                }
            }
        }
        let mut found = heap.into_vec();
        found.sort_unstable();
        found.into_iter().map(|Reverse((_, id))| id).collect()
    }

    /// Marca el nodo como eliminado y descuenta su tamaño de los ancestros.
    pub fn remove(&mut self, id: u32) {
        let (size, files, mut p) = {
            let n = &mut self.nodes[id as usize];
            let r = (n.size, n.files, n.parent);
            n.deleted = true;
            n.size = 0;
            n.files = 0;
            r
        };
        while p != NO_PARENT {
            let n = &mut self.nodes[p as usize];
            n.size -= size;
            n.files -= files;
            p = n.parent;
        }
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    pub fn file(name: &str, size: u64) -> TempNode {
        TempNode { name: name.into(), size, files: 1, is_dir: false, shrunk: false, children: vec![] }
    }

    pub fn dir(name: &str, children: Vec<TempNode>) -> TempNode {
        TempNode {
            name: name.into(),
            size: children.iter().map(|c| c.size).sum(),
            files: children.iter().map(|c| c.files).sum(),
            is_dir: true,
            shrunk: false,
            children,
        }
    }

    pub fn sample() -> Tree {
        Tree::build(
            PathBuf::from(r"C:\raiz"),
            vec![
                file("pequeño.txt", 10),
                dir("docs", vec![file("a.PDF", 300), file("b.pdf", 200), file("LEEME", 5)]),
                file("video.mp4", 1000),
            ],
        )
    }

    fn find(t: &Tree, name: &str) -> u32 {
        t.nodes.iter().position(|n| &*n.name == name).unwrap() as u32
    }

    #[test]
    fn aggregates_and_sorts() {
        let t = sample();
        assert_eq!(t.nodes[0].size, 1515);
        assert_eq!(t.nodes[0].files, 5);
        let names: Vec<&str> = t.children(0).map(|c| &*t.nodes[c as usize].name).collect();
        assert_eq!(names, ["video.mp4", "docs", "pequeño.txt"]);
    }

    #[test]
    fn interns_extensions_case_insensitively() {
        let t = sample();
        assert_eq!(t.nodes[find(&t, "a.PDF") as usize].ext, t.nodes[find(&t, "b.pdf") as usize].ext);
        assert_eq!(t.nodes[find(&t, "LEEME") as usize].ext, 0);
        assert!(t.exts.contains(&"pdf".to_string()));
    }

    #[test]
    fn builds_paths_and_ancestors() {
        let t = sample();
        let a = find(&t, "a.PDF");
        assert_eq!(t.path(a), PathBuf::from(r"C:\raiz\docs\a.PDF"));
        assert_eq!(t.ancestors(a), vec![0, find(&t, "docs"), a]);
        assert_eq!(t.path(0), PathBuf::from(r"C:\raiz"));
    }

    #[test]
    fn largest_files_ranked_and_filtered() {
        let mut t = sample();
        let names = |t: &Tree, ids: Vec<u32>| -> Vec<String> {
            ids.into_iter().map(|i| t.nodes[i as usize].name.to_string()).collect()
        };
        assert_eq!(names(&t, t.largest_files(0, None, 3)), ["video.mp4", "a.PDF", "b.pdf"]);
        assert_eq!(t.largest_files(0, None, 100).len(), 5);
        let pdf = t.exts.iter().position(|e| e == "pdf").unwrap() as u16;
        assert_eq!(names(&t, t.largest_files(0, Some(pdf), 10)), ["a.PDF", "b.pdf"]);
        assert_eq!(names(&t, t.largest_files(find(&t, "docs"), None, 1)), ["a.PDF"]);
        t.remove(find(&t, "video.mp4"));
        assert_eq!(names(&t, t.largest_files(0, None, 1)), ["a.PDF"]);
    }

    #[test]
    fn remove_updates_ancestors() {
        let mut t = sample();
        let a = find(&t, "a.PDF");
        let docs = find(&t, "docs");
        t.remove(a);
        assert_eq!(t.nodes[docs as usize].size, 205);
        assert_eq!(t.nodes[docs as usize].files, 2);
        assert_eq!(t.nodes[0].size, 1215);
        assert_eq!(t.nodes[0].files, 4);
        assert!(t.get(a).is_none());
    }
}
