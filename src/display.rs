use console::{measure_text_width, pad_str, truncate_str, Alignment, Style};
use std::borrow::Cow;
use std::collections::BTreeMap;

/// URLs are never cut to fewer columns than this; narrower lines wrap instead.
const MIN_URL_WIDTH: usize = 20;

pub struct ListOptions {
    /// Emit ANSI colors.
    pub color: bool,
    /// Add the header, summary and empty-state hint.
    pub decorate: bool,
    /// Terminal width to fit URLs into; `None` never truncates.
    pub width: Option<usize>,
}

impl ListOptions {
    /// Output for pipes and redirects: unchanged from earlier releases.
    pub fn plain() -> Self {
        Self {
            color: false,
            decorate: false,
            width: None,
        }
    }
}

/// Renders `web list`: one row per URL with its aliases, ordered by URL.
pub fn render_list(aliases: &[(String, String)], opts: &ListOptions) -> String {
    let dim = Style::new().dim().force_styling(opts.color);
    if aliases.is_empty() {
        let mut out = String::from("No aliases registered.\n");
        if opts.decorate {
            out += &format!("{}\n", dim.apply_to("Add one with: web add <alias> <url>"));
        }
        return out;
    }

    let mut rows: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    for (alias, url) in aliases {
        rows.entry(url).or_default().push(alias);
    }
    let header = "ALIAS";
    let col = rows
        .values()
        .map(|names| measure_text_width(&names.join(", ")))
        .chain(opts.decorate.then_some(header.len()))
        .max()
        .unwrap_or(0);
    let url_width = opts
        .width
        .map(|w| w.saturating_sub(col + 2))
        .filter(|&w| w >= MIN_URL_WIDTH);

    let alias_style = Style::new().cyan().bold().force_styling(opts.color);
    let mut out = String::new();
    if opts.decorate {
        out += &format!("{}\n", dim.apply_to(format!("{header:<col$}  URL")));
    }
    for (url, names) in &rows {
        let names: Vec<String> = names
            .iter()
            .map(|n| alias_style.apply_to(n).to_string())
            .collect();
        let url = url_width.map_or(Cow::Borrowed(*url), |w| truncate_str(url, w, "…"));
        out += &format!(
            "{}  {}\n",
            pad_str(&names.join(", "), col, Alignment::Left, None),
            style_url(&url, &dim)
        );
    }
    if opts.decorate {
        let count =
            |n: usize, one: &str, many: &str| format!("{n} {}", if n == 1 { one } else { many });
        let summary = format!(
            "{} · {}",
            count(aliases.len(), "alias", "aliases"),
            count(rows.len(), "URL", "URLs")
        );
        out += &format!("\n{}\n", dim.apply_to(summary));
    }
    out
}

/// Applies `dim` to a URL's scheme and path so its host stands out.
fn style_url(url: &str, dim: &Style) -> String {
    let host_start = url.find("://").map_or(0, |i| i + 3);
    let host_end = url[host_start..]
        .find(['/', '?', '#'])
        .map_or(url.len(), |i| host_start + i);
    // console emits escape codes even for an empty string
    let dim = |s: &str| {
        if s.is_empty() {
            String::new()
        } else {
            dim.apply_to(s).to_string()
        }
    };
    format!(
        "{}{}{}",
        dim(&url[..host_start]),
        &url[host_start..host_end],
        dim(&url[host_end..])
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn aliases(pairs: &[(&str, &str)]) -> Vec<(String, String)> {
        pairs
            .iter()
            .map(|(a, u)| (a.to_string(), u.to_string()))
            .collect()
    }

    fn sample() -> Vec<(String, String)> {
        aliases(&[
            ("c", "https://claude.ai"),
            ("claude", "https://claude.ai"),
            ("gh", "https://github.com"),
            (
                "jira",
                "https://mycompany.atlassian.net/jira/software/projects/ABC/boards/12",
            ),
        ])
    }

    fn decorated(width: Option<usize>) -> ListOptions {
        ListOptions {
            color: false,
            decorate: true,
            width,
        }
    }

    #[test]
    fn plain_list_matches_legacy_format() {
        assert_eq!(
            render_list(&sample(), &ListOptions::plain()),
            "c, claude  https://claude.ai\n\
             gh         https://github.com\n\
             jira       https://mycompany.atlassian.net/jira/software/projects/ABC/boards/12\n"
        );
    }

    #[test]
    fn decorated_list_has_header_and_summary() {
        assert_eq!(
            render_list(&sample(), &decorated(None)),
            "ALIAS      URL\n\
             c, claude  https://claude.ai\n\
             gh         https://github.com\n\
             jira       https://mycompany.atlassian.net/jira/software/projects/ABC/boards/12\n\
             \n\
             4 aliases · 3 URLs\n"
        );
    }

    #[test]
    fn header_widens_short_alias_column() {
        let list = aliases(&[("gh", "https://github.com")]);
        assert_eq!(
            render_list(&list, &decorated(None)),
            "ALIAS  URL\ngh     https://github.com\n\n1 alias · 1 URL\n"
        );
    }

    #[test]
    fn long_urls_are_cut_to_terminal_width() {
        let out = render_list(&sample(), &decorated(Some(40)));
        let jira = out.lines().find(|l| l.starts_with("jira")).unwrap();
        assert_eq!(jira, "jira       https://mycompany.atlassian.…");
        assert_eq!(measure_text_width(jira), 40);
        assert!(out.contains("gh         https://github.com\n"));
    }

    #[test]
    fn urls_are_left_whole_when_too_little_room() {
        let out = render_list(&sample(), &decorated(Some(30)));
        assert!(
            out.contains("https://mycompany.atlassian.net/jira/software/projects/ABC/boards/12")
        );
    }

    #[test]
    fn color_only_adds_escape_codes() {
        let opts = ListOptions {
            color: true,
            decorate: true,
            width: Some(40),
        };
        let colored = render_list(&sample(), &opts);
        let plain = render_list(&sample(), &decorated(Some(40)));
        assert_ne!(colored, plain);
        assert_eq!(console::strip_ansi_codes(&colored), plain);
    }

    #[test]
    fn style_url_dims_everything_but_the_host() {
        let style = Style::new().dim().force_styling(true);
        let dim = |s: &str| style.apply_to(s).to_string();
        assert_eq!(
            style_url("https://github.com/NMZ0429?tab=repos", &style),
            format!("{}github.com{}", dim("https://"), dim("/NMZ0429?tab=repos"))
        );
        assert_eq!(
            style_url("https://claude.ai", &style),
            format!("{}claude.ai", dim("https://"))
        );
        assert_eq!(
            style_url("localhost:3000/x", &style),
            format!("localhost:3000{}", dim("/x"))
        );
    }

    #[test]
    fn alignment_uses_display_width() {
        let list = aliases(&[
            ("ドキュメント", "https://docs.rs"),
            ("gh", "https://github.com"),
        ]);
        let out = render_list(&list, &ListOptions::plain());
        let starts: Vec<usize> = out
            .lines()
            .map(|l| measure_text_width(&l[..l.find("https").unwrap()]))
            .collect();
        assert_eq!(starts, vec![14, 14]);
    }

    #[test]
    fn empty_list() {
        assert_eq!(
            render_list(&[], &ListOptions::plain()),
            "No aliases registered.\n"
        );
        assert_eq!(
            render_list(&[], &decorated(Some(80))),
            "No aliases registered.\nAdd one with: web add <alias> <url>\n"
        );
    }
}
