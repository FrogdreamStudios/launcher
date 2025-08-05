use dioxus::prelude::*;
use pulldown_cmark::{Event, Parser, Tag, TagEnd};

#[derive(Debug, Clone)]
struct NewsItem {
    date: String,
    content: String,
}

fn parse_markdown() -> Vec<NewsItem> {
    let markdown_content =
        include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/news/news.md"));

    let mut news_items = Vec::new();
    let sections: Vec<&str> = markdown_content.split("---").collect();

    // Process sections, skipping the first empty section
    let mut i = 1;
    while i + 1 < sections.len() {
        let frontmatter = sections[i].trim();
        let content_md = sections[i + 1].trim();

        // Extract date from frontmatter
        let date = frontmatter
            .lines()
            .find(|line| line.trim().starts_with("date:"))
            .map(|line| {
                // Remove "date:" prefix and trim whitespace
                let date_part = line.trim().strip_prefix("date:").unwrap_or("").trim();
                // Remove quotes if present
                if date_part.starts_with('"') && date_part.ends_with('"') {
                    date_part[1..date_part.len() - 1].to_string()
                } else {
                    date_part.to_string()
                }
            })
            .unwrap_or_else(|| "Unknown date".to_string());

        // Parse markdown content to HTML
        let parser = Parser::new(content_md);
        let mut html_content = String::new();

        for event in parser {
            match event {
                Event::Start(Tag::Heading { level, .. }) => match level {
                    pulldown_cmark::HeadingLevel::H1 => html_content.push_str("<h1>"),
                    pulldown_cmark::HeadingLevel::H2 => html_content.push_str("<h2>"),
                    pulldown_cmark::HeadingLevel::H3 => html_content.push_str("<h3>"),
                    _ => html_content.push_str("<h4>"),
                },
                Event::End(TagEnd::Heading(level)) => match level {
                    pulldown_cmark::HeadingLevel::H1 => html_content.push_str("</h1>"),
                    pulldown_cmark::HeadingLevel::H2 => html_content.push_str("</h2>"),
                    pulldown_cmark::HeadingLevel::H3 => html_content.push_str("</h3>"),
                    _ => html_content.push_str("</h4>"),
                },
                Event::Start(Tag::Paragraph) => html_content.push_str("<p>"),
                Event::End(TagEnd::Paragraph) => html_content.push_str("</p>"),
                Event::Text(text) => html_content.push_str(&text),
                Event::SoftBreak => html_content.push(' '),
                Event::HardBreak => html_content.push_str("<br>"),
                _ => {}
            }
        }

        if !html_content.is_empty() {
            news_items.push(NewsItem {
                date,
                content: html_content,
            });
        }

        i += 2; // Skip to next frontmatter section
    }

    news_items
}

#[component]
pub fn News(animations_played: bool) -> Element {
    let news_items = use_signal(parse_markdown);

    rsx! {
        div {
            class: if !animations_played { "news-block news-animate" } else { "news-block" },

            for (index, item) in news_items().iter().enumerate() {
                div {
                    key: "{index}",
                    class: "news-item",
                    div { class: "news-date", "{item.date}" }
                    div { dangerous_inner_html: "{item.content}" }
                }
            }
        }
    }
}
