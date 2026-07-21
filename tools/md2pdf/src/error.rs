use std::fmt;

#[derive(Debug)]
#[allow(dead_code)]
pub enum Md2PdfError {
    Io(std::io::Error),
    FontNotFound(String),
    FontParse(String),
    MarkdownParse(String),
    PdfGeneration(String),
    Signing(String),
    Certificate(String),
}

impl fmt::Display for Md2PdfError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Md2PdfError::Io(e) => write!(f, "IOエラー: {e}"),
            Md2PdfError::FontNotFound(path) => write!(f, "フォントが見つかりません: {path}"),
            Md2PdfError::FontParse(msg) => write!(f, "フォントの解析に失敗: {msg}"),
            Md2PdfError::MarkdownParse(msg) => write!(f, "Markdownの解析に失敗: {msg}"),
            Md2PdfError::PdfGeneration(msg) => write!(f, "PDF生成エラー: {msg}"),
            Md2PdfError::Signing(msg) => write!(f, "署名エラー: {msg}"),
            Md2PdfError::Certificate(msg) => write!(f, "証明書エラー: {msg}"),
        }
    }
}

impl std::error::Error for Md2PdfError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Md2PdfError::Io(e) => Some(e),
            _ => None,
        }
    }
}

impl From<std::io::Error> for Md2PdfError {
    fn from(e: std::io::Error) -> Self {
        Md2PdfError::Io(e)
    }
}
