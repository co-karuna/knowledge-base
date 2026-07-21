mod cli;
mod error;
mod font;
mod layout;
mod markdown;
mod pdf_render;
mod signer;

use anyhow::{Context, Result};
use clap::Parser;

fn main() -> Result<()> {
    let args = cli::Args::parse();

    // 入力ファイル読み込み
    let input_path = &args.input;
    let md_content = std::fs::read_to_string(input_path)
        .with_context(|| format!("入力ファイルの読み込みに失敗: {}", input_path.display()))?;

    eprintln!("📄 {} を読み込みました", input_path.display());

    // Markdown → 中間表現
    eprintln!("📝 Markdownを変換中...");
    let (metadata, blocks) = markdown::parse_markdown(&md_content)?;

    if let Some(title) = &metadata.title {
        eprintln!("   タイトル: {title}");
    }

    // フォント読み込み
    let fonts = font::FontSet::load()?;

    // レイアウト → PDF描画命令
    let layout_ops = layout::LayoutEngine::new(&fonts, args.font_size).layout(&blocks);

    // PDF生成
    eprintln!("📄 PDFを生成中...");
    let pdf_bytes = pdf_render::render_pdf(&layout_ops, &fonts)?;

    // 署名（証明書が指定されている場合）
    let output_bytes = if let Some(cert_path) = &args.cert {
        eprintln!("🔏 電子署名を付与中...");

        let password = match &args.password {
            Some(p) => p.clone(),
            None => rpassword::prompt_password("証明書のパスワード: ")
                .context("パスワードの読み取りに失敗")?,
        };

        signer::sign_pdf(&pdf_bytes, cert_path, &password)?
    } else {
        pdf_bytes
    };

    // 出力
    let output_path = args.output_path();
    std::fs::write(&output_path, &output_bytes)
        .with_context(|| format!("出力ファイルの書き込みに失敗: {}", output_path.display()))?;

    eprintln!(
        "✅ {} ({} bytes)",
        output_path.display(),
        output_bytes.len()
    );

    Ok(())
}
