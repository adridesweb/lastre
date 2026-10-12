use std::path::PathBuf;
use std::sync::atomic::Ordering::Relaxed;
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{Duration, Instant};

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, State};
use windows_sys::Win32::Storage::FileSystem::{
    GetDiskFreeSpaceExW, GetDriveTypeW, GetLogicalDrives,
};

use crate::filetypes;
use crate::safety::{self, Protected, Risk};
use crate::scanner::{self, Progress};
use crate::tree::Tree;
use crate::treemap::{self, Layout};

/// Máximo de hijos que se envían al árbol de la interfaz por directorio.
const MAX_CHILDREN: usize = 2000;
const MAX_FILE_TYPES: usize = 300;
const DRIVE_REMOVABLE: u32 = 2;
const DRIVE_FIXED: u32 = 3;

#[derive(Default)]
pub struct Inner {
    tree: Option<Tree>,
    colors: Vec<[u8; 3]>,
    layout: Option<Layout>,
}

#[derive(Default)]
pub struct AppState {
    inner: Mutex<Inner>,
    scan: Mutex<Option<Arc<Progress>>>,
}

impl AppState {
    fn data(&self) -> MutexGuard<'_, Inner> {
        self.inner.lock().unwrap_or_else(|e| e.into_inner())
    }

    fn scan_slot(&self) -> MutexGuard<'_, Option<Arc<Progress>>> {
        self.scan.lock().unwrap_or_else(|e| e.into_inner())
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DriveDto {
    path: String,
    total: u64,
    free: u64,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct NodeDto {
    id: u32,
    name: String,
    size: u64,
    files: u32,
    is_dir: bool,
    has_children: bool,
    color: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChildrenDto {
    items: Vec<NodeDto>,
    hidden_count: u32,
    hidden_size: u64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NodeInfo {
    id: u32,
    name: String,
    path: String,
    size: u64,
    files: u32,
    is_dir: bool,
    /// Ids desde la raíz del escaneo hasta este nodo.
    ancestors: Vec<u32>,
    /// Rectángulo en el treemap actual (del nodo o de su ancestro visible más cercano).
    rect: Option<[f32; 4]>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileTypeDto {
    ext: u16,
    name: String,
    color: String,
    size: u64,
    count: u32,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct ScanProgress {
    files: u64,
    bytes: u64,
    current: String,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct ScanDone {
    cancelled: bool,
    root: Option<NodeDto>,
    path: String,
    denied: u64,
    elapsed_ms: u64,
}

fn hex(c: [u8; 3]) -> String {
    format!("#{:02x}{:02x}{:02x}", c[0], c[1], c[2])
}

fn node_dto(tree: &Tree, colors: &[[u8; 3]], id: u32) -> NodeDto {
    let n = &tree.nodes[id as usize];
    NodeDto {
        id,
        name: n.name.to_string(),
        size: n.size,
        files: n.files,
        is_dir: n.is_dir,
        has_children: n.child_count > 0,
        color: (!n.is_dir).then(|| hex(colors[n.ext as usize])),
    }
}

fn info(inner: &Inner, id: u32) -> Option<NodeInfo> {
    let tree = inner.tree.as_ref()?;
    let n = tree.get(id)?;
    let ancestors = tree.ancestors(id);
    let rect = inner
        .layout
        .as_ref()
        .and_then(|l| ancestors.iter().rev().find_map(|&a| l.rect(a)));
    Some(NodeInfo {
        id,
        name: n.name.to_string(),
        path: tree.path(id).to_string_lossy().into_owned(),
        size: n.size,
        files: n.files,
        is_dir: n.is_dir,
        ancestors,
        rect,
    })
}

/// Limpia una ruta escrita a mano: separadores repetidos, barra final y "C:" sin barra
/// (que sería el directorio actual de esa unidad, no su raíz).
fn normalize(path: &str) -> PathBuf {
    let path = path.trim();
    if path.len() == 2 && path.ends_with(':') {
        return PathBuf::from(format!("{path}\\"));
    }
    std::path::Path::new(path).components().collect()
}

const NO_SCAN: &str = "No hay ningún escaneo cargado";
const NO_NODE: &str = "El elemento ya no existe";

/// Carpeta pasada como primer argumento (`lastre.exe C:\ruta`), si es válida.
#[tauri::command]
pub fn initial_path() -> Option<String> {
    let path = normalize(&std::env::args().nth(1)?);
    path.is_dir().then(|| path.to_string_lossy().into_owned())
}

#[tauri::command]
pub fn list_drives() -> Vec<DriveDto> {
    let mask = unsafe { GetLogicalDrives() };
    (0..26u8)
        .filter(|i| mask & (1 << i) != 0)
        .filter_map(|i| {
            let path = format!("{}:\\", (b'A' + i) as char);
            let wide: Vec<u16> = path.encode_utf16().chain([0]).collect();
            let kind = unsafe { GetDriveTypeW(wide.as_ptr()) };
            if kind != DRIVE_FIXED && kind != DRIVE_REMOVABLE {
                return None;
            }
            let (mut total, mut free) = (0u64, 0u64);
            let ok = unsafe {
                GetDiskFreeSpaceExW(wide.as_ptr(), std::ptr::null_mut(), &mut total, &mut free)
            };
            (ok != 0).then_some(DriveDto { path, total, free })
        })
        .collect()
}

#[tauri::command]
pub fn start_scan(app: AppHandle, state: State<AppState>, path: String) -> Result<(), String> {
    let root = normalize(&path);
    if !root.is_dir() {
        return Err(format!("No es una carpeta accesible: {}", root.display()));
    }

    let progress = Arc::new(Progress::default());
    {
        let mut scan = state.scan_slot();
        if scan.is_some() {
            return Err("Ya hay un escaneo en curso".into());
        }
        *scan = Some(progress.clone());
    }
    {
        // Liberamos el árbol anterior antes de empezar: puede ocupar cientos de MB.
        let mut inner = state.data();
        inner.tree = None;
        inner.layout = None;
    }

    let ticker = {
        let (app, progress) = (app.clone(), progress.clone());
        std::thread::spawn(move || {
            while !progress.done.load(Relaxed) {
                std::thread::sleep(Duration::from_millis(100));
                let current = progress.current.lock().map(|c| c.clone()).unwrap_or_default();
                let _ = app.emit(
                    "scan-progress",
                    ScanProgress {
                        files: progress.files.load(Relaxed),
                        bytes: progress.bytes.load(Relaxed),
                        current,
                    },
                );
            }
        })
    };

    std::thread::spawn(move || {
        let started = Instant::now();
        let children = scanner::scan(&root, &progress);
        let cancelled = progress.cancel.load(Relaxed);
        let state = app.state::<AppState>();

        let root_dto = (!cancelled).then(|| {
            let tree = Tree::build(root.clone(), children);
            let colors = filetypes::assign_colors(&tree);
            let dto = node_dto(&tree, &colors, 0);
            let mut inner = state.data();
            inner.tree = Some(tree);
            inner.colors = colors;
            inner.layout = None;
            dto
        });

        progress.done.store(true, Relaxed);
        let _ = ticker.join();
        *state.scan_slot() = None;
        let _ = app.emit(
            "scan-done",
            ScanDone {
                cancelled,
                root: root_dto,
                path: root.to_string_lossy().into_owned(),
                denied: progress.denied.load(Relaxed),
                elapsed_ms: started.elapsed().as_millis() as u64,
            },
        );
    });
    Ok(())
}

#[tauri::command]
pub fn cancel_scan(state: State<AppState>) {
    if let Some(progress) = state.scan_slot().as_ref() {
        progress.cancel.store(true, Relaxed);
    }
}

#[tauri::command]
pub fn get_node(state: State<AppState>, id: u32) -> Result<NodeDto, String> {
    let inner = state.data();
    let tree = inner.tree.as_ref().ok_or(NO_SCAN)?;
    tree.get(id).ok_or(NO_NODE)?;
    Ok(node_dto(tree, &inner.colors, id))
}

#[tauri::command(async)]
pub fn get_children(state: State<AppState>, id: u32) -> Result<ChildrenDto, String> {
    let inner = state.data();
    let tree = inner.tree.as_ref().ok_or(NO_SCAN)?;
    tree.get(id).ok_or(NO_NODE)?;
    let mut ids: Vec<u32> =
        tree.children(id).filter(|&c| !tree.nodes[c as usize].deleted).collect();
    // Ya vienen ordenados salvo tras eliminar algo; el orden estable lo hace barato.
    ids.sort_by_key(|&c| std::cmp::Reverse(tree.nodes[c as usize].size));
    let hidden = ids.split_off(ids.len().min(MAX_CHILDREN));
    Ok(ChildrenDto {
        items: ids.iter().map(|&c| node_dto(tree, &inner.colors, c)).collect(),
        hidden_count: hidden.len() as u32,
        hidden_size: hidden.iter().map(|&c| tree.nodes[c as usize].size).sum(),
    })
}

#[tauri::command(async)]
pub fn get_file_types(state: State<AppState>, root_id: u32) -> Result<Vec<FileTypeDto>, String> {
    let inner = state.data();
    let tree = inner.tree.as_ref().ok_or(NO_SCAN)?;
    tree.get(root_id).ok_or(NO_NODE)?;
    let stats = filetypes::stats(tree, root_id);
    let mut types: Vec<FileTypeDto> = stats
        .iter()
        .enumerate()
        .filter(|(_, s)| s.count > 0)
        .map(|(ext, s)| FileTypeDto {
            ext: ext as u16,
            name: tree.exts[ext].clone(),
            color: hex(inner.colors[ext]),
            size: s.size,
            count: s.count,
        })
        .collect();
    types.sort_unstable_by(|a, b| b.size.cmp(&a.size));
    types.truncate(MAX_FILE_TYPES);
    Ok(types)
}

#[tauri::command(async)]
pub fn render_treemap(
    state: State<AppState>,
    root_id: u32,
    width: u32,
    height: u32,
    highlight: Option<u16>,
) -> Result<tauri::ipc::Response, String> {
    let (width, height) = (width.clamp(1, 8192), height.clamp(1, 8192));
    let mut guard = state.data();
    let inner = &mut *guard;
    let tree = inner.tree.as_ref().ok_or(NO_SCAN)?;
    tree.get(root_id).ok_or(NO_NODE)?;
    let reusable = inner
        .layout
        .as_ref()
        .is_some_and(|l| l.root == root_id && l.w == width && l.h == height);
    if !reusable {
        inner.layout = Some(treemap::layout(tree, &inner.colors, root_id, width, height));
    }
    let layout = inner.layout.as_ref().expect("layout recién calculado");
    Ok(tauri::ipc::Response::new(treemap::render(layout, highlight)))
}

#[tauri::command]
pub fn hit_test(state: State<AppState>, x: u32, y: u32) -> Option<NodeInfo> {
    let inner = state.data();
    let node = inner.layout.as_ref()?.node_at(x, y)?;
    info(&inner, node)
}

#[tauri::command]
pub fn node_info(state: State<AppState>, id: u32) -> Option<NodeInfo> {
    info(&state.data(), id)
}

fn path_of(state: &AppState, id: u32) -> Result<PathBuf, String> {
    let inner = state.data();
    let tree = inner.tree.as_ref().ok_or(NO_SCAN)?;
    tree.get(id).ok_or(NO_NODE)?;
    Ok(tree.path(id))
}

#[tauri::command]
pub fn reveal(state: State<AppState>, id: u32) -> Result<(), String> {
    let path = path_of(&state, id)?;
    tauri_plugin_opener::reveal_item_in_dir(path).map_err(|e| e.to_string())
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TrashCheck {
    /// Código del motivo por el que no se puede enviar a la Papelera, si lo hay.
    blocked: Option<String>,
    /// Se puede enviar, pero puede romper programas instalados.
    caution: bool,
}

/// Motivo (como código para la interfaz) por el que un elemento no debe ir a la Papelera.
fn trash_blocker(path: &std::path::Path, size: u64, risk: Option<Risk>) -> Option<String> {
    if risk == Some(Risk::Blocked) || safety::is_root_system_item(path) {
        return Some("protected".into());
    }
    safety::check_bin(path, size).err().map(|issue| issue.code())
}

fn trash_target(state: &AppState, id: u32) -> Result<(PathBuf, u64), String> {
    let inner = state.data();
    let tree = inner.tree.as_ref().ok_or(NO_SCAN)?;
    let node = tree.get(id).ok_or(NO_NODE)?;
    let (path, size, shrunk) = (tree.path(id), node.size, node.shrunk);
    drop(inner);
    // El árbol guarda lo que ocupa en disco; para la Papelera cuenta el tamaño lógico.
    Ok((path.clone(), if shrunk { safety::logical_size(&path) } else { size }))
}

/// Comprueba, sin tocar nada, si un elemento se puede enviar a la Papelera.
#[tauri::command]
pub fn trash_check(state: State<AppState>, id: u32) -> Result<TrashCheck, String> {
    let (path, size) = trash_target(&state, id)?;
    if id == 0 {
        return Ok(TrashCheck { blocked: Some("protected".into()), caution: false });
    }
    let risk = safety::classify(&path, &Protected::from_env());
    Ok(TrashCheck {
        blocked: trash_blocker(&path, size, risk),
        caution: risk == Some(Risk::Caution),
    })
}

/// Envía el elemento a la Papelera de reciclaje. Se niega si no hay garantía
/// de que acabe allí: Windows lo borraría de forma permanente sin avisar.
#[tauri::command(async)]
pub fn trash(state: State<AppState>, id: u32) -> Result<(), String> {
    let (path, size) = trash_target(&state, id)?;
    if id == 0 {
        return Err("protected".into());
    }
    let risk = safety::classify(&path, &Protected::from_env());
    if let Some(code) = trash_blocker(&path, size, risk) {
        return Err(code);
    }
    ::trash::delete(&path).map_err(|e| e.to_string())?;
    let mut inner = state.data();
    if let Some(tree) = inner.tree.as_mut() {
        if tree.get(id).is_some() {
            tree.remove(id);
        }
    }
    inner.layout = None;
    Ok(())
}

/// Abre la Papelera de reciclaje en el Explorador.
#[tauri::command]
pub fn open_recycle_bin() -> Result<(), String> {
    std::process::Command::new("explorer.exe")
        .arg("shell:RecycleBinFolder")
        .spawn()
        .map(|_| ())
        .map_err(|e| format!("No se pudo abrir la Papelera: {e}"))
}

#[tauri::command(async)]
pub fn largest_files(
    state: State<AppState>,
    root_id: u32,
    ext: Option<u16>,
    limit: usize,
) -> Result<Vec<NodeInfo>, String> {
    let inner = state.data();
    let tree = inner.tree.as_ref().ok_or(NO_SCAN)?;
    tree.get(root_id).ok_or(NO_NODE)?;
    let ids = tree.largest_files(root_id, ext, limit.min(1000));
    Ok(ids.into_iter().filter_map(|id| info(&inner, id)).collect())
}
