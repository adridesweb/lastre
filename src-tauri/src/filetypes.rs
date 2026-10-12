use crate::tree::Tree;

/// Colores para las extensiones que más ocupan, por orden de tamaño.
const PALETTE: [[u8; 3]; 14] = [
    [0x4e, 0x79, 0xff],
    [0xf2, 0x54, 0x5b],
    [0x3c, 0xcb, 0x7f],
    [0xf5, 0xc5, 0x42],
    [0x36, 0xc5, 0xf0],
    [0xc6, 0x6b, 0xff],
    [0xff, 0x8a, 0x3d],
    [0xff, 0x6f, 0xb5],
    [0x8b, 0xd4, 0x50],
    [0x2f, 0xb8, 0xac],
    [0xa3, 0xa9, 0xff],
    [0xd9, 0xa0, 0x66],
    [0x7a, 0x8c, 0xc8],
    [0xe0, 0xe0, 0x6a],
];
pub const OTHER_COLOR: [u8; 3] = [0x8c, 0x8c, 0x96];
pub const NO_EXT_COLOR: [u8; 3] = [0x6e, 0x6e, 0x78];

#[derive(Clone, Copy, Default)]
pub struct ExtStat {
    pub size: u64,
    pub count: u32,
}

/// Tamaño y número de archivos por extensión dentro del subárbol de `root`.
pub fn stats(tree: &Tree, root: u32) -> Vec<ExtStat> {
    let mut out = vec![ExtStat::default(); tree.exts.len()];
    let mut stack = vec![root];
    while let Some(id) = stack.pop() {
        let n = &tree.nodes[id as usize];
        if n.deleted {
            continue;
        }
        if n.is_dir {
            stack.extend(tree.children(id));
        } else {
            let s = &mut out[n.ext as usize];
            s.size += n.size;
            s.count += 1;
        }
    }
    out
}

/// Color de cada extensión (indexado por id), según su peso en todo el árbol.
pub fn assign_colors(tree: &Tree) -> Vec<[u8; 3]> {
    let stats = stats(tree, 0);
    let mut ranked: Vec<usize> = (1..stats.len()).collect();
    ranked.sort_unstable_by(|&a, &b| stats[b].size.cmp(&stats[a].size));
    let mut colors = vec![OTHER_COLOR; stats.len()];
    colors[0] = NO_EXT_COLOR;
    for (ext, color) in ranked.into_iter().zip(PALETTE) {
        colors[ext] = color;
    }
    colors
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tree::tests::sample;

    #[test]
    fn stats_per_extension() {
        let t = sample();
        let s = stats(&t, 0);
        let pdf = t.exts.iter().position(|e| e == "pdf").unwrap();
        assert_eq!((s[pdf].size, s[pdf].count), (500, 2));
        assert_eq!((s[0].size, s[0].count), (5, 1));
        assert_eq!(s.iter().map(|e| e.size).sum::<u64>(), t.nodes[0].size);
    }

    #[test]
    fn biggest_extension_gets_first_palette_color() {
        let t = sample();
        let colors = assign_colors(&t);
        let mp4 = t.exts.iter().position(|e| e == "mp4").unwrap();
        assert_eq!(colors[mp4], PALETTE[0]);
        assert_eq!(colors[0], NO_EXT_COLOR);
    }
}
