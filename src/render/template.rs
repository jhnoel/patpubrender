//! Section-placeholder templating for Markdown output.
//!
//! A template is plain text with `{{placeholder}}` tokens. Each placeholder is
//! replaced by a section the renderer produces in code; the template only
//! controls which sections appear, in what order, and what literal text wraps
//! them. There is no expression language and no dependency — substitution only.
//!
//! Recognized placeholders: `frontmatter`, `title`, `abstract`, `description`,
//! and `claims`.

use std::fmt;

/// The default layout, reproducing the standard rendered document.
pub const DEFAULT_TEMPLATE: &str =
    "{{frontmatter}}\n\n{{title}}\n\n{{abstract}}\n\n{{description}}\n\n{{claims}}\n";

/// A renderable section a placeholder can name.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Section {
    Frontmatter,
    Title,
    Abstract,
    Description,
    Claims,
}

impl Section {
    fn from_name(name: &str) -> Option<Self> {
        Some(match name {
            "frontmatter" => Section::Frontmatter,
            "title" => Section::Title,
            "abstract" => Section::Abstract,
            "description" => Section::Description,
            "claims" => Section::Claims,
            _ => return None,
        })
    }
}

/// The code-rendered section blocks a [`Template`] assembles.
pub struct Sections {
    pub frontmatter: String,
    pub title: String,
    pub r#abstract: String,
    pub description: String,
    pub claims: String,
}

impl Sections {
    fn get(&self, section: Section) -> &str {
        match section {
            Section::Frontmatter => &self.frontmatter,
            Section::Title => &self.title,
            Section::Abstract => &self.r#abstract,
            Section::Description => &self.description,
            Section::Claims => &self.claims,
        }
    }
}

#[derive(Debug)]
enum Segment {
    Literal(String),
    Placeholder(Section),
}

/// A parsed Markdown template.
#[derive(Debug)]
pub struct Template {
    segments: Vec<Segment>,
}

/// Why a template string could not be parsed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TemplateError {
    /// A `{{name}}` whose `name` is not a recognized section.
    UnknownPlaceholder(String),
    /// A `{{` with no closing `}}`.
    UnterminatedPlaceholder,
}

impl fmt::Display for TemplateError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TemplateError::UnknownPlaceholder(name) => write!(
                f,
                "unknown template placeholder {{{{{name}}}}}; expected one of \
                 frontmatter, title, abstract, description, claims"
            ),
            TemplateError::UnterminatedPlaceholder => {
                write!(
                    f,
                    "unterminated template placeholder: '{{{{' with no closing '}}}}'"
                )
            }
        }
    }
}

impl std::error::Error for TemplateError {}

impl Default for Template {
    fn default() -> Self {
        // The default template is a constant we control, so parsing it is infallible.
        Template::parse(DEFAULT_TEMPLATE).expect("default template is valid")
    }
}

impl Template {
    /// Parse a template string. Placeholder names are validated up front, so an
    /// invalid template fails here rather than silently dropping content.
    pub fn parse(input: &str) -> Result<Self, TemplateError> {
        let mut segments = Vec::new();
        let mut rest = input;

        while let Some(open) = rest.find("{{") {
            if open > 0 {
                segments.push(Segment::Literal(rest[..open].to_string()));
            }
            let after = &rest[open + 2..];
            let close = after
                .find("}}")
                .ok_or(TemplateError::UnterminatedPlaceholder)?;
            let name = after[..close].trim();
            let section = Section::from_name(name)
                .ok_or_else(|| TemplateError::UnknownPlaceholder(name.to_string()))?;
            segments.push(Segment::Placeholder(section));
            rest = &after[close + 2..];
        }
        if !rest.is_empty() {
            segments.push(Segment::Literal(rest.to_string()));
        }
        Ok(Template { segments })
    }

    /// Assemble `sections` into the final Markdown, collapsing the runs of blank
    /// lines that empty sections leave behind and trimming the edges.
    pub(crate) fn render(&self, sections: &Sections) -> String {
        let mut out = String::new();
        for segment in &self.segments {
            match segment {
                Segment::Literal(text) => out.push_str(text),
                Segment::Placeholder(section) => out.push_str(sections.get(*section)),
            }
        }
        normalize_blank_lines(&out)
    }
}

/// Collapse any run of 3+ newlines down to a single blank line, and trim leading
/// and trailing whitespace. Internal indentation (e.g. YAML) is preserved.
fn normalize_blank_lines(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut newline_run = 0usize;
    for ch in input.chars() {
        if ch == '\n' {
            newline_run += 1;
            if newline_run <= 2 {
                out.push('\n');
            }
        } else {
            newline_run = 0;
            out.push(ch);
        }
    }
    out.trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sections() -> Sections {
        Sections {
            frontmatter: "---\na: 1\n---".to_string(),
            title: "# Widget".to_string(),
            r#abstract: "## Abstract\n\nAn abstract.".to_string(),
            description: "## Description\n\nText.".to_string(),
            claims: "## Claims\n\n1. A claim.".to_string(),
        }
    }

    #[test]
    fn default_template_orders_frontmatter_title_then_sections() {
        let out = Template::default().render(&sections());
        assert!(out.starts_with("---\na: 1\n---\n\n# Widget\n\n## Abstract"));
        assert!(out.ends_with("## Claims\n\n1. A claim."));
        assert!(
            !out.contains("\n\n\n"),
            "blank runs must be collapsed:\n{out}"
        );
    }

    #[test]
    fn custom_template_reorders_and_wraps() {
        let t = Template::parse("# {{title}}\n\n> note\n\n{{claims}}").unwrap();
        let out = t.render(&sections());
        assert_eq!(out, "# # Widget\n\n> note\n\n## Claims\n\n1. A claim.");
    }

    #[test]
    fn empty_section_leaves_no_gap() {
        let mut s = sections();
        s.r#abstract = String::new();
        let out = Template::parse("{{title}}\n\n{{abstract}}\n\n{{claims}}")
            .unwrap()
            .render(&s);
        assert_eq!(out, "# Widget\n\n## Claims\n\n1. A claim.");
    }

    #[test]
    fn unknown_placeholder_is_rejected() {
        assert_eq!(
            Template::parse("{{bogus}}").unwrap_err(),
            TemplateError::UnknownPlaceholder("bogus".to_string())
        );
    }

    #[test]
    fn unterminated_placeholder_is_rejected() {
        assert_eq!(
            Template::parse("{{title").unwrap_err(),
            TemplateError::UnterminatedPlaceholder
        );
    }
}
