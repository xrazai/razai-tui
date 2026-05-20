use std::{fs::OpenOptions, io::Write, os::windows::fs::OpenOptionsExt, path::Path};

use chrono::Local;
use printpdf::{
    Color, Line, LinePoint, Op, PaintMode, ParsedFont, PdfDocument, PdfPage, PdfSaveOptions, Point,
    Polygon, PolygonRing, Pt, Rgb, TextItem, WindingOrder, ops::PdfFontHandle, units::Mm,
};

use crate::models::VendaItem;

const IBM_PLEX_SANS_REGULAR: &[u8] =
    include_bytes!("../../../assets/fonts/IBMPlexSans-Regular.ttf");
const IBM_PLEX_SANS_BOLD: &[u8] = include_bytes!("../../../assets/fonts/IBMPlexSans-Bold.ttf");

pub(super) fn write_pedido_pdf(
    path: &Path,
    pedido_id: i64,
    itens: &[VendaItem],
) -> Result<(), String> {
    const FILE_SHARE_READ: u32 = 0x00000001;
    const FILE_SHARE_WRITE: u32 = 0x00000002;
    const FILE_SHARE_DELETE: u32 = 0x00000004;

    let bytes = pedido_pdf_bytes(pedido_id, itens)?;
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

fn pedido_pdf_bytes(pedido_id: i64, itens: &[VendaItem]) -> Result<Vec<u8>, String> {
    let mut font_warnings = Vec::new();
    let regular = ParsedFont::from_bytes(IBM_PLEX_SANS_REGULAR, 0, &mut font_warnings)
        .ok_or_else(|| String::from("falha ao carregar IBM Plex Sans Regular"))?;
    let bold = ParsedFont::from_bytes(IBM_PLEX_SANS_BOLD, 0, &mut font_warnings)
        .ok_or_else(|| String::from("falha ao carregar IBM Plex Sans Bold"))?;

    let mut pdf = PdfDocument::new(&format!("Pedido #{pedido_id} Razai"));
    let regular_id = pdf.add_font(&regular);
    let bold_id = pdf.add_font(&bold);
    let fonts = PdfFonts {
        regular: PdfFontHandle::External(regular_id),
        bold: PdfFontHandle::External(bold_id),
    };

    let total: f64 = itens
        .iter()
        .map(|item| item.quantidade * item.preco_unitario)
        .sum();
    let pages = paginate_items(itens);
    let total_pages = pages.len().max(1);
    let mut pdf_pages = Vec::with_capacity(total_pages);

    for (page_index, page_items) in pages.iter().enumerate() {
        let mut canvas = PdfCanvas::new(fonts.clone());
        let page_number = page_index + 1;
        let is_last_page = page_number == total_pages;

        draw_header(&mut canvas, pedido_id, page_number, total_pages);
        draw_intro(&mut canvas, page_number, is_last_page);

        let page_start_index = page_index * 40;
        if page_items.len() <= 20 {
            draw_table(
                &mut canvas,
                page_items,
                page_start_index,
                TableLayout {
                    x: 42.0,
                    y: 596.0,
                    width: 511.0,
                    row_height: 24.0,
                    item_chars: 42,
                    compact: false,
                },
            );
        } else {
            let (left, right) = page_items.split_at(20);
            draw_table(
                &mut canvas,
                left,
                page_start_index,
                TableLayout {
                    x: 42.0,
                    y: 596.0,
                    width: 246.0,
                    row_height: 24.0,
                    item_chars: 26,
                    compact: true,
                },
            );
            draw_table(
                &mut canvas,
                right,
                page_start_index + 20,
                TableLayout {
                    x: 307.0,
                    y: 596.0,
                    width: 246.0,
                    row_height: 24.0,
                    item_chars: 26,
                    compact: true,
                },
            );
        }

        if is_last_page {
            draw_summary(&mut canvas, itens, total);
        }
        draw_footer(&mut canvas);
        pdf_pages.push(PdfPage::new(Mm(210.0), Mm(297.0), canvas.into_ops()));
    }

    let mut save_warnings = Vec::new();
    Ok(pdf
        .with_pages(pdf_pages)
        .save(&PdfSaveOptions::default(), &mut save_warnings))
}

fn paginate_items(itens: &[VendaItem]) -> Vec<&[VendaItem]> {
    if itens.is_empty() {
        return vec![itens];
    }
    let page_size = if itens.len() <= 20 { 20 } else { 40 };
    itens.chunks(page_size).collect()
}

fn draw_header(canvas: &mut PdfCanvas, pedido_id: i64, page: usize, total_pages: usize) {
    canvas.line(42.0, 742.0, 553.0, 742.0, 0.72);
    canvas.text(FontKind::Bold, 24.0, 42.0, 780.0, "RAZAI", 0.10);
    canvas.text(
        FontKind::Bold,
        11.5,
        42.0,
        758.0,
        &format!("Pedido #{pedido_id}"),
        0.18,
    );
    canvas.text(
        FontKind::Bold,
        9.0,
        402.0,
        780.0,
        "Documento comercial",
        0.18,
    );
    canvas.text(
        FontKind::Regular,
        9.0,
        402.0,
        764.0,
        &format!("Gerado em {}", Local::now().format("%d/%m/%Y %H:%M")),
        0.35,
    );
    canvas.text(
        FontKind::Regular,
        9.0,
        402.0,
        748.0,
        &format!("Página {page}/{total_pages}"),
        0.35,
    );
}

fn draw_intro(canvas: &mut PdfCanvas, page: usize, is_last_page: bool) {
    let title = if page == 1 {
        "Pedido"
    } else if is_last_page {
        "Itens e fechamento"
    } else {
        "Itens do pedido"
    };
    canvas.text(FontKind::Bold, 18.0, 42.0, 696.0, title, 0.10);
    canvas.text(
        FontKind::Regular,
        9.5,
        42.0,
        678.0,
        "Pedido pendente de pagamento. Após confirmação, será convertido em venda.",
        0.28,
    );
    canvas.text(FontKind::Bold, 8.8, 42.0, 662.0, "PIX: 11 93392 0695", 0.18);
    canvas.text(
        FontKind::Bold,
        8.8,
        42.0,
        646.0,
        "Razão Social: RAZAI LTDA",
        0.18,
    );
    canvas.text(FontKind::Bold, 8.8, 42.0, 630.0, "Banco: ITAÚ", 0.18);
}

fn draw_summary(canvas: &mut PdfCanvas, itens: &[VendaItem], total: f64) {
    let quantidade_total: f64 = itens.iter().map(|item| item.quantidade).sum();
    let values = [
        (
            "Data",
            Local::now().format("%d/%m/%Y").to_string(),
            FontKind::Bold,
            12.0,
        ),
        ("Itens", itens.len().to_string(), FontKind::Bold, 12.0),
        (
            "QTD Total",
            format_quantity(quantidade_total),
            FontKind::Bold,
            12.0,
        ),
        ("Valor Total", format_money(total), FontKind::Bold, 15.5),
    ];
    let block_width = 127.75;
    let y = 118.0;
    let width = block_width * values.len() as f64;

    canvas.rect_stroke(42.0, y, width, 56.0, 0.972, 0.72);
    for (index, (label, value, font, size)) in values.iter().enumerate() {
        let x = 42.0 + index as f64 * block_width;
        if index > 0 {
            canvas.line(x, y, x, y + 56.0, 0.84);
        }
        canvas.text(FontKind::Regular, 8.2, x + 10.0, y + 35.0, label, 0.36);
        canvas.text(*font, *size, x + 10.0, y + 14.0, value, 0.10);
    }
}

fn draw_footer(canvas: &mut PdfCanvas) {
    canvas.line(42.0, 86.0, 553.0, 86.0, 0.80);
    canvas.text(
        FontKind::Regular,
        8.5,
        42.0,
        65.0,
        "Este pedido não confirma reserva de estoque até aprovação do pagamento.",
        0.35,
    );
}

#[derive(Clone, Copy)]
struct TableLayout {
    x: f64,
    y: f64,
    width: f64,
    row_height: f64,
    item_chars: usize,
    compact: bool,
}

fn draw_table(
    canvas: &mut PdfCanvas,
    itens: &[VendaItem],
    start_index: usize,
    layout: TableLayout,
) {
    let header_height = 24.0;
    canvas.rect(layout.x, layout.y, layout.width, header_height, 0.94);

    let item_x = layout.x + 10.0;
    let qty_x = if layout.compact {
        layout.x + 142.0
    } else {
        layout.x + 284.0
    };
    let price_x = if layout.compact {
        layout.x + 170.0
    } else {
        layout.x + 352.0
    };
    let total_x = if layout.compact {
        layout.x + 209.0
    } else {
        layout.x + 428.0
    };
    let header_y = layout.y + 9.0;

    canvas.text(FontKind::Bold, 8.3, item_x, header_y, "Item", 0.15);
    canvas.text(FontKind::Bold, 8.3, qty_x, header_y, "QTD", 0.15);
    canvas.text(FontKind::Bold, 8.3, price_x, header_y, "Preço", 0.15);
    canvas.text(FontKind::Bold, 8.3, total_x, header_y, "Total", 0.15);

    let mut row_y = layout.y - 25.0;
    for (index, item) in itens.iter().enumerate() {
        if index % 2 == 0 {
            canvas.rect(
                layout.x,
                row_y - 7.0,
                layout.width,
                layout.row_height,
                0.985,
            );
        }
        let item_number = start_index + index + 1;
        let subtotal = item.quantidade * item.preco_unitario;
        let font_size = if layout.compact { 7.2 } else { 9.2 };
        let description = format!("{item_number}. {}", item.descricao);

        canvas.text(
            FontKind::Regular,
            font_size,
            item_x,
            row_y,
            &truncate_text(&description, layout.item_chars),
            0.12,
        );
        canvas.text(
            FontKind::Regular,
            font_size,
            qty_x,
            row_y,
            &format_quantity(item.quantidade),
            0.12,
        );
        canvas.text(
            FontKind::Regular,
            font_size,
            price_x,
            row_y,
            &format_money(item.preco_unitario),
            0.12,
        );
        canvas.text(
            FontKind::Bold,
            font_size,
            total_x,
            row_y,
            &format_money(subtotal),
            0.12,
        );
        canvas.line(
            layout.x,
            row_y - 10.0,
            layout.x + layout.width,
            row_y - 10.0,
            0.86,
        );
        row_y -= layout.row_height;
    }
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
        self.ops.push(Op::SetOutlineColor {
            col: gray_color(stroke_gray),
        });
        self.ops.push(Op::SetOutlineThickness { pt: Pt(1.0) });
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

fn format_money(value: f64) -> String {
    format!("R$ {:.2}", value).replace('.', ",")
}

fn format_quantity(value: f64) -> String {
    if value.fract().abs() < f64::EPSILON {
        format!("{}", value as i64)
    } else {
        format!("{value:.2}").replace('.', ",")
    }
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
    fn generate_pdf_preview_when_requested() {
        if std::env::var("RAZAI_GENERATE_PDF_PREVIEW").is_err() {
            return;
        }

        let count = std::env::var("RAZAI_PDF_PREVIEW_ITEMS")
            .ok()
            .and_then(|value| value.parse::<usize>().ok())
            .unwrap_or(48);
        let itens = (1..=count)
            .map(|index| VendaItem {
                descricao: format!("Tecido {index:02} - Cor Especial {index:02}"),
                quantidade: (index % 9 + 1) as f64,
                preco_unitario: 10.0 + index as f64 * 0.73,
                estoque_tecido_id: None,
                estoque_item_id: None,
                estoque_usa_estampas: false,
            })
            .collect::<Vec<_>>();
        let path = std::env::temp_dir().join(format!("razai_pedido_ibm_plex_preview_{count}.pdf"));
        write_pedido_pdf(&path, 2048, &itens).unwrap();
        eprintln!("{}", path.display());
    }
}
