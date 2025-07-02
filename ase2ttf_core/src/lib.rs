use asefile::AsepriteFile;
use chrono::Utc;
use kurbo::BezPath;
use std::fmt::{Debug, Display};
use std::path::Path;
use write_fonts::tables::cmap::{Cmap, CmapSubtable, EncodingRecord};
use write_fonts::tables::glyf::{GlyfLocaBuilder, Glyph};
use write_fonts::tables::hhea::Hhea;
use write_fonts::tables::hmtx::Hmtx;
use write_fonts::tables::maxp::Maxp;
use write_fonts::tables::os2::{Os2, SelectionFlags};
use write_fonts::tables::post::Post;
use write_fonts::tables::vmtx::LongMetric;
use write_fonts::types::{Tag};
use write_fonts::{
    OffsetMarker,
    tables::{
        cmap::PlatformId,
        glyf::SimpleGlyph,
        head::{Head, MacStyle},
        name::{Name, NameRecord},
    },
    types::{Fixed, LongDateTime, NameId},
};

#[cfg(feature = "wasm")] use wasm_bindgen::prelude::*;

use crate::edge::get_edges;

mod edge;
#[cfg_attr(feature = "wasm", wasm_bindgen(getter_with_clone))]
pub struct Params {
    pub file_path: String,
    pub copyright: Option<String>,
    pub family: Option<String>,
    pub subfamily: Option<String>,
    pub font_version: Option<String>,
    pub font_weight: Option<u16>,
    pub glyph_width: Option<u32>,
    pub glyph_height: Option<u32>,
    pub trim: Option<bool>,
    pub trim_pad: Option<u32>,
    pub line_gap: Option<u8>,
    pub baseline: Option<u8>,
}

#[cfg_attr(feature = "wasm", wasm_bindgen)]
impl Params {
    #[cfg_attr(feature = "wasm", wasm_bindgen(constructor))]
    pub fn new(
        file_path: String,
        copyright: Option<String>,
        family: Option<String>,
        subfamily: Option<String>,
        font_version: Option<String>,
        font_weight: Option<u16>,
        glyph_width: Option<u32>,
        glyph_height: Option<u32>,
        trim: Option<bool>,
        trim_pad: Option<u32>,
        line_gap: Option<u8>,
        baseline: Option<u8>,
    ) -> Params {
        Params {
            file_path,
            copyright,
            family,
            subfamily,
            font_version,
            font_weight,
            glyph_width,
            glyph_height,
            trim,
            trim_pad,
            line_gap,
            baseline,
        }
    }
}

#[cfg_attr(feature = "wasm", wasm_bindgen(getter_with_clone))]
pub struct Error {
    pub message: String,
}

impl Error {
    pub fn new(message: String) -> Error {
        Error { message: message }
    }
}

impl Debug for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        None
    }

    fn description(&self) -> &str {
        "description() is deprecated; use Display"
    }

    fn cause(&self) -> Option<&dyn std::error::Error> {
        self.source()
    }
}

#[cfg(feature = "wasm")]
#[wasm_bindgen]
pub fn generate_ttf_js(ase_bytes: &[u8], args: Params) -> Result<Vec<u8>, JsValue> {
    generate_ttf(ase_bytes, args).map_err(|x| x.into())
}

pub fn generate_ttf(ase_bytes: &[u8], args: Params) -> Result<Vec<u8>, Error> {
    let ase = AsepriteFile::read(ase_bytes).map_err(|e| Error::new(e.to_string()))?;

    // params
    let glyph_width = args.glyph_width.unwrap_or(16);
    let glyph_height = args.glyph_height.unwrap_or(16);
    let scale = 64.0 / glyph_width as f64;
    let file_stem = Path::new(&args.file_path)
        .file_stem()
        .unwrap()
        .to_str()
        .unwrap()
        .to_string();

    // validate size
    let width = ase.width() as u32;
    let height = ase.height() as u32;
    if width % glyph_width != 0 || height % glyph_height != 0 {
        panic!(
            "The height and width of the aseprite file must be multiples of glyph-width and glyph-height respectively."
        )
    }

    let mut builder = write_fonts::FontBuilder::new();

    // build glyph
    let mut glyf_builder = GlyfLocaBuilder::new();
    let mut cmap_entries = vec![];
    let mut glyph_widths = vec![];
    let mut glyph_names = vec![];
    let mut glyph_count = 0;
    let mut max_point: u16 = 0;
    let mut max_contour_count: u16 = 0;

    // add .notdef / null / space
    glyf_builder.add_glyph(&SimpleGlyph::default()).unwrap();
    glyf_builder.add_glyph(&SimpleGlyph::default()).unwrap();
    glyf_builder.add_glyph(&SimpleGlyph::default()).unwrap();
    glyph_widths.push(64);
    glyph_widths.push(64);
    glyph_widths.push(64);
    glyph_names.push(".notdef".to_string());
    glyph_names.push("null".to_string());
    glyph_names.push("space".to_string());
    glyph_count += 3;

    for layer in ase.layers() {
        let image = layer.frame(0).image();
        let name = layer.name();
        let base_code = if name.starts_with("U+") || name.starts_with("u+") {
            let hex_part: String = name[2..]
                .chars()
                .take_while(|c| c.is_ascii_hexdigit())
                .collect();
            if let Ok(s) = u32::from_str_radix(&hex_part, 16) {
                s
            } else {
                continue;
            }
        } else {
            continue;
        };

        let cols = width / glyph_width;
        let rows = height / glyph_height;
        for row in 0..rows {
            for col in 0..cols {
                let x0 = col * glyph_width;
                let y0 = row * glyph_height;

                let mut bitmap = vec![0.0f64; (glyph_width * glyph_height) as usize];
                for y in 0..glyph_height {
                    for x in 0..glyph_width {
                        let px = x0 + x;
                        let py = y0 + y;
                        if px >= width || py >= height {
                            continue;
                        }
                        let pixel = image.get_pixel(px, py);
                        bitmap[(y * glyph_width + x) as usize] = pixel[3] as f64 / 256.0;
                    }
                }

                let mut point: u16 = 0;
                let mut contour_count: u16 = 0;
                let mut path = BezPath::new();

                let boundaries = get_edges(&bitmap, glyph_width as usize, glyph_height as usize);
                for edges in boundaries.values() {
                    let paths = crate::edge::edges_to_paths(edges);
                    for path_points in paths {
                        if path_points.is_empty() {
                            continue;
                        }
                        let mut iter = path_points.iter();
                        if let Some(&(x0, y0)) = iter.next() {
                            path.move_to((
                                x0 as f64 * scale,
                                (glyph_height as usize - y0) as f64 * scale - 24.0,
                            ));
                            for &(x, y) in iter {
                                path.line_to((
                                    x as f64 * scale,
                                    (glyph_height as usize - y) as f64 * scale - 24.0,
                                ));
                                point += 1;
                            }
                            path.close_path();
                            contour_count += 1;
                        }
                    }
                }

                if point == 0 {
                    continue;
                }

                glyf_builder
                    .add_glyph(&Glyph::Simple(SimpleGlyph::from_bezpath(&path).unwrap()))
                    .unwrap();
                let codepoint = base_code + (row * cols + col) as u32;
                cmap_entries.push((codepoint, glyph_count));
                glyph_count += 1;
                glyph_names.push(format!("U+{:x>04}", codepoint));

                max_point = if point > max_point { point } else { max_point };
                max_contour_count = if contour_count > max_contour_count {
                    contour_count
                } else {
                    max_contour_count
                };

                if args.trim.unwrap_or(true) {
                    let mut min_x = glyph_width;
                    let mut max_x = 0;
                    for y in 0..glyph_height {
                        for x in 0..glyph_width {
                            let px = x0 + x;
                            let py = y0 + y;
                            if px >= width || py >= height {
                                continue;
                            }
                            let pixel = image.get_pixel(px, py);
                            if pixel[3] != 0 {
                                if x < min_x {
                                    min_x = x;
                                }
                                if x > max_x {
                                    max_x = x;
                                }
                            }
                        }
                    }
                    let trimmed_width = if min_x > max_x {
                        0
                    } else {
                        max_x - min_x + 1 + args.trim_pad.unwrap_or(1)
                    };
                    let scaled_width =
                        ((trimmed_width as f64) * (64.0 / glyph_width as f64)).round() as u16;
                    glyph_widths.push(scaled_width);
                } else {
                    glyph_widths.push(64);
                }
            }
        }
    }

    if glyph_count <= 3 {
        return Err(Error::new(
            "No valid layer found. Parsable layer names must start with U+ and be valid Unicode."
                .to_string(),
        ));
    }

    // head table
    let head = Head::new(
        Fixed::from(0),
        0,
        0b0000000000001011,
        64,
        LongDateTime::new(Utc::now().timestamp()),
        LongDateTime::new(Utc::now().timestamp()),
        0,
        -16,
        64,
        48,
        MacStyle::empty(),
        8,
        0,
    );
    builder
        .add_table(&head)
        .map_err(|e| Error::new(e.to_string()))?;

    // name table
    let font_name = args.family.unwrap_or(file_stem.clone());
    let mut name_records = Vec::new();
    for i in 0..1 {
        let platform_id = match i {
            0 => PlatformId::Macintosh,
            1 => PlatformId::Windows,
            _ => unreachable!(),
        } as u16;

        let encoding_id = match i {
            0 => 0,
            1 => 1,
            _ => unreachable!(),
        };

        // 0: copyright
        if let Some(copyright) = args.copyright.clone() {
            name_records.push(NameRecord {
                platform_id: platform_id,
                encoding_id: encoding_id,
                language_id: 0,
                name_id: NameId::from(0),
                string: OffsetMarker::new(copyright),
            });
        }

        // 1: font family name
        name_records.push(NameRecord {
            platform_id: platform_id,
            encoding_id: encoding_id,
            language_id: 0,
            name_id: NameId::from(1),
            string: OffsetMarker::new(font_name.clone()),
        });

        // 2: subfamily name
        name_records.push(NameRecord {
            platform_id: platform_id,
            encoding_id: encoding_id,
            language_id: 0,
            name_id: NameId::from(2),
            string: OffsetMarker::new(args.subfamily.clone().unwrap_or("Regular".to_string())),
        });

        // 3: identifier
        name_records.push(NameRecord {
            platform_id: platform_id,
            encoding_id: encoding_id,
            language_id: 0,
            name_id: NameId::from(3),
            string: OffsetMarker::new(format!("ase2ttf: {}", font_name.clone())),
        });

        // 4: font name
        name_records.push(NameRecord {
            platform_id: platform_id,
            encoding_id: encoding_id,
            language_id: 0,
            name_id: NameId::from(4),
            string: OffsetMarker::new(font_name.clone()),
        });

        // 5: version
        name_records.push(NameRecord {
            platform_id: platform_id,
            encoding_id: encoding_id,
            language_id: 0,
            name_id: NameId::from(5),
            string: OffsetMarker::new(format!(
                "Version {}",
                args.font_version.clone().unwrap_or("1.0".to_string())
            )),
        });

        // 6: PostScript name
        name_records.push(NameRecord {
            platform_id: platform_id,
            encoding_id: encoding_id,
            language_id: 0,
            name_id: NameId::from(6),
            string: OffsetMarker::new(font_name.clone()),
        });
    }

    let name = Name::new(name_records);
    builder
        .add_table(&name)
        .map_err(|e| Error::new(e.to_string()))?;

    // OS/2 table
    let os2 = Os2 {
        x_avg_char_width: 31,
        us_weight_class: if let Some(weight_class) = args.font_weight {
            weight_class
        } else {
            match args
                .subfamily
                .as_deref()
                .unwrap_or("regular")
                .to_lowercase()
                .as_str()
            {
                "thin" => 100,
                "extra-light" | "extralight" | "ultra-light" | "ultralight" => 200,
                "light" => 300,
                "regular" => 400,
                "medium" => 500,
                "semibold" | "semi-bold" | "demi-bold" | "demibold" => 600,
                "bold" => 700,
                "extrabold" | "extra-bold" | "ultrabold" | "ultra-bold" => 800,
                "black" | "heavy" => 900,
                _ => 400,
            }
        },
        us_width_class: 5,
        fs_type: 0b0000_0000_0000_0000,
        y_subscript_x_size: 32,
        y_subscript_y_size: 32,
        y_subscript_x_offset: 0,
        y_subscript_y_offset: 32,
        y_superscript_x_size: 32,
        y_superscript_y_size: 32,
        y_superscript_x_offset: 0,
        y_superscript_y_offset: 32,
        y_strikeout_size: 8,
        y_strikeout_position: 24,
        s_family_class: 0,
        panose_10: [0; 10],
        ul_unicode_range_1: 0,
        ul_unicode_range_2: 0,
        ul_unicode_range_3: 0,
        ul_unicode_range_4: 0,
        ach_vend_id: Tag::from_u32(0),
        fs_selection: SelectionFlags::empty(),
        us_first_char_index: 0,
        us_last_char_index: 57,
        s_typo_ascender: 48,
        s_typo_descender: -16,
        s_typo_line_gap: 0,
        us_win_ascent: 48,
        us_win_descent: 16,
        ul_code_page_range_1: Default::default(),
        ul_code_page_range_2: Default::default(),
        sx_height: Default::default(),
        s_cap_height: Default::default(),
        us_default_char: Default::default(),
        us_break_char: Default::default(),
        us_max_context: Default::default(),
        us_lower_optical_point_size: Default::default(),
        us_upper_optical_point_size: Default::default(),
    };
    builder
        .add_table(&os2)
        .map_err(|e| Error::new(e.to_string()))?;

    // maxp table
    let maxp = Maxp {
        num_glyphs: glyph_count,
        max_points: Some(max_point),
        max_contours: Some(max_contour_count),
        max_composite_points: Some(0),
        max_composite_contours: Some(0),
        max_zones: Some(2),
        max_twilight_points: Some(0),
        max_storage: Some(1),
        max_function_defs: Some(1),
        max_instruction_defs: Some(0),
        max_stack_elements: Some(64),
        max_size_of_instructions: Some(0),
        max_component_elements: Some(0),
        max_component_depth: Some(0),
    };
    builder
        .add_table(&maxp)
        .map_err(|e| Error::new(e.to_string()))?;

    // post table
    let glyph_name_refs: Vec<&str> = glyph_names.iter().map(|s| s.as_str()).collect();
    let post = Post::new_v2(glyph_name_refs);
    builder
        .add_table(&post)
        .map_err(|e| Error::new(e.to_string()))?;

    // cmap table
    let mut start_code = Vec::new();
    let mut end_code = Vec::new();
    let mut id_delta = Vec::new();
    let mut id_range_offsets = Vec::new();
    let glyph_id_array = Vec::new();
    for (codepoint, glyph_id) in &cmap_entries {
        let unicode = *codepoint as u16;
        start_code.push(unicode);
        end_code.push(unicode);
        id_delta.push((*glyph_id as i32 - unicode as i32) as i16);
        id_range_offsets.push(0);
    }
    start_code.push(0xFFFF);
    end_code.push(0xFFFF);
    id_delta.push(1);
    id_range_offsets.push(0);

    let subtable = CmapSubtable::format_4(
        0,
        end_code,
        start_code,
        id_delta,
        id_range_offsets,
        glyph_id_array,
    );

    let cmap = Cmap::new(vec![
        EncodingRecord {
            platform_id: PlatformId::Unicode,
            encoding_id: 3,
            subtable: OffsetMarker::new(subtable.clone()),
        },
        EncodingRecord {
            platform_id: PlatformId::Macintosh,
            encoding_id: 0,
            subtable: OffsetMarker::new(subtable.clone()),
        },
        EncodingRecord {
            platform_id: PlatformId::Windows,
            encoding_id: 1,
            subtable: OffsetMarker::new(subtable),
        },
    ]);
    builder
        .add_table(&cmap)
        .map_err(|e| Error::new(e.to_string()))?;

    // hhea table
    let base_line = (args.baseline.unwrap_or(2) as f64 * scale).round() as i16;
    let line_gap = (args.line_gap.unwrap_or(0) as f64 * scale).round() as i16;
    let hhea = Hhea::new(
        (64 - base_line).into(),
        (-base_line).into(),
        line_gap.into(),
        64.into(),
        0.into(),
        0.into(),
        64.into(),
        1,
        0,
        0,
        glyph_count,
    );
    builder
        .add_table(&hhea)
        .map_err(|e| Error::new(e.to_string()))?;

    // hmtx table
    let hmtx = Hmtx::new(
        glyph_widths
            .iter()
            .map(|x| LongMetric::new(*x, 8))
            .collect(),
        vec![],
    );
    builder
        .add_table(&hmtx)
        .map_err(|e| Error::new(e.to_string()))?;

    // glyf / loca table
    let (glyf, loca, _) = glyf_builder.build();
    builder
        .add_table(&glyf)
        .map_err(|e| Error::new(e.to_string()))?;
    builder
        .add_table(&loca)
        .map_err(|e| Error::new(e.to_string()))?;

    Ok(builder.build())
}
