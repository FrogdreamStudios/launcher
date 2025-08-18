use dioxus::prelude::*;

#[derive(Debug, Clone)]
struct NewsItem {
    date: String,
    content: String,
}

fn markdown_to_html(markdown: &str) -> String {
    let mut html = String::new();
    let lines: Vec<&str> = markdown.lines().collect();
    let mut in_paragraph = false;

    for line in lines {
        let trimmed = line.trim();

        if trimmed.is_empty() {
            if in_paragraph {
                html.push_str("</p>\n");
                in_paragraph = false;
            }
            continue;
        }

        // Handle headers
        if let Some(stripped) = trimmed.strip_prefix("# ") {
            if in_paragraph {
                html.push_str("</p>\n");
                in_paragraph = false;
            }
            html.push_str(&format!("<h1>{stripped}</h1>\n"));
        } else if let Some(stripped) = trimmed.strip_prefix("## ") {
            if in_paragraph {
                html.push_str("</p>\n");
                in_paragraph = false;
            }
            html.push_str(&format!("<h2>{stripped}</h2>\n"));
        } else if let Some(stripped) = trimmed.strip_prefix("### ") {
            if in_paragraph {
                html.push_str("</p>\n");
                in_paragraph = false;
            }
            html.push_str(&format!("<h3>{stripped}</h3>\n"));
        } else {
            // Regular text - treat as paragraph
            if in_paragraph {
                html.push(' ');
            } else {
                html.push_str("<p>");
                in_paragraph = true;
            }
            html.push_str(trimmed);
        }
    }

    if in_paragraph {
        html.push_str("</p>\n");
    }

    html
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

        // Extract date from the frontmatter
        let date = frontmatter
            .lines()
            .find(|line| line.trim().starts_with("date:"))
            .map(|line| {
                // Remove the "date:" prefix and trim whitespace
                let date_part = line.trim().strip_prefix("date:").unwrap_or("").trim();
                // Remove quotes if present
                if date_part.starts_with('"') && date_part.ends_with('"') {
                    date_part[1..date_part.len() - 1].to_string()
                } else {
                    date_part.to_string()
                }
            })
            .unwrap_or_else(|| "Unknown date".to_string());

        // Convert Markdown to HTML using a simple parser
        let html_content = markdown_to_html(content_md);

        if !html_content.is_empty() {
            news_items.push(NewsItem {
                date,
                content: html_content,
            });
        }

        i += 2; // Skip to the next frontmatter section
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
