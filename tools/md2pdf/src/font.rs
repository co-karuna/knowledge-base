use anyhow::{Context, Result};
use printpdf::{ParsedFont, PdfFontParseWarning};
use std::path::Path;

/// ヒラギノ角ゴシック W3（Regular/本文用）のパス
const HIRAGINO_W3: &str = "/System/Library/Fonts/ヒラギノ角ゴシック W3.ttc";
/// ヒラギノ角ゴシック W6（Bold/見出し用）のパス
const HIRAGINO_W6: &str = "/System/Library/Fonts/ヒラギノ角ゴシック W6.ttc";

/// CJKフォントの全角文字デフォルト幅（font units, 1000 units/em想定）
const DEFAULT_CJK_WIDTH: u16 = 1000;
/// 半角文字デフォルト幅
const DEFAULT_HALF_WIDTH: u16 = 500;

pub struct FontSet {
    pub regular: ParsedFont,
    pub bold: ParsedFont,
}

impl FontSet {
    pub fn load() -> Result<Self> {
        let regular_bytes = load_font_bytes(HIRAGINO_W3)?;
        let bold_bytes = load_font_bytes(HIRAGINO_W6)?;

        let regular = parse_ttc_font(&regular_bytes, "Regular (W3)")?;
        let bold = parse_ttc_font(&bold_bytes, "Bold (W6)")?;

        Ok(FontSet { regular, bold })
    }

    /// 文字のグリフ幅を取得（font units）
    fn char_width(&self, c: char, bold: bool) -> u16 {
        let font = if bold { &self.bold } else { &self.regular };
        match font.lookup_glyph_index(c as u32) {
            Some(gid) => font.get_horizontal_advance(gid),
            None => default_char_width(c),
        }
    }

    /// テキスト幅をpt単位で計算
    pub fn text_width_pt(&self, text: &str, font_size_pt: f32, bold: bool) -> f32 {
        let units_per_em = 1000.0_f32;
        let total_units: f32 = text.chars().map(|c| self.char_width(c, bold) as f32).sum();
        total_units * font_size_pt / units_per_em
    }
}

fn default_char_width(c: char) -> u16 {
    if c.is_ascii() {
        DEFAULT_HALF_WIDTH
    } else {
        DEFAULT_CJK_WIDTH
    }
}

fn load_font_bytes(path: &str) -> Result<Vec<u8>> {
    let p = Path::new(path);
    if !p.exists() {
        anyhow::bail!("フォントファイルが見つかりません: {path}");
    }
    std::fs::read(p).with_context(|| format!("フォントファイルの読み込みに失敗: {path}"))
}

fn parse_ttc_font(bytes: &[u8], label: &str) -> Result<ParsedFont> {
    let mut warnings: Vec<PdfFontParseWarning> = Vec::new();
    ParsedFont::from_bytes(bytes, 0, &mut warnings)
        .with_context(|| format!("フォントの解析に失敗 ({label})"))
}
