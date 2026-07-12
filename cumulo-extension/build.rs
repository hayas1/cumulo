use std::fs;

use resvg::tiny_skia::{Pixmap, Transform};
use resvg::usvg;

const FAVICON: &str = "../cumulo-web/public/favicon.svg";
const OUT_DIR: &str = "public/icons";
const SIZES: [u32; 4] = [16, 32, 48, 128];

fn main() {
    println!("cargo:rerun-if-changed={FAVICON}");
    println!("cargo:rerun-if-changed=build.rs");

    let svg = fs::read(FAVICON).expect("failed to read favicon.svg");
    let tree = usvg::Tree::from_data(&svg, &usvg::Options::default()).expect("failed to parse SVG");

    fs::create_dir_all(OUT_DIR).unwrap();
    for size in SIZES {
        fs::write(format!("{OUT_DIR}/icon{size}.png"), render(&tree, size)).unwrap();
    }
}

fn render(tree: &usvg::Tree, size: u32) -> Vec<u8> {
    let mut pixmap = Pixmap::new(size, size).unwrap();
    let scale = size as f32 / tree.size().width();
    resvg::render(
        tree,
        Transform::from_scale(scale, scale),
        &mut pixmap.as_mut(),
    );
    pixmap.encode_png().expect("failed to encode PNG")
}
