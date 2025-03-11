// SPDX-FileCopyrightText: 2025 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::{iter::Peekable, ops::Range};

use flutter_rust_bridge::frb;
use pulldown_cmark::{Event, Options, Parser, Tag, TagEnd};

const MAX_DEPTH: usize = 50;

#[derive(thiserror::Error, Debug, PartialEq, Eq)]
pub enum Error {
    #[error("expected more events")]
    ExpectedMoreEvents,
    #[error("expected specific tag")]
    ExpectedSpecificTag,
    #[error("table content not in table")]
    TableContentNotInTable,
    #[error("list item not in list")]
    ListItemNotInList,
    #[error("metadata blocks not supported")]
    MetadataBlocksNotSupported,
    #[error("footnotes not supported")]
    FootnotesNotSupported,
    #[error("definition lists not supported")]
    DefinitionListsNotSupported,
    #[error("block element inline")]
    BlockElementInline,
    #[error("HTML not in block")]
    HtmlNotInBlock,
    #[error("math not supported")]
    MathNotSupported,
    #[error("depth limit reached")]
    DepthLimitReached,
    #[error("invalid UTF-8")]
    InvalidUtf8,
}

type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
#[frb]
pub struct MessageContent {
    pub content: Vec<RangedBlockElement>,
}

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
#[frb]
pub struct RangedInlineElement {
    pub range: (u32, u32),
    pub element: InlineElement,
}

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
#[frb]
pub struct RangedBlockElement {
    pub range: (u32, u32),
    pub element: BlockElement,
}

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
#[frb]
pub enum BlockElement {
    Paragraph(Vec<RangedInlineElement>),
    Heading(Vec<RangedInlineElement>),
    Quote(Vec<RangedBlockElement>),
    UnorderedList(Vec<Vec<RangedBlockElement>>), // Each item has multiple block elements
    OrderedList(u64, Vec<Vec<RangedBlockElement>>), // Each item has multiple block elements
    Table {
        head: Vec<Vec<RangedBlockElement>>,
        rows: Vec<Vec<Vec<RangedBlockElement>>>,
    },
    HorizontalRule,

    /// If code blocks are indented, each line is a separate String
    CodeBlock(Vec<((u32, u32), String)>), // Range and String

    Error(String),
}

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
#[frb]
pub enum InlineElement {
    Text(String),
    Code(String),
    Link {
        dest_url: String,
        children: Vec<RangedInlineElement>,
    },
    Bold(Vec<RangedInlineElement>),
    Italic(Vec<RangedInlineElement>),
    Strikethrough(Vec<RangedInlineElement>),
    Spoiler(Vec<RangedInlineElement>),
    Image(String),
    TaskListMarker(bool),
    //UserMention(String),
    //RoomMention(String),
    //Video,
    //Audio,
    //Voice,
    //Meeting,
    //File,
}

impl MessageContent {
    pub fn error(message: String) -> Self {
        Self {
            content: vec![RangedBlockElement {
                range: (
                    0,
                    u32::try_from(message.chars().count()).unwrap_or(u32::MAX),
                ),
                element: BlockElement::Error(message),
            }],
        }
    }

    #[frb(sync)]
    pub fn parse_markdown_raw(string: Vec<u8>) -> Result<Self> {
        Self::try_parse_markdown(&String::from_utf8(string).map_err(|_| Error::InvalidUtf8)?)
    }

    pub fn parse_markdown(string: &str) -> Self {
        Self::try_parse_markdown(string)
            .unwrap_or_else(|e| Self::error(format!("Invalid message: {e}")))
    }

    fn try_parse_markdown(string: &str) -> Result<Self> {
        let parsed = Parser::new_ext(
            string,
            // Do not enable Options::ENABLE_GFM, it activates special blockquotes which are not part of the GFM spec https://github.com/orgs/community/discussions/16925
            Options::ENABLE_STRIKETHROUGH | Options::ENABLE_TABLES | Options::ENABLE_TASKLISTS,
        )
        .into_offset_iter();
        let mut result = Vec::new();
        let mut iter = parsed
            .map(|(element, range)| {
                (
                    element,
                    u32::try_from(range.start).unwrap_or(u32::MAX)
                        ..u32::try_from(range.end).unwrap_or(u32::MAX),
                )
            })
            .peekable();

        while iter.peek().is_some() {
            result.push(parse_block_element(&mut iter, 1)?);
        }

        Ok(Self { content: result })
    }
}

fn parse_block_element<'a, I>(iter: &mut Peekable<I>, depth: usize) -> Result<RangedBlockElement>
where
    I: Iterator<Item = (Event<'a>, Range<u32>)>,
{
    if depth > MAX_DEPTH {
        return Err(Error::DepthLimitReached);
    }

    let peek = iter.peek().ok_or(Error::ExpectedMoreEvents)?;
    let block = match peek.clone().0 {
        Event::Start(Tag::Paragraph) => {
            let start = iter.next().expect("we already peeked");
            let value = BlockElement::Paragraph(parse_inline_elements(iter, depth + 1)?);
            let end = iter.next().ok_or(Error::ExpectedMoreEvents)?;

            if end.0 != Event::End(TagEnd::Paragraph) {
                return Err(Error::ExpectedSpecificTag);
            }

            RangedBlockElement {
                range: (start.1.start, end.1.end),
                element: value,
            }
        }
        Event::Start(Tag::Heading { level, .. }) => {
            let start = iter.next().expect("we already peeked");
            let value = BlockElement::Heading(parse_inline_elements(iter, depth + 1)?);
            let end = iter.next().ok_or(Error::ExpectedMoreEvents)?;

            if end.0 != Event::End(TagEnd::Heading(level)) {
                return Err(Error::ExpectedSpecificTag);
            }

            RangedBlockElement {
                range: (start.1.start, end.1.end),
                element: value,
            }
        }
        Event::Start(Tag::List(number)) => {
            let start = iter.next().expect("we already peeked");
            let value = match number {
                Some(s) => BlockElement::OrderedList(s, parse_list_items(iter, depth + 1)?),
                None => BlockElement::UnorderedList(parse_list_items(iter, depth + 1)?),
            };
            let end = iter.next().ok_or(Error::ExpectedMoreEvents)?;

            if end.0 != Event::End(TagEnd::List(number.is_some())) {
                return Err(Error::ExpectedSpecificTag);
            }

            RangedBlockElement {
                range: (start.1.start, end.1.end),
                element: value,
            }
        }
        Event::Start(Tag::Table(_alignments)) => {
            let start = iter.next().expect("we already peeked");
            let value = parse_table_content(iter, depth + 1)?;
            let end = iter.next().ok_or(Error::ExpectedMoreEvents)?;

            if end.0 != Event::End(TagEnd::Table) {
                return Err(Error::ExpectedSpecificTag);
            }

            RangedBlockElement {
                range: (start.1.start, end.1.end),
                element: value,
            }
        }
        Event::Start(Tag::BlockQuote(_)) => {
            let start = iter.next().expect("we already peeked");
            let mut quote_blocks = Vec::new();
            let end;
            loop {
                let peek = iter.peek().ok_or(Error::ExpectedMoreEvents)?;
                if matches!(peek.0, Event::End(TagEnd::BlockQuote(..))) {
                    end = iter.next().expect("we already peeked");
                    break;
                }
                quote_blocks.push(parse_block_element(iter, depth + 1)?);
            }

            RangedBlockElement {
                range: (start.1.start, end.1.end),
                element: BlockElement::Quote(quote_blocks),
            }
        }
        Event::Start(Tag::CodeBlock(_code_block_kind)) => {
            let start = iter.next().expect("we already peeked");
            let mut value = Vec::new();

            while let Event::Text(str) = iter.peek().ok_or(Error::ExpectedMoreEvents)?.clone().0 {
                let event = iter.next().expect("we already peeked");
                value.push(((event.1.start, event.1.end), str.to_string()));
            }

            // A code block cannot contain any other data
            let end = iter.next().ok_or(Error::ExpectedMoreEvents)?;

            if end.0 != Event::End(TagEnd::CodeBlock) {
                return Err(Error::ExpectedSpecificTag);
            }

            // // We need this code, otherwise there is an empty line at the end of code blocks
            // if value.ends_with('\n') {
            //     value.truncate(value.len() - 1);
            // }

            RangedBlockElement {
                range: (start.1.start, end.1.end),
                element: BlockElement::CodeBlock(value),
            }
        }
        Event::Rule => {
            let item = iter.next().expect("we already peeked");
            let value = BlockElement::HorizontalRule;

            RangedBlockElement {
                range: (item.1.start, item.1.end),
                element: value,
            }
        }

        // Create implicit paragraph for inline elements
        Event::Start(Tag::Emphasis)
        | Event::Start(Tag::Strong)
        | Event::Start(Tag::Strikethrough)
        | Event::Start(Tag::Link { .. })
        | Event::Start(Tag::Image { .. })
        | Event::Text(_)
        | Event::Code(_)
        | Event::TaskListMarker(_)
        | Event::SoftBreak
        | Event::HardBreak => {
            let inner = parse_inline_elements(iter, depth + 1)?;
            RangedBlockElement {
                range: (inner[0].range.0, inner[inner.len() - 1].range.1),
                element: BlockElement::Paragraph(inner),
            }
        }

        // The rest are invalid events
        Event::InlineMath(_) | Event::DisplayMath(_) => {
            return Err(Error::MathNotSupported);
        }

        Event::Start(Tag::HtmlBlock) => {
            let start = iter.next().expect("we already peeked");
            let mut value = Vec::new();

            while let Event::Html(str) = iter.peek().ok_or(Error::ExpectedMoreEvents)?.clone().0 {
                let event = iter.next().expect("we already peeked");
                value.push(RangedInlineElement {
                    range: (event.1.start, event.1.end),
                    element: InlineElement::Text(str.to_string()),
                });
            }

            // A code block cannot contain any other data
            let end = iter.next().ok_or(Error::ExpectedMoreEvents)?;

            if end.0 != Event::End(TagEnd::HtmlBlock) {
                return Err(Error::ExpectedSpecificTag);
            }

            RangedBlockElement {
                range: (start.1.start, end.1.end),
                element: BlockElement::Paragraph(value),
            }
        }

        Event::Html(_) | Event::InlineHtml(_) => {
            return Err(Error::HtmlNotInBlock);
        }

        Event::Start(Tag::Item) => return Err(Error::ListItemNotInList),

        Event::Start(Tag::FootnoteDefinition(_)) | Event::FootnoteReference(_) => {
            return Err(Error::FootnotesNotSupported);
        }

        Event::Start(Tag::DefinitionList)
        | Event::Start(Tag::DefinitionListTitle)
        | Event::Start(Tag::DefinitionListDefinition) => {
            return Err(Error::DefinitionListsNotSupported);
        }

        Event::Start(Tag::TableHead)
        | Event::Start(Tag::TableRow)
        | Event::Start(Tag::TableCell) => return Err(Error::TableContentNotInTable),

        Event::Start(Tag::MetadataBlock(_)) => {
            return Err(Error::MetadataBlocksNotSupported);
        }

        Event::End(_) => return Err(Error::ExpectedSpecificTag),
    };

    Ok(block)
}

fn parse_inline_elements<'a, I>(
    iter: &mut Peekable<I>,
    depth: usize,
) -> Result<Vec<RangedInlineElement>>
where
    I: Iterator<Item = (Event<'a>, Range<u32>)>,
{
    if depth > MAX_DEPTH {
        return Err(Error::DepthLimitReached);
    }

    let mut result = Vec::new();
    loop {
        let peek = iter.peek().ok_or(Error::ExpectedMoreEvents)?;
        match peek.clone().0 {
            Event::Start(Tag::Emphasis) => {
                let start = iter.next().expect("we already peeked");
                let value = InlineElement::Italic(parse_inline_elements(iter, depth + 1)?);
                let end = iter.next().ok_or(Error::ExpectedMoreEvents)?;

                if end.0 != Event::End(TagEnd::Emphasis) {
                    return Err(Error::ExpectedSpecificTag);
                }

                result.push(RangedInlineElement {
                    range: (start.1.start, end.1.end),
                    element: value,
                });
            }

            Event::Start(Tag::Strong) => {
                let start = iter.next().expect("we already peeked");
                let value = InlineElement::Bold(parse_inline_elements(iter, depth + 1)?);
                let end = iter.next().ok_or(Error::ExpectedMoreEvents)?;

                if end.0 != Event::End(TagEnd::Strong) {
                    return Err(Error::ExpectedSpecificTag);
                }

                result.push(RangedInlineElement {
                    range: (start.1.start, end.1.end),
                    element: value,
                });
            }
            Event::Start(Tag::Strikethrough) => {
                let start = iter.next().expect("we already peeked");
                let value = InlineElement::Strikethrough(parse_inline_elements(iter, depth + 1)?);
                let end = iter.next().ok_or(Error::ExpectedMoreEvents)?;

                if end.0 != Event::End(TagEnd::Strikethrough) {
                    return Err(Error::ExpectedSpecificTag);
                }

                result.push(RangedInlineElement {
                    range: (start.1.start, end.1.end),
                    element: value,
                });
            }

            Event::Start(Tag::Link { dest_url, .. }) => {
                let start = iter.next().expect("we already peeked");
                let value = InlineElement::Link {
                    dest_url: dest_url.to_string(),
                    children: parse_inline_elements(iter, depth + 1)?,
                };
                let end = iter.next().ok_or(Error::ExpectedMoreEvents)?;

                if end.0 != Event::End(TagEnd::Link) {
                    return Err(Error::ExpectedSpecificTag);
                }

                result.push(RangedInlineElement {
                    range: (start.1.start, end.1.end),
                    element: value,
                });
            }

            Event::Start(Tag::Image { dest_url, .. }) => {
                let start = iter.next().expect("we already peeked");
                let value = InlineElement::Image(dest_url.to_string());

                let _description = parse_inline_elements(iter, depth + 1)?;

                let end = iter.next().ok_or(Error::ExpectedMoreEvents)?;

                if end.0 != Event::End(TagEnd::Image) {
                    return Err(Error::ExpectedSpecificTag);
                }

                result.push(RangedInlineElement {
                    range: (start.1.start, end.1.end),
                    element: value,
                });
            }

            Event::Text(str) => {
                let value = iter.next().expect("we already peeked");
                result.push(RangedInlineElement {
                    range: (value.1.start, value.1.end),
                    element: InlineElement::Text(str.to_string()),
                });
            }

            Event::Code(str) => {
                let value = iter.next().expect("we already peeked");
                result.push(RangedInlineElement {
                    range: (value.1.start, value.1.end),
                    element: InlineElement::Code(str.to_string()),
                });
            }

            Event::SoftBreak | Event::HardBreak => {
                let value = iter.next().expect("we already peeked");
                result.push(RangedInlineElement {
                    range: (value.1.start, value.1.end),
                    element: InlineElement::Text("\n".to_owned()),
                });
            }

            Event::TaskListMarker(bool) => {
                let value = iter.next().expect("we already peeked");
                result.push(RangedInlineElement {
                    range: (value.1.start, value.1.end),
                    element: InlineElement::TaskListMarker(bool),
                });
            }

            // This is the end of the container
            Event::End(_) => return Ok(result),

            // Inline HTML should just show as text
            Event::InlineHtml(str) => {
                let value = iter.next().expect("we already peeked");
                result.push(RangedInlineElement {
                    range: (value.1.start, value.1.end),
                    element: InlineElement::Text(str.to_string()),
                });
            }

            // If a block element starts, this inline element has ended
            Event::Start(Tag::Paragraph)
            | Event::Start(Tag::Heading { .. })
            | Event::Start(Tag::BlockQuote(_))
            | Event::Start(Tag::CodeBlock(_))
            | Event::Start(Tag::List(_))
            | Event::Start(Tag::Table(_))
            | Event::Start(Tag::HtmlBlock)
            | Event::Rule
            | Event::Html(_) => return Ok(result),

            // The rest are invalid events
            Event::Start(Tag::TableHead)
            | Event::Start(Tag::TableRow)
            | Event::Start(Tag::TableCell) => return Err(Error::TableContentNotInTable),

            Event::Start(Tag::MetadataBlock(_)) => {
                return Err(Error::MetadataBlocksNotSupported);
            }

            Event::Start(Tag::Item) => return Err(Error::ListItemNotInList),

            Event::Start(Tag::FootnoteDefinition(_)) | Event::FootnoteReference(_) => {
                return Err(Error::FootnotesNotSupported);
            }

            Event::Start(Tag::DefinitionList)
            | Event::Start(Tag::DefinitionListTitle)
            | Event::Start(Tag::DefinitionListDefinition) => {
                return Err(Error::DefinitionListsNotSupported);
            }

            Event::InlineMath(_) | Event::DisplayMath(_) => {
                return Err(Error::MathNotSupported);
            }
        }
    }
}

fn parse_list_items<'a, I>(
    iter: &mut Peekable<I>,
    depth: usize,
) -> Result<Vec<Vec<RangedBlockElement>>>
where
    I: Iterator<Item = (Event<'a>, Range<u32>)>,
{
    if depth > MAX_DEPTH {
        return Err(Error::DepthLimitReached);
    }

    let mut items = Vec::new();

    loop {
        let peek = iter.peek().ok_or(Error::ExpectedMoreEvents)?;
        match peek.0 {
            Event::Start(Tag::Item) => {
                iter.next().expect("we already peeked");
                let mut item_blocks = Vec::new();
                loop {
                    let peek = iter.peek().ok_or(Error::ExpectedMoreEvents)?;
                    if peek.0 == Event::End(TagEnd::Item) {
                        iter.next().expect("we already peeked");
                        break;
                    }
                    item_blocks.push(parse_block_element(iter, depth + 1)?);
                }
                items.push(item_blocks);
            }

            // This is the end of the container
            Event::End(_) => return Ok(items),

            _ => return Err(Error::ExpectedSpecificTag),
        }
    }
}

fn parse_table_content<'a, I>(iter: &mut Peekable<I>, depth: usize) -> Result<BlockElement>
where
    I: Iterator<Item = (Event<'a>, Range<u32>)>,
{
    if depth > MAX_DEPTH {
        return Err(Error::DepthLimitReached);
    }

    if !matches!(iter.next(), Some((Event::Start(Tag::TableHead), _))) {
        return Err(Error::ExpectedSpecificTag);
    }

    let table_head = parse_table_cells(iter, depth + 1)?;

    if !matches!(iter.next(), Some((Event::End(TagEnd::TableHead), _))) {
        return Err(Error::ExpectedSpecificTag);
    }

    let mut table_rows = Vec::new();

    loop {
        let peek = iter.peek().ok_or(Error::ExpectedMoreEvents)?;
        match peek.0 {
            Event::Start(Tag::TableRow) => {
                iter.next().expect("we already peeked");
                let cells = parse_table_cells(iter, depth + 1)?;
                table_rows.push(cells);
                if !matches!(iter.next(), Some((Event::End(TagEnd::TableRow), _))) {
                    return Err(Error::ExpectedSpecificTag);
                }
            }

            // This is the end of the container
            Event::End(TagEnd::Table) => break,

            _ => return Err(Error::ExpectedSpecificTag),
        }
    }

    Ok(BlockElement::Table {
        head: table_head,
        rows: table_rows,
    })
}

fn parse_table_cells<'a, I>(
    iter: &mut Peekable<I>,
    depth: usize,
) -> Result<Vec<Vec<RangedBlockElement>>>
where
    I: Iterator<Item = (Event<'a>, Range<u32>)>,
{
    if depth > MAX_DEPTH {
        return Err(Error::DepthLimitReached);
    }

    let mut cells = Vec::new();

    loop {
        let peek = iter.peek().ok_or(Error::ExpectedMoreEvents)?;
        match peek.0 {
            Event::Start(Tag::TableCell) => {
                iter.next().expect("we already peeked");
                let mut cell_blocks = Vec::new();
                loop {
                    let peek = iter.peek().ok_or(Error::ExpectedMoreEvents)?;

                    if peek.0 == Event::End(TagEnd::TableCell) {
                        iter.next().expect("we already peeked");
                        break;
                    }
                    cell_blocks.push(parse_block_element(iter, depth + 1)?);
                }
                cells.push(cell_blocks);
            }

            // This is the end of the container
            Event::End(TagEnd::TableHead) | Event::End(TagEnd::TableRow) => return Ok(cells),

            _ => return Err(Error::ExpectedSpecificTag),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nested_images() {
        MessageContent::try_parse_markdown(r#"![hey *ho*](url)"#).unwrap();
        MessageContent::try_parse_markdown(r#"![![Bad link](img.jpg)](url)"#).unwrap();
    }

    #[test]
    fn raw_html() {
        MessageContent::try_parse_markdown(
            r#"<div><div><p><s>Oh no! Unclosed html tags!

But it ends after the paragraph"#,
        )
        .unwrap();
    }

    #[test]
    fn indented_code_block() {
        MessageContent::try_parse_markdown(
            r#"
    asdf
    asdf"#,
        )
        .unwrap();
    }

    #[test]
    fn max_depth() {
        // Test max depth using nested quotes
        MessageContent::try_parse_markdown(&">".repeat(MAX_DEPTH)).unwrap();
        assert_eq!(
            MessageContent::try_parse_markdown(&">".repeat(MAX_DEPTH + 1)),
            Err(Error::DepthLimitReached)
        );
    }
}
