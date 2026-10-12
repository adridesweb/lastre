use std::fs;
use std::os::windows::fs::MetadataExt;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering::Relaxed};

use rayon::prelude::*;

use crate::tree::TempNode;

const FILE_ATTRIBUTE_OFFLINE: u32 = 0x1000;
const FILE_ATTRIBUTE_RECALL_ON_OPEN: u32 = 0x4_0000;
const FILE_ATTRIBUTE_RECALL_ON_DATA_ACCESS: u32 = 0x40_0000;
/// Archivos que solo existen en la nube (OneDrive, etc.): no ocupan disco.
const CLOUD_ONLY: u32 =
    FILE_ATTRIBUTE_OFFLINE | FILE_ATTRIBUTE_RECALL_ON_OPEN | FILE_ATTRIBUTE_RECALL_ON_DATA_ACCESS;

#[derive(Default)]
pub struct Progress {
    pub files: AtomicU64,
    pub bytes: AtomicU64,
    pub denied: AtomicU64,
    pub cancel: AtomicBool,
    pub done: AtomicBool,
    pub current: Mutex<String>,
}

/// Escanea `root` en paralelo. Devuelve los hijos directos de la raíz.
pub fn scan(root: &Path, progress: &Progress) -> Vec<TempNode> {
    // La recursión con robo de trabajo de rayon puede apilar muchos marcos
    // en árboles profundos; damos pila de sobra a los workers.
    match rayon::ThreadPoolBuilder::new().stack_size(32 << 20).build() {
        Ok(pool) => pool.install(|| scan_dir(root, progress)),
        Err(_) => scan_dir(root, progress),
    }
}

fn scan_dir(path: &Path, p: &Progress) -> Vec<TempNode> {
    if p.cancel.load(Relaxed) {
        return Vec::new();
    }
    let rd = match fs::read_dir(path) {
        Ok(rd) => rd,
        Err(_) => {
            p.denied.fetch_add(1, Relaxed);
            return Vec::new();
        }
    };
    if let Ok(mut current) = p.current.try_lock() {
        current.clear();
        current.push_str(&path.to_string_lossy());
    }

    let mut out = Vec::new();
    let mut dirs: Vec<(Box<str>, PathBuf)> = Vec::new();
    let mut bytes = 0u64;
    for entry in rd.flatten() {
        let Ok(ft) = entry.file_type() else { continue };
        // Junctions y symlinks: no se siguen (bucles y dobles conteos).
        if ft.is_symlink() {
            continue;
        }
        let name: Box<str> = entry.file_name().to_string_lossy().into();
        if ft.is_dir() {
            dirs.push((name, entry.path()));
        } else {
            // En Windows los metadatos vienen del propio listado, sin llamada extra.
            let size = match entry.metadata() {
                Ok(m) if m.file_attributes() & CLOUD_ONLY == 0 => m.len(),
                _ => 0,
            };
            bytes += size;
            out.push(TempNode { name, size, files: 1, is_dir: false, children: Vec::new() });
        }
    }
    p.files.fetch_add(out.len() as u64, Relaxed);
    p.bytes.fetch_add(bytes, Relaxed);

    let subdirs: Vec<TempNode> = dirs
        .into_par_iter()
        .map(|(name, path)| {
            let children = scan_dir(&path, p);
            TempNode {
                name,
                size: children.iter().map(|c| c.size).sum(),
                files: children.iter().map(|c| c.files).sum(),
                is_dir: true,
                children,
            }
        })
        .collect();
    out.extend(subdirs);
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tree::Tree;
    use std::process::Command;

    #[test]
    fn scans_sizes_and_skips_junctions() {
        let tmp = tempfile::tempdir().unwrap();
        let a = tmp.path().join("a");
        fs::create_dir_all(a.join("sub")).unwrap();
        fs::write(a.join("b.txt"), vec![0u8; 100]).unwrap();
        fs::write(a.join("sub").join("c.bin"), vec![0u8; 50]).unwrap();
        fs::create_dir(tmp.path().join("vacia")).unwrap();

        let link = tmp.path().join("enlace");
        let status = Command::new("cmd")
            .args(["/C", "mklink", "/J"])
            .arg(&link)
            .arg(&a)
            .output()
            .unwrap()
            .status;
        assert!(status.success(), "no se pudo crear la junction");

        let progress = Progress::default();
        let tree = Tree::build(tmp.path().to_path_buf(), scan(tmp.path(), &progress));

        assert_eq!(tree.nodes[0].size, 150);
        assert_eq!(tree.nodes[0].files, 2);
        assert!(tree.nodes.iter().all(|n| &*n.name != "enlace"));
        assert!(tree.nodes.iter().any(|n| &*n.name == "vacia" && n.is_dir));
        assert_eq!(progress.files.load(Relaxed), 2);
        assert_eq!(progress.bytes.load(Relaxed), 150);
        assert_eq!(progress.denied.load(Relaxed), 0);
    }

    #[test]
    fn missing_root_counts_as_denied() {
        let progress = Progress::default();
        assert!(scan(Path::new(r"Z:\no\existe\de\verdad"), &progress).is_empty());
        assert_eq!(progress.denied.load(Relaxed), 1);
    }
}
