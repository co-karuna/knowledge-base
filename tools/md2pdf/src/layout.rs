use crate::font::FontSet;
use crate::markdown::{MdBlock, StyledText};

/// A4サイズ (mm)
const PAGE_WIDTH_MM: f32 = 210.0;
const PAGE_HEIGHT_MM: f32 = 297.0;

/// マージン (mm)
const MARGIN_TOP: f32 = 25.0;
const MARGIN_BOTTOM: f32 = 25.0;
const MARGIN_LEFT: f32 = 20.0;
const MARGIN_RIGHT: f32 = 20.0;

/// 利用可能な幅・高さ (mm)
const CONTENT_WIDTH: f32 = PAGE_WIDTH_MM - MARGIN_LEFT - MARGIN_RIGHT;
/// 行頭禁止文字（kinsoku shori）
const KINSOKU_NOT_AT_START: &str = "、。，．）】」』》〉}]!?！？ー～…・:;；：ぁぃぅぇぉっゃゅょゎァィゥェォッャュョヮヵヶ";
/// 行末禁止文字
const KINSOKU_NOT_AT_END: &str = "（【「『《〈{[";

/// 見出しフォントサイズ (pt)
fn heading_font_size(level: u8) -> f32 {
    match level {
        1 => 20.0,
        2 => 16.0,
        3 => 13.0,
        _ => 11.0,
    }
}

/// 見出し前後のスペース (mm)
fn heading_spacing_before(level: u8) -> f32 {
    match level {
        1 => 8.0,
        2 => 6.0,
        3 => 4.0,
        _ => 3.0,
    }
}

fn heading_spacing_after(level: u8) -> f32 {
    match level {
        1 => 4.0,
        2 => 3.0,
        _ => 2.0,
    }
}

/// mm → pt 変換
fn mm_to_pt(mm: f32) -> f32 {
    mm * 72.0 / 25.4
}

/// pt → mm 変換
fn pt_to_mm(pt: f32) -> f32 {
    pt * 25.4 / 72.0
}

/// レイアウト済みの描画命令
#[derive(Debug, Clone)]
pub enum LayoutOp {
    /// テキスト描画
    Text {
        x_mm: f32,
        y_mm: f32,
        text: String,
        font_size_pt: f32,
        bold: bool,
    },
    /// 線描画
    Line {
        x1_mm: f32,
        y1_mm: f32,
        x2_mm: f32,
        y2_mm: f32,
        thickness_pt: f32,
    },
    /// 矩形描画（塗りつぶし）
    FilledRect {
        x_mm: f32,
        y_mm: f32,
        w_mm: f32,
        h_mm: f32,
        r: f32,
        g: f32,
        b: f32,
    },
    /// 改ページ
    PageBreak,
}

pub struct LayoutEngine<'a> {
    fonts: &'a FontSet,
    base_font_size: f32,
    cursor_y: f32, // 現在のY位置 (mm, ページ上端から)
    ops: Vec<LayoutOp>,
}

impl<'a> LayoutEngine<'a> {
    pub fn new(fonts: &'a FontSet, base_font_size: f32) -> Self {
        LayoutEngine {
            fonts,
            base_font_size,
            cursor_y: MARGIN_TOP,
            ops: Vec::new(),
        }
    }

    pub fn layout(mut self, blocks: &[MdBlock]) -> Vec<LayoutOp> {
        for block in blocks {
            self.layout_block(block);
        }
        self.ops
    }

    fn remaining_height(&self) -> f32 {
        PAGE_HEIGHT_MM - MARGIN_BOTTOM - self.cursor_y
    }

    fn ensure_space(&mut self, needed_mm: f32) {
        if self.remaining_height() < needed_mm {
            self.new_page();
        }
    }

    fn new_page(&mut self) {
        self.ops.push(LayoutOp::PageBreak);
        self.cursor_y = MARGIN_TOP;
    }

    fn layout_block(&mut self, block: &MdBlock) {
        match block {
            MdBlock::Heading { level, text } => {
                self.layout_heading(*level, text);
            }
            MdBlock::Paragraph { text } => {
                self.layout_paragraph(text, self.base_font_size, false, 0.0);
            }
            MdBlock::List {
                items,
                ordered,
                start,
            } => {
                self.layout_list(items, *ordered, *start);
            }
            MdBlock::Table { headers, rows } => {
                self.layout_table(headers, rows);
            }
            MdBlock::CodeBlock { code, .. } => {
                self.layout_code_block(code);
            }
            MdBlock::HorizontalRule => {
                self.layout_hr();
            }
        }
    }

    fn layout_heading(&mut self, level: u8, text: &[StyledText]) {
        let font_size = heading_font_size(level);
        let line_height_mm = pt_to_mm(font_size * 1.4);

        self.cursor_y += heading_spacing_before(level);
        self.ensure_space(line_height_mm + heading_spacing_after(level));

        let plain = styled_text_to_plain(text);
        let lines = self.wrap_text(&plain, font_size, true, 0.0);

        for line in &lines {
            self.ensure_space(line_height_mm);
            self.ops.push(LayoutOp::Text {
                x_mm: MARGIN_LEFT,
                y_mm: self.cursor_y,
                text: line.clone(),
                font_size_pt: font_size,
                bold: true,
            });
            self.cursor_y += line_height_mm;
        }

        self.cursor_y += heading_spacing_after(level);
    }

    fn layout_paragraph(
        &mut self,
        text: &[StyledText],
        font_size: f32,
        bold: bool,
        indent_mm: f32,
    ) {
        let line_height_mm = pt_to_mm(font_size * 1.6);
        let plain = styled_text_to_plain(text);

        if plain.trim().is_empty() {
            return;
        }

        let lines = self.wrap_text(&plain, font_size, bold, indent_mm);

        for line in &lines {
            self.ensure_space(line_height_mm);
            self.ops.push(LayoutOp::Text {
                x_mm: MARGIN_LEFT + indent_mm,
                y_mm: self.cursor_y,
                text: line.clone(),
                font_size_pt: font_size,
                bold,
            });
            self.cursor_y += line_height_mm;
        }

        self.cursor_y += pt_to_mm(font_size * 0.4);
    }

    fn layout_list(
        &mut self,
        items: &[Vec<StyledText>],
        ordered: bool,
        start: Option<u64>,
    ) {
        let font_size = self.base_font_size;
        let line_height_mm = pt_to_mm(font_size * 1.6);
        let indent = 5.0_f32; // mm
        let bullet_width = 3.0_f32; // mm

        for (i, item) in items.iter().enumerate() {
            let prefix = if ordered {
                let num = start.unwrap_or(1) + i as u64;
                format!("{num}. ")
            } else {
                "• ".to_string()
            };

            let plain = styled_text_to_plain(item);
            let full_text = format!("{prefix}{plain}");
            let lines = self.wrap_text(&full_text, font_size, false, indent);

            for (j, line) in lines.iter().enumerate() {
                self.ensure_space(line_height_mm);
                let x = if j == 0 {
                    MARGIN_LEFT + indent
                } else {
                    MARGIN_LEFT + indent + bullet_width
                };
                self.ops.push(LayoutOp::Text {
                    x_mm: x,
                    y_mm: self.cursor_y,
                    text: line.clone(),
                    font_size_pt: font_size,
                    bold: false,
                });
                self.cursor_y += line_height_mm;
            }
        }

        self.cursor_y += pt_to_mm(font_size * 0.3);
    }

    fn layout_table(
        &mut self,
        headers: &[Vec<StyledText>],
        rows: &[Vec<Vec<StyledText>>],
    ) {
        let font_size = self.base_font_size - 1.0;
        let cell_padding_mm = 2.0;
        let row_height_mm = pt_to_mm(font_size * 1.6) + cell_padding_mm * 2.0;

        let num_cols = headers.len().max(rows.iter().map(|r| r.len()).max().unwrap_or(0));
        if num_cols == 0 {
            return;
        }

        // 列幅を計算
        let col_widths = self.calc_column_widths(headers, rows, num_cols, font_size);

        // テーブル全体の高さをチェック
        let total_height = row_height_mm * (1 + rows.len()) as f32;
        self.ensure_space(row_height_mm.min(total_height));

        let x_start = MARGIN_LEFT;

        // ヘッダ行（背景色付き）
        let mut x = x_start;
        for (col_idx, header) in headers.iter().enumerate() {
            let w = col_widths.get(col_idx).copied().unwrap_or(20.0);
            // ヘッダ背景
            self.ops.push(LayoutOp::FilledRect {
                x_mm: x,
                y_mm: self.cursor_y,
                w_mm: w,
                h_mm: row_height_mm,
                r: 0.92,
                g: 0.92,
                b: 0.92,
            });
            // ヘッダテキスト
            let text = styled_text_to_plain(header);
            self.ops.push(LayoutOp::Text {
                x_mm: x + cell_padding_mm,
                y_mm: self.cursor_y + cell_padding_mm,
                text,
                font_size_pt: font_size,
                bold: true,
            });
            x += w;
        }
        let table_width: f32 = col_widths.iter().sum();
        // ヘッダ上罫線
        self.ops.push(LayoutOp::Line {
            x1_mm: x_start,
            y1_mm: self.cursor_y,
            x2_mm: x_start + table_width,
            y2_mm: self.cursor_y,
            thickness_pt: 0.5,
        });
        self.cursor_y += row_height_mm;
        // ヘッダ下罫線
        self.ops.push(LayoutOp::Line {
            x1_mm: x_start,
            y1_mm: self.cursor_y,
            x2_mm: x_start + table_width,
            y2_mm: self.cursor_y,
            thickness_pt: 0.5,
        });

        // データ行
        for row in rows {
            self.ensure_space(row_height_mm);
            x = x_start;
            for (col_idx, cell) in row.iter().enumerate() {
                let w = col_widths.get(col_idx).copied().unwrap_or(20.0);
                let text = styled_text_to_plain(cell);
                self.ops.push(LayoutOp::Text {
                    x_mm: x + cell_padding_mm,
                    y_mm: self.cursor_y + cell_padding_mm,
                    text,
                    font_size_pt: font_size,
                    bold: false,
                });
                x += w;
            }
            self.cursor_y += row_height_mm;
            // 行下罫線
            self.ops.push(LayoutOp::Line {
                x1_mm: x_start,
                y1_mm: self.cursor_y,
                x2_mm: x_start + table_width,
                y2_mm: self.cursor_y,
                thickness_pt: 0.3,
            });
        }

        // 縦罫線
        x = x_start;
        let y_top = self.cursor_y - row_height_mm * (1 + rows.len()) as f32;
        let y_bottom = self.cursor_y;
        for w in &col_widths {
            self.ops.push(LayoutOp::Line {
                x1_mm: x,
                y1_mm: y_top,
                x2_mm: x,
                y2_mm: y_bottom,
                thickness_pt: 0.3,
            });
            x += w;
        }
        // 右端の縦罫線
        self.ops.push(LayoutOp::Line {
            x1_mm: x,
            y1_mm: y_top,
            x2_mm: x,
            y2_mm: y_bottom,
            thickness_pt: 0.3,
        });

        self.cursor_y += 2.0;
    }

    fn calc_column_widths(
        &self,
        headers: &[Vec<StyledText>],
        rows: &[Vec<Vec<StyledText>>],
        num_cols: usize,
        font_size: f32,
    ) -> Vec<f32> {
        let cell_padding_mm = 2.0;
        let mut max_widths = vec![0.0_f32; num_cols];

        // ヘッダの幅を計測
        for (i, header) in headers.iter().enumerate() {
            let text = styled_text_to_plain(header);
            let w = pt_to_mm(self.fonts.text_width_pt(&text, font_size, true)) + cell_padding_mm * 2.0;
            max_widths[i] = max_widths[i].max(w);
        }

        // データ行の幅を計測
        for row in rows {
            for (i, cell) in row.iter().enumerate() {
                if i >= num_cols {
                    break;
                }
                let text = styled_text_to_plain(cell);
                let w =
                    pt_to_mm(self.fonts.text_width_pt(&text, font_size, false)) + cell_padding_mm * 2.0;
                max_widths[i] = max_widths[i].max(w);
            }
        }

        // 最小幅を確保
        for w in &mut max_widths {
            *w = w.max(15.0);
        }

        // 合計がCONTENT_WIDTHを超える場合は按分
        let total: f32 = max_widths.iter().sum();
        if total > CONTENT_WIDTH {
            let scale = CONTENT_WIDTH / total;
            for w in &mut max_widths {
                *w *= scale;
            }
        }

        max_widths
    }

    fn layout_code_block(&mut self, code: &str) {
        let font_size = self.base_font_size - 1.5;
        let line_height_mm = pt_to_mm(font_size * 1.4);
        let padding_mm = 3.0;

        let lines: Vec<&str> = code.lines().collect();
        let block_height = line_height_mm * lines.len() as f32 + padding_mm * 2.0;

        self.ensure_space(line_height_mm + padding_mm * 2.0);

        // 背景
        let bg_y = self.cursor_y;
        self.ops.push(LayoutOp::FilledRect {
            x_mm: MARGIN_LEFT,
            y_mm: bg_y,
            w_mm: CONTENT_WIDTH,
            h_mm: block_height.min(self.remaining_height()),
            r: 0.95,
            g: 0.95,
            b: 0.95,
        });

        self.cursor_y += padding_mm;

        for line in &lines {
            self.ensure_space(line_height_mm);
            self.ops.push(LayoutOp::Text {
                x_mm: MARGIN_LEFT + padding_mm,
                y_mm: self.cursor_y,
                text: line.to_string(),
                font_size_pt: font_size,
                bold: false,
            });
            self.cursor_y += line_height_mm;
        }

        self.cursor_y += padding_mm + 2.0;
    }

    fn layout_hr(&mut self) {
        self.cursor_y += 3.0;
        self.ensure_space(1.0);
        self.ops.push(LayoutOp::Line {
            x1_mm: MARGIN_LEFT,
            y1_mm: self.cursor_y,
            x2_mm: PAGE_WIDTH_MM - MARGIN_RIGHT,
            y2_mm: self.cursor_y,
            thickness_pt: 0.5,
        });
        self.cursor_y += 3.0;
    }

    /// テキストを指定幅で折り返す（禁則処理付き）
    fn wrap_text(
        &self,
        text: &str,
        font_size: f32,
        bold: bool,
        indent_mm: f32,
    ) -> Vec<String> {
        let max_width_pt = mm_to_pt(CONTENT_WIDTH - indent_mm);
        let mut lines = Vec::new();
        let mut current_line = String::new();
        let mut current_width = 0.0_f32;

        let chars: Vec<char> = text.chars().collect();
        let mut i = 0;

        while i < chars.len() {
            let c = chars[i];

            // 改行文字
            if c == '\n' {
                lines.push(current_line.clone());
                current_line.clear();
                current_width = 0.0;
                i += 1;
                continue;
            }

            let char_str = c.to_string();
            let char_width = self.fonts.text_width_pt(&char_str, font_size, bold);

            if current_width + char_width > max_width_pt && !current_line.is_empty() {
                // 禁則処理: 次の文字が行頭禁止文字なら現在行に含める
                if KINSOKU_NOT_AT_START.contains(chars[i]) {
                    current_line.push(c);
                    i += 1;
                    lines.push(current_line.clone());
                    current_line.clear();
                    current_width = 0.0;
                    continue;
                }

                // 禁則処理: 現在行末が行末禁止文字なら次の行に送る
                if let Some(last) = current_line.chars().last() {
                    if KINSOKU_NOT_AT_END.contains(last) {
                        let removed = current_line.pop().unwrap();
                        let removed_width = self.fonts.text_width_pt(
                            &removed.to_string(),
                            font_size,
                            bold,
                        );
                        lines.push(current_line.clone());
                        current_line.clear();
                        current_line.push(removed);
                        current_line.push(c);
                        current_width = removed_width + char_width;
                        i += 1;
                        continue;
                    }
                }

                lines.push(current_line.clone());
                current_line.clear();
                current_width = 0.0;
            }

            current_line.push(c);
            current_width += char_width;
            i += 1;
        }

        if !current_line.is_empty() {
            lines.push(current_line);
        }

        if lines.is_empty() {
            lines.push(String::new());
        }

        lines
    }
}

fn styled_text_to_plain(text: &[StyledText]) -> String {
    text.iter().map(|s| s.text.as_str()).collect()
}

pub fn page_width_mm() -> f32 {
    PAGE_WIDTH_MM
}

pub fn page_height_mm() -> f32 {
    PAGE_HEIGHT_MM
}

