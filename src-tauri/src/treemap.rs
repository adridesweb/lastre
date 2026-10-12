use std::collections::HashMap;

use rayon::prelude::*;

use crate::filetypes::OTHER_COLOR;
use crate::tree::Tree;

const NO_LEAF: u32 = u32::MAX;
/// Marca el bloque que agrupa los hijos demasiado pequeños para dibujarse.
const REMAINDER: u32 = u32::MAX;
const NO_EXT: u16 = u16::MAX;
const BACKGROUND: [u8; 3] = [0x1c, 0x1c, 0x20];

/// Hijos con menos área que esto (px²) se agrupan en un único bloque.
const MIN_CHILD_AREA: f64 = 0.4;

// Parámetros del sombreado acolchado (cushion treemap, van Wijk & van de Wetering).
const CUSHION_HEIGHT: f64 = 0.38;
const CUSHION_SCALE: f64 = 0.91;
const AMBIENT: f32 = 0.26;
const DIFFUSE: f32 = 1.04;
// Luz desde arriba a la izquierda: (-1, -1, 10) normalizado.
const LIGHT: [f32; 3] = [-0.099_015, -0.099_015, 0.990_148];

type Rect = [f64; 4]; // x, y, ancho, alto

pub struct Leaf {
    pub node: u32,
    surface: [f32; 4],
    color: [u8; 3],
    ext: u16,
}

pub struct Layout {
    pub root: u32,
    pub w: u32,
    pub h: u32,
    /// Índice de hoja por píxel (o `NO_LEAF`).
    owner: Vec<u32>,
    leaves: Vec<Leaf>,
    /// Rectángulo de cada nodo visitado, en píxeles.
    rects: HashMap<u32, [f32; 4]>,
}

impl Layout {
    pub fn node_at(&self, x: u32, y: u32) -> Option<u32> {
        if x >= self.w || y >= self.h {
            return None;
        }
        let leaf = self.owner[(y * self.w + x) as usize];
        (leaf != NO_LEAF).then(|| self.leaves[leaf as usize].node)
    }

    pub fn rect(&self, node: u32) -> Option<[f32; 4]> {
        self.rects.get(&node).copied()
    }

    fn push_leaf(&mut self, node: u32, r: Rect, surface: [f64; 4], color: [u8; 3], ext: u16) {
        let x0 = (r[0].round().max(0.0) as u32).min(self.w);
        let y0 = (r[1].round().max(0.0) as u32).min(self.h);
        let x1 = ((r[0] + r[2]).round().max(0.0) as u32).min(self.w);
        let y1 = ((r[1] + r[3]).round().max(0.0) as u32).min(self.h);
        if x1 <= x0 || y1 <= y0 {
            return;
        }
        let idx = self.leaves.len() as u32;
        for y in y0..y1 {
            let row = (y * self.w) as usize;
            self.owner[row + x0 as usize..row + x1 as usize].fill(idx);
        }
        self.leaves.push(Leaf { node, surface: surface.map(|v| v as f32), color, ext });
    }
}

fn add_ridge(surface: &mut [f64; 4], r: Rect, height: f64) {
    if r[2] > 0.0 {
        let f = 4.0 * height / r[2];
        surface[2] += f * (r[0] + r[0] + r[2]);
        surface[0] -= f;
    }
    if r[3] > 0.0 {
        let f = 4.0 * height / r[3];
        surface[3] += f * (r[1] + r[1] + r[3]);
        surface[1] -= f;
    }
}

fn worst_ratio(short: f64, sum: f64, min: f64, max: f64) -> f64 {
    let s2 = short * short;
    let sum2 = sum * sum;
    (s2 * max / sum2).max(sum2 / (s2 * min))
}

/// Treemap "squarified" (Bruls, Huizing & van Wijk). `items` son (id, área)
/// y sus áreas deben sumar el área de `r`.
fn squarify(items: &[(u32, f64)], mut r: Rect, mut emit: impl FnMut(u32, Rect)) {
    let mut i = 0;
    while i < items.len() {
        let [x, y, w, h] = r;
        let short = w.min(h);
        if short <= 0.0 {
            break;
        }
        let mut sum = items[i].1;
        let (mut min, mut max) = (sum, sum);
        let mut worst = worst_ratio(short, sum, min, max);
        let mut j = i + 1;
        while j < items.len() {
            let a = items[j].1;
            let (nsum, nmin, nmax) = (sum + a, min.min(a), max.max(a));
            let nworst = worst_ratio(short, nsum, nmin, nmax);
            if nworst > worst {
                break;
            }
            (sum, min, max, worst) = (nsum, nmin, nmax, nworst);
            j += 1;
        }
        let thick = sum / short;
        if w >= h {
            let mut cy = y;
            for &(id, a) in &items[i..j] {
                let ch = a / thick;
                emit(id, [x, cy, thick, ch]);
                cy += ch;
            }
            r = [x + thick, y, (w - thick).max(0.0), h];
        } else {
            let mut cx = x;
            for &(id, a) in &items[i..j] {
                let cw = a / thick;
                emit(id, [cx, y, cw, thick]);
                cx += cw;
            }
            r = [x, y + thick, w, (h - thick).max(0.0)];
        }
        i = j;
    }
}

/// Color del archivo más grande siguiendo siempre el primer hijo vivo.
fn representative_color(tree: &Tree, colors: &[[u8; 3]], mut id: u32) -> [u8; 3] {
    loop {
        let n = &tree.nodes[id as usize];
        if !n.is_dir {
            return colors[n.ext as usize];
        }
        let biggest = tree
            .children(id)
            .filter(|&c| !tree.nodes[c as usize].deleted)
            .max_by_key(|&c| tree.nodes[c as usize].size);
        match biggest {
            Some(c) if tree.nodes[c as usize].size > 0 => id = c,
            _ => return OTHER_COLOR,
        }
    }
}

pub fn layout(tree: &Tree, colors: &[[u8; 3]], root: u32, w: u32, h: u32) -> Layout {
    let mut l = Layout {
        root,
        w,
        h,
        owner: vec![NO_LEAF; (w * h) as usize],
        leaves: Vec::new(),
        rects: HashMap::new(),
    };

    let mut stack: Vec<(u32, Rect, [f64; 4], f64)> =
        vec![(root, [0.0, 0.0, w as f64, h as f64], [0.0; 4], CUSHION_HEIGHT)];
    let mut items: Vec<(u32, f64)> = Vec::new();

    while let Some((id, r, surface, height)) = stack.pop() {
        l.rects.insert(id, r.map(|v| v as f32));
        let n = &tree.nodes[id as usize];
        if !n.is_dir {
            l.push_leaf(id, r, surface, colors[n.ext as usize], n.ext);
            continue;
        }
        if r[2] * r[3] < 6.0 || r[2].min(r[3]) < 1.5 {
            l.push_leaf(id, r, surface, representative_color(tree, colors, id), NO_EXT);
            continue;
        }

        items.clear();
        items.extend(tree.children(id).filter_map(|c| {
            let cn = &tree.nodes[c as usize];
            (!cn.deleted && cn.size > 0).then_some((c, cn.size as f64))
        }));
        if items.is_empty() {
            continue;
        }
        if !items.is_sorted_by(|a, b| a.1 >= b.1) {
            items.sort_unstable_by(|a, b| b.1.total_cmp(&a.1));
        }
        let total: f64 = items.iter().map(|i| i.1).sum();
        let scale = r[2] * r[3] / total;
        for item in &mut items {
            item.1 *= scale;
        }
        // Los hijos diminutos se agrupan en un bloque que pertenece al directorio.
        let cut = items.partition_point(|i| i.1 >= MIN_CHILD_AREA);
        if cut < items.len() {
            let rest: f64 = items[cut..].iter().map(|i| i.1).sum();
            items.truncate(cut);
            let pos = items.partition_point(|i| i.1 >= rest);
            items.insert(pos, (REMAINDER, rest));
        }

        let mut pending: Vec<(u32, Rect)> = Vec::with_capacity(items.len());
        squarify(&items, r, |child, rect| pending.push((child, rect)));
        for (child, rect) in pending {
            let mut s = surface;
            add_ridge(&mut s, rect, height);
            if child == REMAINDER {
                l.push_leaf(id, rect, s, OTHER_COLOR, NO_EXT);
            } else {
                stack.push((child, rect, s, height * CUSHION_SCALE));
            }
        }
    }
    l
}

/// Devuelve la imagen RGBA. Con `highlight`, atenúa todo lo que no sea de esa extensión.
pub fn render(l: &Layout, highlight: Option<u16>) -> Vec<u8> {
    let w = l.w as usize;
    let mut buf = vec![0u8; w * l.h as usize * 4];
    buf.par_chunks_mut(w * 4).enumerate().for_each(|(y, row)| {
        let owners = &l.owner[y * w..(y + 1) * w];
        let fy = y as f32 + 0.5;
        for (x, px) in row.chunks_exact_mut(4).enumerate() {
            let rgb = match owners[x] {
                NO_LEAF => BACKGROUND,
                idx => {
                    let leaf = &l.leaves[idx as usize];
                    let s = &leaf.surface;
                    let nx = -(2.0 * s[0] * (x as f32 + 0.5) + s[2]);
                    let ny = -(2.0 * s[1] * fy + s[3]);
                    let cos = (nx * LIGHT[0] + ny * LIGHT[1] + LIGHT[2])
                        / (nx * nx + ny * ny + 1.0).sqrt();
                    let mut b = AMBIENT + DIFFUSE * cos.max(0.0);
                    if highlight.is_some_and(|h| h != leaf.ext) {
                        b *= 0.22;
                    }
                    leaf.color.map(|c| (c as f32 * b).min(255.0) as u8)
                }
            };
            px[..3].copy_from_slice(&rgb);
            px[3] = 255;
        }
    });
    buf
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::filetypes::assign_colors;
    use crate::tree::tests::{dir, file, sample};
    use std::path::PathBuf;

    fn squarified(areas: &[f64], r: Rect) -> Vec<Rect> {
        let items: Vec<(u32, f64)> = areas.iter().enumerate().map(|(i, &a)| (i as u32, a)).collect();
        let mut out = vec![[0.0; 4]; areas.len()];
        squarify(&items, r, |id, rect| out[id as usize] = rect);
        out
    }

    #[test]
    fn squarify_preserves_areas_and_bounds() {
        let areas = [6.0, 6.0, 4.0, 3.0, 2.0, 2.0, 1.0];
        let rects = squarified(&areas, [0.0, 0.0, 6.0, 4.0]);
        for (r, a) in rects.iter().zip(areas) {
            assert!((r[2] * r[3] - a).abs() < 1e-9, "área {r:?} != {a}");
            assert!(r[0] >= -1e-9 && r[1] >= -1e-9);
            assert!(r[0] + r[2] <= 6.0 + 1e-9 && r[1] + r[3] <= 4.0 + 1e-9);
        }
    }

    #[test]
    fn squarify_rects_do_not_overlap() {
        let areas = [6.0, 6.0, 4.0, 3.0, 2.0, 2.0, 1.0];
        let rects = squarified(&areas, [0.0, 0.0, 6.0, 4.0]);
        for (i, a) in rects.iter().enumerate() {
            for b in &rects[i + 1..] {
                let ox = (a[0] + a[2]).min(b[0] + b[2]) - a[0].max(b[0]);
                let oy = (a[1] + a[3]).min(b[1] + b[3]) - a[1].max(b[1]);
                assert!(ox <= 1e-9 || oy <= 1e-9, "{a:?} solapa con {b:?}");
            }
        }
    }

    #[test]
    fn squarify_matches_paper_example() {
        // Ejemplo del artículo original: el primer bloque de 6 ocupa 3x2.
        let rects = squarified(&[6.0, 6.0, 4.0, 3.0, 2.0, 2.0, 1.0], [0.0, 0.0, 6.0, 4.0]);
        assert_eq!(rects[0], [0.0, 0.0, 3.0, 2.0]);
        assert_eq!(rects[1], [0.0, 2.0, 3.0, 2.0]);
    }

    #[test]
    fn layout_covers_every_pixel_proportionally() {
        let t = sample();
        let colors = assign_colors(&t);
        let l = layout(&t, &colors, 0, 200, 100);
        assert!(l.owner.iter().all(|&o| o != NO_LEAF));

        let video = t.nodes.iter().position(|n| &*n.name == "video.mp4").unwrap() as u32;
        let px = l.owner.iter().filter(|&&o| l.leaves[o as usize].node == video).count() as f64;
        let expected = 20_000.0 * 1000.0 / 1515.0;
        assert!((px - expected).abs() / expected < 0.03, "{px} vs {expected}");

        let r = l.rect(video).unwrap();
        assert_eq!(l.node_at((r[0] + r[2] / 2.0) as u32, (r[1] + r[3] / 2.0) as u32), Some(video));
        assert_eq!(l.node_at(200, 0), None);
    }

    #[test]
    fn tiny_children_collapse_into_parent_block() {
        let many: Vec<_> = (0..5000).map(|i| file(&format!("f{i}.log"), 1)).collect();
        let t = Tree::build(PathBuf::from(r"C:\x"), vec![dir("logs", many), file("grande.iso", 5000)]);
        let colors = assign_colors(&t);
        let l = layout(&t, &colors, 0, 40, 20);
        assert!(l.owner.iter().all(|&o| o != NO_LEAF));
        assert!(l.leaves.len() < 20, "se esperaba agrupar, hay {} hojas", l.leaves.len());
        let logs = t.nodes.iter().position(|n| &*n.name == "logs").unwrap() as u32;
        assert!(l.leaves.iter().any(|leaf| leaf.node == logs));
    }

    #[test]
    fn layout_skips_deleted_nodes() {
        let mut t = sample();
        let video = t.nodes.iter().position(|n| &*n.name == "video.mp4").unwrap() as u32;
        t.remove(video);
        let colors = assign_colors(&t);
        let l = layout(&t, &colors, 0, 100, 100);
        assert!(l.owner.iter().all(|&o| o != NO_LEAF && l.leaves[o as usize].node != video));
    }

    #[test]
    fn render_dims_non_highlighted_extensions() {
        let t = sample();
        let colors = assign_colors(&t);
        let l = layout(&t, &colors, 0, 64, 64);
        let plain = render(&l, None);
        assert_eq!(plain.len(), 64 * 64 * 4);
        assert!(plain.chunks_exact(4).all(|p| p[3] == 255));

        let mp4 = t.exts.iter().position(|e| e == "mp4").unwrap() as u16;
        let lit = render(&l, Some(mp4));
        let sum = |b: &[u8]| b.iter().map(|&v| v as u64).sum::<u64>();
        assert!(sum(&lit) < sum(&plain));
        let video = t.nodes.iter().position(|n| &*n.name == "video.mp4").unwrap() as u32;
        let r = l.rect(video).unwrap();
        let i = ((r[1] + r[3] / 2.0) as usize * 64 + (r[0] + r[2] / 2.0) as usize) * 4;
        assert_eq!(plain[i..i + 4], lit[i..i + 4]);
    }
}
