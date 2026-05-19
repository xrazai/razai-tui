use std::{fs::OpenOptions, io::Write, os::windows::fs::OpenOptionsExt, path::Path};

use chrono::Local;
use printpdf::{
    Color, Line, LinePoint, Op, PaintMode, ParsedFont, PdfDocument, PdfPage, PdfSaveOptions, Point,
    Polygon, PolygonRing, Pt, Rgb, TextItem, WindingOrder, ops::PdfFontHandle, units::Mm,
};

use crate::db::{TecidoRecord, VinculoRecord};
use crate::models::parse_hex_color;

const IBM_PLEX_SANS_REGULAR: &[u8] =
    include_bytes!("../../../assets/fonts/IBMPlexSans-Regular.ttf");
const IBM_PLEX_SANS_BOLD: &[u8] = include_bytes!("../../../assets/fonts/IBMPlexSans-Bold.ttf");

pub(super) struct ChecklistSection {
    pub tecido: TecidoRecord,
    pub vinculos: Vec<VinculoRecord>,
}

pub(super) fn write_checklist_pdf(
    path: &Path,
    sections: &[ChecklistSection],
) -> Result<(), String> {
    const FILE_SHARE_READ: u32 = 0x00000001;
    const FILE_SHARE_WRITE: u32 = 0x00000002;
    const FILE_SHARE_DELETE: u32 = 0x00000004;

    let bytes = checklist_pdf_bytes(sections)?;
    let mut file = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .share_mode(FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE)
        .open(path)
        .map_err(|error| format!("falha ao criar PDF: {error}"))?;
    file.write_all(&bytes)
        .map_err(|error| format!("falha ao gravar PDF: {error}"))?;
    file.sync_all()
        .map_err(|error| format!("falha ao finalizar PDF: {error}"))
}

fn checklist_pdf_bytes(sections: &[ChecklistSection]) -> Result<Vec<u8>, String> {
    let mut font_warnings = Vec::new();
    let regular = ParsedFont::from_bytes(IBM_PLEX_SANS_REGULAR, 0, &mut font_warnings)
        .ok_or_else(|| String::from("falha ao carregar IBM Plex Sans Regular"))?;
    let bold = ParsedFont::from_bytes(IBM_PLEX_SANS_BOLD, 0, &mut font_warnings)
        .ok_or_else(|| String::from("falha ao carregar IBM Plex Sans Bold"))?;

    let mut pdf = PdfDocument::new("Checklist de Vinculos Razai");
    let fonts = PdfFonts {
        regular: PdfFontHandle::External(pdf.add_font(&regular)),
        bold: PdfFontHandle::External(pdf.add_font(&bold)),
    };
    let mut pages = Vec::new();
    let mut canvas = PdfCanvas::new(fonts.clone());
    let mut page_number = 1usize;
    let mut y = PAGE_TOP;

    draw_header(&mut canvas, page_number);
    for section in sections {
        if y - MIN_SECTION_HEIGHT < PAGE_BOTTOM {
            draw_footer(&mut canvas, page_number);
            pages.push(PdfPage::new(Mm(210.0), Mm(297.0), canvas.into_ops()));
            page_number += 1;
            canvas = PdfCanvas::new(fonts.clone());
            draw_header(&mut canvas, page_number);
            y = PAGE_TOP;
        }

        let mut row_y = draw_section_header(&mut canvas, section, y, false);
        if section.vinculos.is_empty() {
            draw_empty_row(&mut canvas, row_y);
            y = row_y - ROW_HEIGHT - 12.0;
            continue;
        }

        for (index, vinculo) in section.vinculos.iter().enumerate() {
            if row_y - ROW_HEIGHT < PAGE_BOTTOM {
                draw_footer(&mut canvas, page_number);
                pages.push(PdfPage::new(Mm(210.0), Mm(297.0), canvas.into_ops()));
                page_number += 1;
                canvas = PdfCanvas::new(fonts.clone());
                draw_header(&mut canvas, page_number);
                row_y = draw_section_header(&mut canvas, section, PAGE_TOP, true);
            }
            draw_row(&mut canvas, vinculo, row_y, index % 2 == 0);
            row_y -= ROW_HEIGHT;
        }
        y = row_y - 12.0;
    }
    draw_footer(&mut canvas, page_number);
    pages.push(PdfPage::new(Mm(210.0), Mm(297.0), canvas.into_ops()));

    let mut save_warnings = Vec::new();
    Ok(pdf
        .with_pages(pages)
        .save(&PdfSaveOptions::default(), &mut save_warnings))
}

const PAGE_TOP: f64 = 718.0;
const PAGE_BOTTOM: f64 = 72.0;
const LEFT: f64 = 42.0;
const WIDTH: f64 = 511.0;
const SECTION_HEADER_HEIGHT: f64 = 30.0;
const TABLE_HEADER_HEIGHT: f64 = 22.0;
const ROW_HEIGHT: f64 = 50.0;
const THUMB: f64 = 42.5;
const MIN_SECTION_HEIGHT: f64 = SECTION_HEADER_HEIGHT + TABLE_HEADER_HEIGHT + ROW_HEIGHT;

fn draw_header(canvas: &mut PdfCanvas, page: usize) {
    canvas.text(FontKind::Bold, 24.0, LEFT, 780.0, "RAZAI", 0.10);
    canvas.text(
        FontKind::Bold,
        14.0,
        LEFT,
        754.0,
        "Checklist de Vinculos",
        0.12,
    );
    canvas.text(
        FontKind::Regular,
        9.0,
        392.0,
        780.0,
        &format!("Gerado em {}", Local::now().format("%d/%m/%Y %H:%M")),
        0.35,
    );
    canvas.text(
        FontKind::Regular,
        9.0,
        392.0,
        762.0,
        &format!("Pagina {page}"),
        0.35,
    );
    canvas.line(LEFT, 742.0, LEFT + WIDTH, 742.0, 0.72);
}

fn draw_footer(canvas: &mut PdfCanvas, page: usize) {
    canvas.line(LEFT, 52.0, LEFT + WIDTH, 52.0, 0.82);
    canvas.text(
        FontKind::Regular,
        8.0,
        LEFT,
        34.0,
        &format!("Checklist interno de separacao/conferencia - pagina {page}"),
        0.40,
    );
}

fn draw_section_header(
    canvas: &mut PdfCanvas,
    section: &ChecklistSection,
    y: f64,
    continued: bool,
) -> f64 {
    canvas.rect(
        LEFT,
        y - SECTION_HEADER_HEIGHT + 8.0,
        WIDTH,
        SECTION_HEADER_HEIGHT,
        0.94,
    );
    canvas.text(
        FontKind::Bold,
        12.0,
        LEFT + 10.0,
        y - 10.0,
        &if continued {
            format!(
                "{} - {} (continuacao)",
                section.tecido.sku,
                truncate_text(&section.tecido.nome, 55)
            )
        } else {
            format!(
                "{} - {}",
                section.tecido.sku,
                truncate_text(&section.tecido.nome, 70)
            )
        },
        0.12,
    );
    let table_top = y - SECTION_HEADER_HEIGHT;
    draw_table_header(canvas, table_top);
    table_top - TABLE_HEADER_HEIGHT
}

fn draw_table_header(canvas: &mut PdfCanvas, y: f64) {
    canvas.rect(
        LEFT,
        y - TABLE_HEADER_HEIGHT,
        WIDTH,
        TABLE_HEADER_HEIGHT,
        0.88,
    );
    canvas.text(
        FontKind::Bold,
        8.4,
        LEFT + 10.0,
        y - 14.0,
        "Thumbnail",
        0.12,
    );
    canvas.text(FontKind::Bold, 8.4, LEFT + 72.0, y - 14.0, "Tecido", 0.12);
    canvas.text(
        FontKind::Bold,
        8.4,
        LEFT + 252.0,
        y - 14.0,
        "Nome da Cor",
        0.12,
    );
    canvas.text(FontKind::Bold, 8.4, LEFT + 448.0, y - 14.0, "Check", 0.12);
}

fn draw_row(canvas: &mut PdfCanvas, vinculo: &VinculoRecord, y: f64, shaded: bool) {
    if shaded {
        canvas.rect(LEFT, y - ROW_HEIGHT, WIDTH, ROW_HEIGHT, 0.985);
    }
    let thumb_y = y - 45.5;
    draw_swatch(canvas, LEFT + 10.0, thumb_y, vinculo.cor_hex.as_deref());
    canvas.text(
        FontKind::Regular,
        8.6,
        LEFT + 72.0,
        y - 28.0,
        &truncate_text(&vinculo.tecido_nome, 34),
        0.16,
    );
    canvas.text(
        FontKind::Regular,
        8.6,
        LEFT + 252.0,
        y - 28.0,
        &truncate_text(&vinculo.cor_nome, 34),
        0.16,
    );
    canvas.rect_stroke(LEFT + 458.0, y - 34.0, 15.0, 15.0, 1.0, 0.20);
    canvas.line(LEFT, y - ROW_HEIGHT, LEFT + WIDTH, y - ROW_HEIGHT, 0.86);
}

fn draw_empty_row(canvas: &mut PdfCanvas, y: f64) {
    canvas.rect(LEFT, y - ROW_HEIGHT, WIDTH, ROW_HEIGHT, 0.985);
    canvas.text(
        FontKind::Regular,
        9.0,
        LEFT + 10.0,
        y - 28.0,
        "Nenhum vinculo cadastrado para este tecido.",
        0.30,
    );
    canvas.line(LEFT, y - ROW_HEIGHT, LEFT + WIDTH, y - ROW_HEIGHT, 0.86);
}

fn draw_swatch(canvas: &mut PdfCanvas, x: f64, y: f64, hex: Option<&str>) {
    if let Some(rgb) = hex.and_then(parse_hex_color) {
        canvas.rect_rgb(x, y, THUMB, THUMB, rgb);
    } else {
        canvas.rect(x, y, THUMB, THUMB, 0.94);
        canvas.text(FontKind::Regular, 6.8, x + 7.0, y + 19.0, "sem cor", 0.36);
    }
    canvas.rect_outline(x, y, THUMB, THUMB, 0.55);
}

#[derive(Clone)]
struct PdfFonts {
    regular: PdfFontHandle,
    bold: PdfFontHandle,
}

#[derive(Clone, Copy)]
enum FontKind {
    Regular,
    Bold,
}

struct PdfCanvas {
    fonts: PdfFonts,
    ops: Vec<Op>,
}

impl PdfCanvas {
    fn new(fonts: PdfFonts) -> Self {
        Self {
            fonts,
            ops: Vec::new(),
        }
    }

    fn into_ops(self) -> Vec<Op> {
        self.ops
    }

    fn rect(&mut self, x: f64, y: f64, width: f64, height: f64, gray: f64) {
        self.ops.push(Op::SetFillColor {
            col: gray_color(gray),
        });
        self.ops.push(Op::DrawPolygon {
            polygon: rect_polygon(x, y, width, height, PaintMode::Fill),
        });
    }

    fn rect_rgb(&mut self, x: f64, y: f64, width: f64, height: f64, rgb: (u8, u8, u8)) {
        let (r, g, b) = rgb;
        self.ops.push(Op::SetFillColor {
            col: Color::Rgb(Rgb {
                r: f32::from(r) / 255.0,
                g: f32::from(g) / 255.0,
                b: f32::from(b) / 255.0,
                icc_profile: None,
            }),
        });
        self.ops.push(Op::DrawPolygon {
            polygon: rect_polygon(x, y, width, height, PaintMode::Fill),
        });
    }

    fn rect_stroke(
        &mut self,
        x: f64,
        y: f64,
        width: f64,
        height: f64,
        fill_gray: f64,
        stroke_gray: f64,
    ) {
        self.rect(x, y, width, height, fill_gray);
        self.rect_outline(x, y, width, height, stroke_gray);
    }

    fn rect_outline(&mut self, x: f64, y: f64, width: f64, height: f64, gray: f64) {
        self.ops.push(Op::SetOutlineColor {
            col: gray_color(gray),
        });
        self.ops.push(Op::SetOutlineThickness { pt: Pt(0.75) });
        self.ops.push(Op::DrawLine {
            line: rect_line(x, y, width, height),
        });
    }

    fn line(&mut self, x1: f64, y1: f64, x2: f64, y2: f64, gray: f64) {
        self.ops.push(Op::SetOutlineColor {
            col: gray_color(gray),
        });
        self.ops.push(Op::SetOutlineThickness { pt: Pt(0.75) });
        self.ops.push(Op::DrawLine {
            line: Line {
                points: vec![line_point(x1, y1), line_point(x2, y2)],
                is_closed: false,
            },
        });
    }

    fn text(&mut self, font: FontKind, size: f64, x: f64, y: f64, value: &str, gray: f64) {
        let font_handle = match font {
            FontKind::Regular => self.fonts.regular.clone(),
            FontKind::Bold => self.fonts.bold.clone(),
        };
        self.ops.push(Op::SetFillColor {
            col: gray_color(gray),
        });
        self.ops.push(Op::SetFont {
            font: font_handle,
            size: Pt(size as f32),
        });
        self.ops.push(Op::StartTextSection);
        self.ops.push(Op::SetTextCursor {
            pos: Point {
                x: Pt(x as f32),
                y: Pt(y as f32),
            },
        });
        self.ops.push(Op::ShowText {
            items: vec![TextItem::Text(value.to_string())],
        });
        self.ops.push(Op::EndTextSection);
    }
}

fn rect_polygon(x: f64, y: f64, width: f64, height: f64, mode: PaintMode) -> Polygon {
    Polygon {
        rings: vec![PolygonRing {
            points: vec![
                line_point(x, y),
                line_point(x + width, y),
                line_point(x + width, y + height),
                line_point(x, y + height),
            ],
        }],
        mode,
        winding_order: WindingOrder::NonZero,
    }
}

fn rect_line(x: f64, y: f64, width: f64, height: f64) -> Line {
    Line {
        points: vec![
            line_point(x, y),
            line_point(x + width, y),
            line_point(x + width, y + height),
            line_point(x, y + height),
        ],
        is_closed: true,
    }
}

fn line_point(x: f64, y: f64) -> LinePoint {
    LinePoint {
        p: Point {
            x: Pt(x as f32),
            y: Pt(y as f32),
        },
        bezier: false,
    }
}

fn gray_color(gray: f64) -> Color {
    let value = gray as f32;
    Color::Rgb(Rgb {
        r: value,
        g: value,
        b: value,
        icc_profile: None,
    })
}

fn truncate_text(value: &str, max_chars: usize) -> String {
    if value.chars().count() <= max_chars {
        return value.to_string();
    }
    value
        .chars()
        .take(max_chars.saturating_sub(3))
        .collect::<String>()
        + "..."
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn writes_checklist_pdf_with_page_breaks() {
        let tecido = TecidoRecord {
            id: 1,
            nome: String::from("Anarruga"),
            sku: String::from("ANAR"),
            composicao: String::from("Poliester"),
            largura_m: 1.5,
            custo_base: None,
            tipo: String::from("Liso"),
            transparencia: String::from("Baixa"),
            elasticidade: String::from("Baixa"),
            acabamento: String::from("Padrao"),
            rendimento_m_kg: None,
            gramatura_linear_g_m: Some(120),
            gramatura_g_m2: None,
        };
        let vinculos = (0..30)
            .map(|index| VinculoRecord {
                cor_id: index,
                tecido_nome: tecido.nome.clone(),
                cor_nome: format!("Cor {index}"),
                cor_hex: Some(String::from("#AA2233")),
                sku: Some(format!("ANAR-{index}")),
                tecido_custo_base: None,
                custo_override: None,
                custo_efetivo: None,
                has_imagem_original: false,
                has_imagem_brand: false,
                has_imagem_modelo: false,
                has_imagem_alternativa: false,
            })
            .collect::<Vec<_>>();
        let path = std::env::temp_dir().join("razai_checklist_test.pdf");
        write_checklist_pdf(&path, &[ChecklistSection { tecido, vinculos }]).unwrap();
        assert!(path.is_file());
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn writes_large_checklist_without_panic() {
        let tecido = TecidoRecord {
            id: 1,
            nome: String::from("Helanca"),
            sku: String::from("HELA"),
            composicao: String::from("Poliester"),
            largura_m: 1.5,
            custo_base: None,
            tipo: String::from("Liso"),
            transparencia: String::from("Baixa"),
            elasticidade: String::from("Baixa"),
            acabamento: String::from("Padrao"),
            rendimento_m_kg: None,
            gramatura_linear_g_m: Some(120),
            gramatura_g_m2: None,
        };
        let vinculos = (0..250)
            .map(|index| VinculoRecord {
                cor_id: index,
                tecido_nome: tecido.nome.clone(),
                cor_nome: format!("Cor {index}"),
                cor_hex: Some(String::from("#22AA99")),
                sku: Some(format!("HELA-{index}")),
                tecido_custo_base: None,
                custo_override: None,
                custo_efetivo: None,
                has_imagem_original: false,
                has_imagem_brand: false,
                has_imagem_modelo: false,
                has_imagem_alternativa: false,
            })
            .collect::<Vec<_>>();
        let path = std::env::temp_dir().join("razai_checklist_large_test.pdf");
        write_checklist_pdf(&path, &[ChecklistSection { tecido, vinculos }]).unwrap();
        assert!(path.is_file());
        let metadata = std::fs::metadata(&path).unwrap();
        assert!(metadata.len() > 0);
        let _ = std::fs::remove_file(path);
    }
}
