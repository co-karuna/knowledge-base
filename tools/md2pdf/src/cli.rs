use clap::Parser;
use std::path::PathBuf;

/// Markdown → 署名付きPDF変換ツール
#[derive(Parser, Debug)]
#[command(name = "md2pdf", version, about)]
pub struct Args {
    /// 入力Markdownファイルパス
    #[arg(short, long)]
    pub input: PathBuf,

    /// 出力PDFファイルパス（省略時は入力ファイルの拡張子を.pdfに変更）
    #[arg(short, long)]
    pub output: Option<PathBuf>,

    /// PKCS#12証明書ファイルパス（電子署名用）
    #[arg(short, long)]
    pub cert: Option<PathBuf>,

    /// 証明書パスワード（未指定時はプロンプトで入力）
    #[arg(short, long)]
    pub password: Option<String>,

    /// 本文フォントサイズ（pt）
    #[arg(long, default_value = "10.5")]
    pub font_size: f32,
}

impl Args {
    pub fn output_path(&self) -> PathBuf {
        self.output
            .clone()
            .unwrap_or_else(|| self.input.with_extension("pdf"))
    }
}
