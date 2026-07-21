use anyhow::Result;
use printpdf::*;

use crate::font::FontSet;
use crate::layout::{self, LayoutOp};

pub fn render_pdf(ops: &[LayoutOp], fonts: &FontSet) -> Result<Vec<u8>> {
    let mut doc = PdfDocument::new("md2pdf");

    let regular_font_id = doc.add_font(&fonts.regular);
    let bold_font_id = doc.add_font(&fonts.bold);

    let mut pages: Vec<PdfPage> = Vec::new();
    let mut current_ops: Vec<Op> = Vec::new();

    let page_w = Mm(layout::page_width_mm());
    let page_h = Mm(layout::page_height_mm());

    for op in ops {
        match op {
            LayoutOp::PageBreak => {
                pages.push(PdfPage::new(page_w, page_h, std::mem::take(&mut current_ops)));
            }
            LayoutOp::Text {
                x_mm,
                y_mm,
                text,
                font_size_pt,
                bold,
            } => {
                let pdf_y = layout::page_height_mm() - y_mm - pt_to_mm(*font_size_pt);

                let font_handle = if *bold {
                    PdfFontHandle::External(bold_font_id.clone())
                } else {
                    PdfFontHandle::External(regular_font_id.clone())
                };

                current_ops.push(Op::StartTextSection);
                current_ops.push(Op::SetTextCursor {
                    pos: Point::new(Mm(*x_mm), Mm(pdf_y)),
                });
                current_ops.push(Op::SetFont {
                    font: font_handle,
                    size: Pt(*font_size_pt),
                });
                current_ops.push(Op::ShowText {
                    items: vec![TextItem::Text(text.clone())],
                });
                current_ops.push(Op::EndTextSection);
            }
            LayoutOp::Line {
                x1_mm,
                y1_mm,
                x2_mm,
                y2_mm,
                thickness_pt,
            } => {
                let pdf_y1 = layout::page_height_mm() - y1_mm;
                let pdf_y2 = layout::page_height_mm() - y2_mm;

                current_ops.push(Op::SetOutlineThickness {
                    pt: Pt(*thickness_pt),
                });
                current_ops.push(Op::SetOutlineColor {
                    col: Color::Rgb(Rgb::new(0.6, 0.6, 0.6, None)),
                });
                current_ops.push(Op::DrawLine {
                    line: Line {
                        points: vec![
                            LinePoint {
                                p: Point::new(Mm(*x1_mm), Mm(pdf_y1)),
                                bezier: false,
                            },
                            LinePoint {
                                p: Point::new(Mm(*x2_mm), Mm(pdf_y2)),
                                bezier: false,
                            },
                        ],
                        is_closed: false,
                    },
                });
            }
            LayoutOp::FilledRect {
                x_mm,
                y_mm,
                w_mm,
                h_mm,
                r,
                g,
                b,
            } => {
                let pdf_y = layout::page_height_mm() - y_mm - h_mm;

                current_ops.push(Op::SetFillColor {
                    col: Color::Rgb(Rgb::new(*r, *g, *b, None)),
                });
                current_ops.push(Op::DrawPolygon {
                    polygon: Polygon {
                        rings: vec![PolygonRing {
                            points: vec![
                                LinePoint {
                                    p: Point::new(Mm(*x_mm), Mm(pdf_y)),
                                    bezier: false,
                                },
                                LinePoint {
                                    p: Point::new(Mm(x_mm + w_mm), Mm(pdf_y)),
                                    bezier: false,
                                },
                                LinePoint {
                                    p: Point::new(Mm(x_mm + w_mm), Mm(pdf_y + h_mm)),
                                    bezier: false,
                                },
                                LinePoint {
                                    p: Point::new(Mm(*x_mm), Mm(pdf_y + h_mm)),
                                    bezier: false,
                                },
                            ],
                        }],
                        mode: PaintMode::Fill,
                        winding_order: WindingOrder::NonZero,
                    },
                });
                // テキスト色をリセット
                current_ops.push(Op::SetFillColor {
                    col: Color::Rgb(Rgb::new(0.0, 0.0, 0.0, None)),
                });
            }
        }
    }

    // 最後のページを追加
    if !current_ops.is_empty() {
        pages.push(PdfPage::new(page_w, page_h, current_ops));
    }

    if pages.is_empty() {
        pages.push(PdfPage::new(page_w, page_h, vec![]));
    }

    let mut warnings = Vec::new();
    // CJKフォントのサブセッティングでグリフが欠落する問題を回避
    let save_options = PdfSaveOptions {
        subset_fonts: false,
        ..Default::default()
    };
    let pdf_bytes = doc.with_pages(pages).save(&save_options, &mut warnings);

    Ok(pdf_bytes)
}

fn pt_to_mm(pt: f32) -> f32 {
    pt * 25.4 / 72.0
}
