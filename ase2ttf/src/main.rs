use std::{
    fs::{self, File},
    io::Write,
    path::Path,
};

use ase2ttf_core::{Params, generate_ttf};
use clap::Parser;

#[derive(Debug, Parser)]
#[command(version = "0.1.0", about, long_about = None)]
struct Args {
    path: String,

    #[arg(short, long)]
    output: Option<String>,

    #[arg(long)]
    copyright: Option<String>,

    #[arg(long)]
    name: Option<String>,

    #[arg(long)]
    family: Option<String>,

    #[arg(long)]
    font_version: Option<String>,

    #[arg(long, require_equals = true)]
    font_weight: Option<u16>,

    #[arg(long, require_equals = true, default_value_t = 16)]
    glyph_width: u32,

    #[arg(long, require_equals = true, default_value_t = 16)]
    glyph_height: u32,

    #[arg(long, default_value_t = false)]
    trim: bool,

    #[arg(long, require_equals = true, default_value_t = 1)]
    trim_pad: u32,
}

fn main() {
    let args = Args::parse();
    let path = Path::new(&args.path);

    let ase_bytes = fs::read(path).unwrap();
    let ttf_bytes = generate_ttf(
        &ase_bytes,
        Params {
            file_path: args.path.clone(),
            copyright: args.copyright,
            name: args.name,
            family: args.family,
            font_version: args.font_version,
            font_weight: args.font_weight,
            glyph_width: args.glyph_width,
            glyph_height: args.glyph_height,
            trim: args.trim,
            trim_pad: args.trim_pad,
        },
    ).unwrap();

    let file_stem = Path::new(&args.path)
        .file_stem()
        .unwrap()
        .to_str()
        .unwrap()
        .to_string();

    let mut file = File::create(args.output.unwrap_or(format!("{0}.ttf", file_stem))).unwrap();
    file.write_all(&ttf_bytes).expect("Failed to write file.");
    file.flush().expect("Failed to write file.");
}
