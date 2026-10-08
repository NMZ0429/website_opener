use console::{measure_text_width, truncate_str, Style};
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

    let rows = group_by_url(aliases);
    let header = "ALIAS";
    let col = rows
        .iter()
        .map(|(names, _)| measure_text_width(&names.join(", ")))
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
    for (names, url) in &rows {
        let pad = " ".repeat(col - measure_text_width(&names.join(", ")));
        let names: Vec<String> = names
            .iter()
            .map(|n| alias_style.apply_to(n).to_string())
            .collect();
        let url = match url_width {
            Some(w) => truncate_str(url, w, "…"),
            None => Cow::Borrowed(*url),
        };
        out += &format!(
            "{}{pad}  {}\n",
            names.join(", "),
            style_url(&url, opts.color)
        );
    }
    if opts.decorate {
        let summary = format!(
            "{} {} · {} {}",
            aliases.len(),
            if aliases.len() == 1 {
                "alias"
            } else {
                "aliases"
            },
            rows.len(),
            if rows.len() == 1 { "URL" } else { "URLs" },
        );
        out += &format!("\n{}\n", dim.apply_to(summary));
    }
    out
}

/// Renders `alias:url` lines for zsh's `_describe`.
///
/// Given the terminal width, a leading `https://` is dropped and URLs are cut with `…`
/// to fit the menu, rather than being clipped mid-character by zsh.
pub fn render_completions(aliases: &[(String, String)], width: Option<usize>) -> String {
    let url_width = width.and_then(|w| describe_url_width(aliases, w));
    let mut out = String::new();
    for (alias, url) in aliases {
        let url = match url_width {
            Some(w) => truncate_str(url.strip_prefix("https://").unwrap_or(url), w, "…"),
            None => Cow::Borrowed(url.as_str()),
        };
        // Escape colons and backslashes for zsh _describe format
        let alias = alias.replace('\\', "\\\\").replace(':', "\\:");
        let url = url.replace('\\', "\\\\");
        out += &format!("{alias}:{url}\n");
    }
    out
}

/// Columns left for a URL in zsh's `_describe` menu, or `None` to leave URLs whole.
///
/// `_describe` puts aliases sharing a URL on one line, laid out as a table: the n-th alias
/// of each line (in reverse input order) shares a column, columns are two spaces apart,
/// and `  -- ` precedes the URL. zsh clips lines at `COLUMNS - 2`. An alias table wider
/// than `COLUMNS / 2` (the `max-matches-width` default) switches zsh to a multi-line
/// layout, which we don't try to predict.
fn describe_url_width(aliases: &[(String, String)], cols: usize) -> Option<usize> {
    let mut col_widths: Vec<usize> = Vec::new();
    for (names, _) in group_by_url(aliases) {
        for (i, name) in names.iter().rev().enumerate() {
            let w = measure_text_width(name);
            match col_widths.get_mut(i) {
                Some(max) => *max = (*max).max(w),
                None => col_widths.push(w),
            }
        }
    }
    let table = col_widths.iter().sum::<usize>() + 2 * col_widths.len().saturating_sub(1);
    if table > cols / 2 {
        return None;
    }
    cols.checked_sub(table + 7).filter(|&w| w >= MIN_URL_WIDTH)
}

/// Groups aliases by URL, ordered by URL.
fn group_by_url(aliases: &[(String, String)]) -> Vec<(Vec<&str>, &str)> {
    let mut by_url: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    for (alias, url) in aliases {
        by_url.entry(url).or_default().push(alias);
    }
    by_url
        .into_iter()
        .map(|(url, names)| (names, url))
        .collect()
}

/// Dims a URL's scheme and path so its host stands out.
fn style_url(url: &str, color: bool) -> String {
    let host_start = url.find("://").map_or(0, |i| i + 3);
    let host_end = url[host_start..]
        .find(['/', '?', '#'])
        .map_or(url.len(), |i| host_start + i);
    let dim = |s: &str| {
        if s.is_empty() {
            String::new()
        } else {
            Style::new()
                .dim()
                .force_styling(color)
                .apply_to(s)
                .to_string()
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
        assert_ne!(colored, render_list(&sample(), &decorated(Some(40))));
        assert_eq!(
            console::strip_ansi_codes(&colored),
            render_list(&sample(), &decorated(Some(40)))
        );
    }

    #[test]
    fn style_url_dims_everything_but_the_host() {
        let dim = |s: &str| {
            Style::new()
                .dim()
                .force_styling(true)
                .apply_to(s)
                .to_string()
        };
        assert_eq!(
            style_url("https://github.com/NMZ0429?tab=repos", true),
            format!("{}github.com{}", dim("https://"), dim("/NMZ0429?tab=repos"))
        );
        assert_eq!(
            style_url("https://claude.ai", true),
            format!("{}claude.ai", dim("https://"))
        );
        assert_eq!(
            style_url("localhost:3000/x", true),
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

    #[test]
    fn completions_without_width_match_legacy_format() {
        let list = aliases(&[("a:b", "https://x.com/a\\b"), ("gh", "https://github.com")]);
        assert_eq!(
            render_completions(&list, None),
            "a\\:b:https://x.com/a\\\\b\ngh:https://github.com\n"
        );
    }

    #[test]
    fn completions_drop_https_and_fit_the_menu() {
        let list = aliases(&[
            ("gh", "https://github.com"),
            (
                "old",
                "http://example.com/0123456789/0123456789/0123456789/0123456789",
            ),
        ]);
        // Alias table is 3 wide ("old"), so URLs get 50 - 3 - 7 = 40 columns.
        assert_eq!(
            render_completions(&list, Some(50)),
            "gh:github.com\nold:http://example.com/0123456789/012345678…\n"
        );
    }

    // Widths below were measured from zsh 5.9's `_describe` output.
    #[test]
    fn describe_width_matches_zsh_layout() {
        let long = "https://x.com/0123456789/0123456789/0123456789/0123456789";
        let list = aliases(&[
            ("a1", long),
            ("a2", long),
            ("a3", long),
            ("a4", long),
            ("c", "https://claude.ai"),
            ("claude", "https://claude.ai"),
            ("gh", "https://github.com"),
        ]);
        // "a4      a3  a2  a1  -- " / "claude  c           -- " / "gh                  -- "
        assert_eq!(describe_url_width(&list, 60), Some(35));

        let one = "https://one.example.com/";
        let two = "https://two.example.com/";
        let list = aliases(&[
            ("b", one),
            ("k", two),
            ("mmm", one),
            ("qqqqqqqqqq", two),
            ("solo", "https://three.example.com/"),
            ("zzzzzz", one),
        ]);
        // "qqqqqqqqqq  k       -- " / "zzzzzz      mmm  b  -- "
        assert_eq!(describe_url_width(&list, 80), Some(55));
        // An 18-wide alias table exceeds 30 / 2, so zsh wraps aliases onto extra lines.
        assert_eq!(describe_url_width(&list, 30), None);
    }
}
