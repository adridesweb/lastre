//! Comprobaciones previas a enviar algo a la Papelera.
//!
//! Windows borra de forma permanente, sin avisar, lo que no cabe en la Papelera
//! o está en una unidad sin Papelera. Aquí se detectan esos casos antes de
//! tocar nada, y se protegen las carpetas de las que depende el sistema.

use std::os::windows::fs::MetadataExt;
use std::path::{Component, Path, PathBuf};

use windows_sys::Win32::Storage::FileSystem::{
    GetDiskFreeSpaceExW, GetDriveTypeW, GetVolumeNameForVolumeMountPointW, GetVolumePathNameW,
    QueryDosDeviceW,
};
use windows_sys::Win32::UI::Shell::{SHQUERYRBINFO, SHQueryRecycleBinW};
use winreg::RegKey;
use winreg::enums::HKEY_CURRENT_USER;

const DRIVE_FIXED: u32 = 3;
const FILE_ATTRIBUTE_SYSTEM: u32 = 0x4;
const GB: u64 = 1 << 30;

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum Risk {
    /// Nunca se envía a la Papelera desde la app.
    Blocked,
    /// Se permite, pero con un aviso: puede romper programas instalados.
    Caution,
}

/// Carpetas de las que depende Windows o los programas instalados.
pub struct Protected {
    pub system_root: Option<PathBuf>,
    /// Program Files, Program Files (x86) y ProgramData.
    pub program_dirs: Vec<PathBuf>,
    pub profile: Option<PathBuf>,
}

impl Protected {
    pub fn from_env() -> Self {
        let var = |name: &str| std::env::var_os(name).map(PathBuf::from);
        Protected {
            system_root: var("SystemRoot"),
            program_dirs: ["ProgramFiles", "ProgramFiles(x86)", "ProgramW6432", "ProgramData"]
                .into_iter()
                .filter_map(var)
                .collect(),
            profile: var("USERPROFILE"),
        }
    }
}

fn parts(path: &Path) -> Vec<String> {
    path.components()
        .filter(|c| !matches!(c, Component::RootDir | Component::CurDir))
        .map(|c| c.as_os_str().to_string_lossy().to_lowercase())
        .collect()
}

/// `path` es `base` o está dentro de `base` (sin distinguir mayúsculas).
fn is_within(path: &Path, base: &Path) -> bool {
    let (path, base) = (parts(path), parts(base));
    path.len() >= base.len() && path[..base.len()] == base[..]
}

pub fn classify(path: &Path, protected: &Protected) -> Option<Risk> {
    // La raíz de una unidad.
    if parts(path).len() <= 1 {
        return Some(Risk::Blocked);
    }
    if protected.system_root.as_deref().is_some_and(|root| is_within(path, root)) {
        return Some(Risk::Blocked);
    }
    // Las propias carpetas protegidas y cualquier carpeta que las contenga.
    let anchors = protected.program_dirs.iter().chain(&protected.profile);
    if anchors.clone().any(|dir| is_within(dir, path)) {
        return Some(Risk::Blocked);
    }
    // Los perfiles de los demás usuarios (Default, Public, otras cuentas).
    let users_dir = protected.profile.as_deref().and_then(Path::parent);
    if users_dir.is_some_and(|users| path.parent().is_some_and(|p| parts(p) == parts(users))) {
        return Some(Risk::Blocked);
    }

    if protected.program_dirs.iter().any(|dir| is_within(path, dir)) {
        return Some(Risk::Caution);
    }
    let app_data = protected.profile.as_deref().map(|p| p.join("AppData"));
    if app_data.is_some_and(|dir| is_within(path, &dir)) {
        return Some(Risk::Caution);
    }
    None
}

/// Archivos y carpetas de sistema en la raíz de una unidad
/// (pagefile.sys, hiberfil.sys, System Volume Information, $Recycle.Bin…).
pub fn is_root_system_item(path: &Path) -> bool {
    parts(path).len() == 2
        && std::fs::symlink_metadata(path)
            .is_ok_and(|m| m.file_attributes() & FILE_ATTRIBUTE_SYSTEM != 0)
}

/// Tamaño lógico de un archivo o carpeta leído del disco. Es el que Windows compara
/// con la capacidad de la Papelera, aunque en disco ocupe menos.
pub fn logical_size(path: &Path) -> u64 {
    let mut total = 0u64;
    let mut pending = vec![path.to_path_buf()];
    while let Some(current) = pending.pop() {
        let Ok(meta) = std::fs::symlink_metadata(&current) else { continue };
        if meta.file_type().is_symlink() {
            continue;
        }
        if meta.is_dir() {
            if let Ok(entries) = std::fs::read_dir(&current) {
                pending.extend(entries.flatten().map(|e| e.path()));
            }
        } else {
            total = total.saturating_add(meta.len());
        }
    }
    total
}

#[derive(Debug, PartialEq, Eq)]
pub enum BinIssue {
    /// La unidad no tiene Papelera (USB, red…) o no se ha podido comprobar.
    NoBin,
    /// El usuario tiene configurado borrar sin pasar por la Papelera.
    Disabled,
    /// El elemento es mayor que la capacidad total de la Papelera.
    TooBig { capacity: u64 },
    /// Cabría, pero Windows vaciaría elementos antiguos de la Papelera para hacerle sitio.
    Full { capacity: u64, used: u64 },
}

impl BinIssue {
    /// Código que la interfaz traduce a un mensaje.
    pub fn code(&self) -> String {
        match self {
            BinIssue::NoBin => "no-bin".into(),
            BinIssue::Disabled => "bin-disabled".into(),
            BinIssue::TooBig { capacity } => format!("too-big:{capacity}"),
            BinIssue::Full { capacity, used } => format!("bin-full:{capacity}:{used}"),
        }
    }
}

fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain([0]).collect()
}

fn from_wide(buffer: &[u16]) -> String {
    let len = buffer.iter().position(|&c| c == 0).unwrap_or(buffer.len());
    String::from_utf16_lossy(&buffer[..len])
}

/// Capacidad que Windows asigna por defecto a la Papelera de un volumen:
/// el 10 % de los primeros 40 GB y el 5 % del resto.
fn default_capacity(volume_size: u64) -> u64 {
    let first = volume_size.min(40 * GB);
    first / 10 + (volume_size - first) / 20
}

struct BinState {
    capacity: u64,
    used: u64,
}

/// Carpeta a la que apunta una unidad `subst` ("L:\" -> "C:\datos\demo"), si lo es.
fn subst_target(root: &[u16]) -> Option<PathBuf> {
    let device = [*root.first()?, *root.get(1)?, 0];
    let mut target = [0u16; 520];
    let len = unsafe { QueryDosDeviceW(device.as_ptr(), target.as_mut_ptr(), target.len() as u32) };
    if len == 0 {
        return None;
    }
    from_wide(&target).strip_prefix(r"\??\").map(PathBuf::from)
}

fn bin_state(path: &Path) -> Result<BinState, BinIssue> {
    bin_state_of(path, true)
}

fn bin_state_of(path: &Path, follow_subst: bool) -> Result<BinState, BinIssue> {
    let path_w = wide(&path.to_string_lossy());
    let mut root = [0u16; 261];
    if unsafe { GetVolumePathNameW(path_w.as_ptr(), root.as_mut_ptr(), root.len() as u32) } == 0 {
        return Err(BinIssue::NoBin);
    }
    if unsafe { GetDriveTypeW(root.as_ptr()) } != DRIVE_FIXED {
        return Err(BinIssue::NoBin);
    }
    let mut volume = [0u16; 64];
    let named = unsafe {
        GetVolumeNameForVolumeMountPointW(root.as_ptr(), volume.as_mut_ptr(), volume.len() as u32)
    };
    if named == 0 {
        // Una unidad creada con `subst` no es un volumen: se comprueba la carpeta real.
        return match subst_target(&root) {
            Some(target) if follow_subst => bin_state_of(&target, false),
            _ => Err(BinIssue::NoBin),
        };
    }
    // "\\?\Volume{guid}\" -> "{guid}"
    let volume = from_wide(&volume);
    let guid = volume
        .find('{')
        .zip(volume.find('}'))
        .map(|(start, end)| &volume[start..=end])
        .ok_or(BinIssue::NoBin)?;

    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let policy = hkcu.open_subkey(r"Software\Microsoft\Windows\CurrentVersion\Policies\Explorer");
    if policy.is_ok_and(|key| key.get_value::<u32, _>("NoRecycleFiles").unwrap_or(0) != 0) {
        return Err(BinIssue::Disabled);
    }
    let settings = hkcu.open_subkey(format!(
        r"Software\Microsoft\Windows\CurrentVersion\Explorer\BitBucket\Volume\{guid}"
    ));
    let mut capacity = None;
    if let Ok(key) = settings {
        if key.get_value::<u32, _>("NukeOnDelete").unwrap_or(0) != 0 {
            return Err(BinIssue::Disabled);
        }
        // El registro guarda la capacidad en MB.
        capacity = key.get_value::<u32, _>("MaxCapacity").ok().map(|mb| mb as u64 * (1 << 20));
    }
    let capacity = match capacity {
        Some(capacity) => capacity,
        None => {
            let mut total = 0u64;
            let ok = unsafe {
                GetDiskFreeSpaceExW(root.as_ptr(), std::ptr::null_mut(), &mut total, std::ptr::null_mut())
            };
            if ok == 0 {
                return Err(BinIssue::NoBin);
            }
            default_capacity(total)
        }
    };

    let mut info = SHQUERYRBINFO {
        cbSize: std::mem::size_of::<SHQUERYRBINFO>() as u32,
        i64Size: 0,
        i64NumItems: 0,
    };
    // Si la consulta falla la Papelera de ese volumen aún no existe: está vacía.
    let used = match unsafe { SHQueryRecycleBinW(root.as_ptr(), &mut info) } {
        0 => info.i64Size.max(0) as u64,
        _ => 0,
    };
    Ok(BinState { capacity, used })
}

fn fits(size: u64, bin: &BinState) -> Result<(), BinIssue> {
    if size > bin.capacity {
        return Err(BinIssue::TooBig { capacity: bin.capacity });
    }
    if bin.used.saturating_add(size) > bin.capacity {
        return Err(BinIssue::Full { capacity: bin.capacity, used: bin.used });
    }
    Ok(())
}

/// Comprueba que `path`, de `size` bytes, acabará de verdad en la Papelera
/// sin que Windows borre nada de forma permanente para conseguirlo.
pub fn check_bin(path: &Path, size: u64) -> Result<(), BinIssue> {
    fits(size, &bin_state(path)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn protected() -> Protected {
        Protected {
            system_root: Some(PathBuf::from(r"C:\Windows")),
            program_dirs: vec![
                PathBuf::from(r"C:\Program Files"),
                PathBuf::from(r"C:\Program Files (x86)"),
                PathBuf::from(r"C:\ProgramData"),
            ],
            profile: Some(PathBuf::from(r"C:\Users\ana")),
        }
    }

    fn risk(path: &str) -> Option<Risk> {
        classify(Path::new(path), &protected())
    }

    #[test]
    fn blocks_what_the_system_depends_on() {
        for path in [
            r"C:\",
            r"D:\",
            r"C:\Windows",
            r"c:\windows\System32\drivers",
            r"C:\WINDOWS\Temp\algo.tmp",
            r"C:\Program Files",
            r"C:\Program Files (x86)",
            r"C:\ProgramData",
            r"C:\Users",
            r"C:\Users\ana",
            r"C:\Users\Public",
            r"C:\Users\Default",
        ] {
            assert_eq!(risk(path), Some(Risk::Blocked), "{path}");
        }
    }

    #[test]
    fn warns_inside_program_and_app_data_folders() {
        for path in [
            r"C:\Program Files\Juego",
            r"C:\Program Files (x86)\Steam\steamapps\common\Juego\data.pak",
            r"C:\ProgramData\Paquetes",
            r"C:\Users\ana\AppData",
            r"C:\Users\ana\AppData\Local\Temp\cache.bin",
        ] {
            assert_eq!(risk(path), Some(Risk::Caution), "{path}");
        }
    }

    #[test]
    fn allows_ordinary_user_and_data_paths() {
        for path in [
            r"C:\Users\ana\Downloads",
            r"C:\Users\ana\Videos\boda.mp4",
            r"C:\XboxGames\Juego",
            r"D:\Copias\2021.zip",
            r"C:\Windows.old",
            r"C:\Program Files Extra",
        ] {
            assert_eq!(risk(path), None, "{path}");
        }
    }

    #[test]
    fn default_capacity_matches_windows_formula() {
        assert_eq!(default_capacity(20 * GB), 2 * GB);
        assert_eq!(default_capacity(40 * GB), 4 * GB);
        assert_eq!(default_capacity(1040 * GB), 54 * GB);
    }

    #[test]
    fn refuses_items_that_would_not_be_kept_in_the_bin() {
        let bin = BinState { capacity: 50 * GB, used: 30 * GB };
        assert_eq!(fits(10 * GB, &bin), Ok(()));
        assert_eq!(fits(20 * GB, &bin), Ok(()));
        assert_eq!(fits(21 * GB, &bin), Err(BinIssue::Full { capacity: 50 * GB, used: 30 * GB }));
        assert_eq!(fits(51 * GB, &bin), Err(BinIssue::TooBig { capacity: 50 * GB }));
    }

    #[test]
    fn reads_the_real_bin_of_the_temp_drive() {
        // La carpeta temporal está en un disco fijo con Papelera.
        let tmp = tempfile::tempdir().unwrap();
        let state = bin_state(tmp.path());
        assert!(state.as_ref().is_ok_and(|s| s.capacity > 0), "sin Papelera en {:?}", tmp.path());
        assert_eq!(check_bin(tmp.path(), u64::MAX).map_err(|e| e.code().starts_with("too-big:")), Err(true));
    }

    #[test]
    fn logical_size_sums_files_recursively() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::create_dir(tmp.path().join("sub")).unwrap();
        std::fs::write(tmp.path().join("a.bin"), vec![0u8; 300]).unwrap();
        std::fs::write(tmp.path().join("sub").join("b.bin"), vec![0u8; 700]).unwrap();
        assert_eq!(logical_size(tmp.path()), 1000);
        assert_eq!(logical_size(&tmp.path().join("a.bin")), 300);
    }

    #[test]
    fn unknown_volumes_have_no_bin() {
        assert_eq!(check_bin(Path::new(r"\\servidor-inexistente\recurso\archivo.txt"), 1), Err(BinIssue::NoBin));
    }

    #[test]
    fn root_system_items_are_detected() {
        let system_drive = std::env::var("SystemDrive").unwrap_or_else(|_| "C:".into());
        let bin = PathBuf::from(format!(r"{system_drive}\$Recycle.Bin"));
        if bin.exists() {
            assert!(is_root_system_item(&bin));
        }
        let tmp = tempfile::tempdir().unwrap();
        assert!(!is_root_system_item(tmp.path()));
    }
}
