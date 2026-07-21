use anyhow::Result;
use pulldown_cmark::{Event, Options, Parser, Tag, TagEnd, html};

/// Frontmatterから抽出されたメタデータ
#[derive(Debug, Default)]
pub struct Metadata {
    pub title: Option<String>,
    pub doc_type: Option<String>,
    pub description: Option<String>,
    pub tags: Vec<String>,
    pub timestamp: Option<String>,
}

/// テキストのスタイル
#[derive(Debug, Clone, PartialEq)]
#[allow(dead_code)]
pub enum TextStyle {
    Normal,
    Bold,
    Italic,
    Code,
}

/// スタイル付きテキスト片
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct StyledText {
    pub text: String,
    pub style: TextStyle,
}

/// Markdownブロックの中間表現
#[derive(Debug)]
#[allow(dead_code)]
pub enum MdBlock {
    Heading {
        level: u8,
        text: Vec<StyledText>,
    },
    Paragraph {
        text: Vec<StyledText>,
    },
    List {
        items: Vec<Vec<StyledText>>,
        ordered: bool,
        start: Option<u64>,
    },
    Table {
        headers: Vec<Vec<StyledText>>,
        rows: Vec<Vec<Vec<StyledText>>>,
    },
    CodeBlock {
        language: Option<String>,
        code: String,
    },
    HorizontalRule,
}

#[allow(dead_code)]
pub fn parse_markdown(input: &str) -> Result<(Metadata, Vec<MdBlock>)> {
    let metadata = extract_frontmatter(input);

    let options = Options::ENABLE_YAML_STYLE_METADATA_BLOCKS
        | Options::ENABLE_TABLES
        | Options::ENABLE_STRIKETHROUGH;
    let parser = Parser::new_ext(input, options);

    let blocks = events_to_blocks(parser);
    Ok((metadata, blocks))
}

/// Markdownをfrontmatter除去済みのHTMLに変換
pub fn markdown_to_html(input: &str) -> Result<(Metadata, String)> {
    let metadata = extract_frontmatter(input);

    let options = Options::ENABLE_YAML_STYLE_METADATA_BLOCKS
        | Options::ENABLE_TABLES
        | Options::ENABLE_STRIKETHROUGH;
    let parser = Parser::new_ext(input, options);

    let mut in_metadata = false;
    let events: Vec<Event<'_>> = parser
        .filter(|event| {
            match event {
                Event::Start(Tag::MetadataBlock(_)) => {
                    in_metadata = true;
                    false
                }
                Event::End(TagEnd::MetadataBlock(_)) => {
                    in_metadata = false;
                    false
                }
                _ if in_metadata => false,
                _ => true,
            }
        })
        .collect();

    let mut html_output = String::new();
    html::push_html(&mut html_output, events.into_iter());

    Ok((metadata, html_output))
}

fn extract_frontmatter(input: &str) -> Metadata {
    let options = Options::ENABLE_YAML_STYLE_METADATA_BLOCKS;
    let parser = Parser::new_ext(input, options);

    let mut yaml_content = String::new();
    let mut in_metadata = false;

    for event in parser {
        match event {
            Event::Start(Tag::MetadataBlock(_)) => {
                in_metadata = true;
            }
            Event::Text(text) if in_metadata => {
                yaml_content.push_str(&text);
            }
            Event::End(TagEnd::MetadataBlock(_)) => {
                break;
            }
            _ => {
                if !in_metadata && !yaml_content.is_empty() {
                    break;
                }
            }
        }
    }

    if yaml_content.is_empty() {
        return Metadata::default();
    }

    parse_yaml_metadata(&yaml_content)
}

fn parse_yaml_metadata(yaml: &str) -> Metadata {
    let mut metadata = Metadata::default();

    let Ok(value) = serde_yaml::from_str::<serde_yaml::Value>(yaml) else {
        return metadata;
    };

    if let Some(map) = value.as_mapping() {
        if let Some(v) = map.get(&serde_yaml::Value::String("title".into())) {
            metadata.title = v.as_str().map(String::from);
        }
        if let Some(v) = map.get(&serde_yaml::Value::String("type".into())) {
            metadata.doc_type = v.as_str().map(String::from);
        }
        if let Some(v) = map.get(&serde_yaml::Value::String("description".into())) {
            metadata.description = v.as_str().map(String::from);
        }
        if let Some(v) = map.get(&serde_yaml::Value::String("timestamp".into())) {
            metadata.timestamp = v.as_str().map(String::from);
        }
        if let Some(v) = map.get(&serde_yaml::Value::String("tags".into())) {
            if let Some(seq) = v.as_sequence() {
                metadata.tags = seq
                    .iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect();
            }
        }
    }

    metadata
}

#[allow(dead_code)]
fn events_to_blocks(parser: Parser<'_>) -> Vec<MdBlock> {
    let mut blocks = Vec::new();
    let mut current_text: Vec<StyledText> = Vec::new();
    let mut style_stack: Vec<TextStyle> = vec![TextStyle::Normal];

    // テーブル状態
    let mut _in_table = false;
    let mut table_headers: Vec<Vec<StyledText>> = Vec::new();
    let mut table_rows: Vec<Vec<Vec<StyledText>>> = Vec::new();
    let mut current_row: Vec<Vec<StyledText>> = Vec::new();
    let mut in_table_head = false;

    // リスト状態
    let mut in_list = false;
    let mut list_ordered = false;
    let mut list_start: Option<u64> = None;
    let mut list_items: Vec<Vec<StyledText>> = Vec::new();

    // コードブロック状態
    let mut in_code_block = false;
    let mut code_lang: Option<String> = None;
    let mut code_content = String::new();

    // 見出し状態
    let mut _in_heading = false;
    let mut heading_level: u8 = 1;

    // メタデータブロックスキップ
    let mut in_metadata = false;

    for event in parser {
        match event {
            Event::Start(Tag::MetadataBlock(_)) => {
                in_metadata = true;
            }
            Event::End(TagEnd::MetadataBlock(_)) => {
                in_metadata = false;
            }
            _ if in_metadata => continue,

            Event::Start(Tag::Heading { level, .. }) => {
                _in_heading = true;
                heading_level = level as u8;
                current_text.clear();
            }
            Event::End(TagEnd::Heading(_)) => {
                _in_heading = false;
                if !current_text.is_empty() {
                    blocks.push(MdBlock::Heading {
                        level: heading_level,
                        text: std::mem::take(&mut current_text),
                    });
                }
            }

            Event::Start(Tag::Paragraph) => {
                current_text.clear();
            }
            Event::End(TagEnd::Paragraph) => {
                if !current_text.is_empty() && !in_list {
                    blocks.push(MdBlock::Paragraph {
                        text: std::mem::take(&mut current_text),
                    });
                }
            }

            Event::Start(Tag::Strong) => {
                style_stack.push(TextStyle::Bold);
            }
            Event::End(TagEnd::Strong) => {
                style_stack.pop();
            }
            Event::Start(Tag::Emphasis) => {
                style_stack.push(TextStyle::Italic);
            }
            Event::End(TagEnd::Emphasis) => {
                style_stack.pop();
            }

            Event::Start(Tag::List(start)) => {
                in_list = true;
                list_ordered = start.is_some();
                list_start = start;
                list_items.clear();
            }
            Event::End(TagEnd::List(_)) => {
                in_list = false;
                if !list_items.is_empty() {
                    blocks.push(MdBlock::List {
                        items: std::mem::take(&mut list_items),
                        ordered: list_ordered,
                        start: list_start,
                    });
                }
            }
            Event::Start(Tag::Item) => {
                current_text.clear();
            }
            Event::End(TagEnd::Item) => {
                list_items.push(std::mem::take(&mut current_text));
            }

            Event::Start(Tag::Table(_)) => {
                _in_table = true;
                table_headers.clear();
                table_rows.clear();
            }
            Event::End(TagEnd::Table) => {
                _in_table = false;
                blocks.push(MdBlock::Table {
                    headers: std::mem::take(&mut table_headers),
                    rows: std::mem::take(&mut table_rows),
                });
            }
            Event::Start(Tag::TableHead) => {
                in_table_head = true;
                current_row.clear();
            }
            Event::End(TagEnd::TableHead) => {
                in_table_head = false;
                table_headers = std::mem::take(&mut current_row);
            }
            Event::Start(Tag::TableRow) => {
                current_row.clear();
            }
            Event::End(TagEnd::TableRow) => {
                if !in_table_head {
                    table_rows.push(std::mem::take(&mut current_row));
                }
            }
            Event::Start(Tag::TableCell) => {
                current_text.clear();
            }
            Event::End(TagEnd::TableCell) => {
                current_row.push(std::mem::take(&mut current_text));
            }

            Event::Start(Tag::CodeBlock(kind)) => {
                in_code_block = true;
                code_content.clear();
                code_lang = match kind {
                    pulldown_cmark::CodeBlockKind::Fenced(lang) => {
                        let l = lang.to_string();
                        if l.is_empty() { None } else { Some(l) }
                    }
                    pulldown_cmark::CodeBlockKind::Indented => None,
                };
            }
            Event::End(TagEnd::CodeBlock) => {
                in_code_block = false;
                blocks.push(MdBlock::CodeBlock {
                    language: code_lang.take(),
                    code: std::mem::take(&mut code_content),
                });
            }

            Event::Text(text) => {
                if in_code_block {
                    code_content.push_str(&text);
                } else {
                    let style = style_stack.last().cloned().unwrap_or(TextStyle::Normal);
                    current_text.push(StyledText {
                        text: text.to_string(),
                        style,
                    });
                }
            }
            Event::Code(text) => {
                current_text.push(StyledText {
                    text: text.to_string(),
                    style: TextStyle::Code,
                });
            }
            Event::SoftBreak | Event::HardBreak => {
                current_text.push(StyledText {
                    text: " ".to_string(),
                    style: TextStyle::Normal,
                });
            }
            Event::Rule => {
                blocks.push(MdBlock::HorizontalRule);
            }
            _ => {}
        }
    }

    blocks
}
