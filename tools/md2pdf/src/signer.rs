use anyhow::{Context, Result};
use lopdf::{Dictionary, Document, Object, ObjectId, StringFormat};
use openssl::pkcs12::Pkcs12;
use openssl::pkcs7::Pkcs7;
use openssl::pkcs7::Pkcs7Flags;
use openssl::stack::Stack;
use openssl::x509::X509;
use std::path::Path;

/// 署名プレースホルダーのサイズ（hex文字数 = バイト数 * 2）
const SIGNATURE_PLACEHOLDER_SIZE: usize = 8192;
/// ByteRange値のプレースホルダー幅（固定幅で数値を書き込む）
const BYTERANGE_PLACEHOLDER: &str = "0000000000";

pub fn sign_pdf(
    pdf_bytes: &[u8],
    cert_path: &Path,
    password: &str,
) -> Result<Vec<u8>> {
    // PKCS#12から秘密鍵と証明書を取得
    let p12_bytes = std::fs::read(cert_path)
        .with_context(|| format!("証明書ファイルの読み込みに失敗: {}", cert_path.display()))?;
    let pkcs12 = Pkcs12::from_der(&p12_bytes).context("PKCS#12の解析に失敗")?;
    let parsed = pkcs12.parse2(password).context("PKCS#12のパスワードが不正")?;

    let pkey = parsed.pkey.context("秘密鍵が見つかりません")?;
    let cert = parsed.cert.context("証明書が見つかりません")?;

    // lopdfで署名辞書を注入
    let mut doc = Document::load_mem(pdf_bytes).context("PDFの読み込みに失敗")?;

    // 署名辞書を作成
    let byterange_placeholder = format!(
        "[{pad} {pad} {pad} {pad}]",
        pad = BYTERANGE_PLACEHOLDER
    );

    let sig_dict = Dictionary::from_iter(vec![
        ("Type", Object::Name(b"Sig".to_vec())),
        ("Filter", Object::Name(b"Adobe.PPKLite".to_vec())),
        ("SubFilter", Object::Name(b"adbe.pkcs7.detached".to_vec())),
        (
            "ByteRange",
            Object::String(
                byterange_placeholder.as_bytes().to_vec(),
                StringFormat::Literal,
            ),
        ),
        (
            "Contents",
            Object::String(
                vec![0u8; SIGNATURE_PLACEHOLDER_SIZE / 2],
                StringFormat::Hexadecimal,
            ),
        ),
    ]);

    let sig_obj_id = doc.add_object(Object::Dictionary(sig_dict));

    // AcroForm と SigFlags を設定
    let page_ids: Vec<ObjectId> = doc.page_iter().collect();
    if let Some(&first_page_id) = page_ids.first() {
        // 署名ウィジェットアノテーション
        let widget_dict = Dictionary::from_iter(vec![
            ("Type", Object::Name(b"Annot".to_vec())),
            ("Subtype", Object::Name(b"Widget".to_vec())),
            ("FT", Object::Name(b"Sig".to_vec())),
            (
                "T",
                Object::String(b"Signature1".to_vec(), StringFormat::Literal),
            ),
            ("V", Object::Reference(sig_obj_id)),
            ("F", Object::Integer(132)), // Print + Locked
            (
                "Rect",
                Object::Array(vec![
                    Object::Integer(0),
                    Object::Integer(0),
                    Object::Integer(0),
                    Object::Integer(0),
                ]),
            ),
            ("P", Object::Reference(first_page_id)),
        ]);

        let widget_id = doc.add_object(Object::Dictionary(widget_dict));

        // ページにアノテーション追加
        if let Ok(page) = doc.get_object_mut(first_page_id) {
            if let Object::Dictionary(ref mut dict) = page {
                match dict.get_mut(b"Annots") {
                    Ok(Object::Array(ref mut arr)) => {
                        arr.push(Object::Reference(widget_id));
                    }
                    _ => {
                        dict.set(
                            "Annots",
                            Object::Array(vec![Object::Reference(widget_id)]),
                        );
                    }
                }
            }
        }

        // AcroForm設定
        let acro_form = Dictionary::from_iter(vec![
            ("Fields", Object::Array(vec![Object::Reference(widget_id)])),
            ("SigFlags", Object::Integer(3)), // SignaturesExist | AppendOnly
        ]);

        // Catalogを取得してAcroFormを設定
        let root_id = doc
            .trailer
            .get(b"Root")
            .ok()
            .and_then(|r| r.as_reference().ok());
        if let Some(root_id) = root_id {
            if let Ok(Object::Dictionary(ref mut catalog)) = doc.get_object_mut(root_id) {
                catalog.set("AcroForm", Object::Dictionary(acro_form));
            }
        }
    }

    // PDFをシリアライズ
    let mut serialized = Vec::new();
    doc.save_to(&mut serialized)
        .context("PDFのシリアライズに失敗")?;

    // Contents hex文字列の位置を見つける
    // lopdfはhex stringを <HEXHEX...> の形で出力する
    let hex_placeholder = "00".repeat(SIGNATURE_PLACEHOLDER_SIZE / 2);
    let contents_marker_str = format!("<{hex_placeholder}>");
    let contents_marker = contents_marker_str.as_bytes();

    let contents_pos = find_bytes(&serialized, contents_marker)
        .context("署名プレースホルダーが見つかりません")?;

    let contents_start = contents_pos + 1; // '<' の次
    let contents_end = contents_start + hex_placeholder.len(); // '>' の前

    // ByteRangeの値を計算
    let byte_range = [
        0usize,
        contents_pos,                         // 0 ～ '<'の直前
        contents_end + 1,                      // '>'の次から
        serialized.len() - (contents_end + 1), // ファイル末尾まで
    ];

    // ByteRange文字列を構築（固定幅）
    let br_value = format!(
        "[{:>width$} {:>width$} {:>width$} {:>width$}]",
        byte_range[0],
        byte_range[1],
        byte_range[2],
        byte_range[3],
        width = BYTERANGE_PLACEHOLDER.len()
    );

    // ByteRangeプレースホルダーを置換
    let br_placeholder = format!(
        "[{pad} {pad} {pad} {pad}]",
        pad = BYTERANGE_PLACEHOLDER
    );
    let br_placeholder_bytes = br_placeholder.as_bytes();
    let br_value_bytes = br_value.as_bytes();

    assert_eq!(
        br_placeholder_bytes.len(),
        br_value_bytes.len(),
        "ByteRangeプレースホルダーと値の長さが一致しません"
    );

    if let Some(br_pos) = find_bytes(&serialized, br_placeholder_bytes) {
        serialized[br_pos..br_pos + br_value_bytes.len()].copy_from_slice(br_value_bytes);
    }

    // 署名対象データを構築（/Contents以外の全バイト）
    let mut sign_data = Vec::new();
    sign_data.extend_from_slice(&serialized[byte_range[0]..byte_range[1]]);
    sign_data.extend_from_slice(&serialized[byte_range[2]..byte_range[2] + byte_range[3]]);

    // PKCS#7署名を生成
    let ca_stack = match parsed.ca {
        Some(ca) => ca,
        None => Stack::<X509>::new().context("CAスタックの作成に失敗")?,
    };

    let flags = Pkcs7Flags::DETACHED | Pkcs7Flags::BINARY;
    let pkcs7 = Pkcs7::sign(&cert, &pkey, &ca_stack, &sign_data, flags)
        .context("PKCS#7署名の生成に失敗")?;

    let sig_der = pkcs7.to_der().context("署名のDERエンコードに失敗")?;

    // 署名をhexエンコード
    let sig_hex = hex::encode(&sig_der);

    if sig_hex.len() > hex_placeholder.len() {
        anyhow::bail!(
            "署名サイズ({})がプレースホルダー({})を超えています",
            sig_hex.len(),
            hex_placeholder.len()
        );
    }

    // 署名をゼロパディングして固定長に
    let padded_sig = format!("{:0<width$}", sig_hex, width = hex_placeholder.len());

    // プレースホルダーに署名を書き込み
    serialized[contents_start..contents_end].copy_from_slice(padded_sig.as_bytes());

    Ok(serialized)
}

/// バイト列中から部分列を検索
fn find_bytes(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}
