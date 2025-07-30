#![cfg_attr(docsrs, feature(doc_auto_cfg))]
#![doc = include_str!("../README.md")]

#[derive(Debug, Clone, Copy)]
pub enum FrontmatterFormat {
    Json,
    Toml,
    Yaml,
}

#[derive(Debug, Clone, Copy)]
pub struct SplitFrontmatter<'a> {
    pub body: &'a str,
    pub format: Option<FrontmatterFormat>,
    pub frontmatter: Option<&'a str>,
}

#[derive(Debug, Clone, Copy)]
pub struct ParsedFrontmatter<'a, T> {
    pub body: &'a str,
    pub format: Option<FrontmatterFormat>,
    pub frontmatter: Option<T>,
}

/// The crates error type
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Disabled frontmatter format
    #[error("disabled format {0:?}, enable corresponding cargo feature")]
    DisabledFormat(FrontmatterFormat),
    /// Absent closing delimiter
    #[error("absent closing {0:?} delimiter")]
    AbsentClosingDelimiter(FrontmatterFormat),

    #[cfg(feature = "yaml")]
    /// Invalid YAML
    #[error("invalid YAML: {1}")]
    InvalidYaml(#[source] serde_yml::Error, String),
    #[cfg(feature = "json")]
    /// Invalid JSON
    #[error("invalid YAML: {1}")]
    InvalidJson(#[source] serde_json::Error, String),
    #[cfg(feature = "toml")]
    /// Invalid TOML
    #[error("invalid TOML: {1}")]
    InvalidToml(#[source] toml::de::Error, String),
}

pub fn split(content: &str) -> Result<SplitFrontmatter<'_>, Error> {
    let content = content.trim_start();
    let mut lines = LineSpan::new(content);

    let Some(span) = lines.next() else {
        // Empty document
        return Ok(SplitFrontmatter::empty(content));
    };

    let Some(format) = FrontmatterFormat::detect(span.line) else {
        // No frontmatter
        return Ok(SplitFrontmatter::empty(content));
    };

    let closing_delimiter = format.delimiter().1;
    for span in lines {
        if span.line == closing_delimiter {
            let matter = &content[..span.start];
            let body = &content[span.next_start..];
            return Ok(SplitFrontmatter {
                body,
                format: Some(format),
                frontmatter: Some(matter),
            });
        }
    }
    Err(Error::AbsentClosingDelimiter(format))
}

#[cfg(feature = "serde")]
/// Parses frontmatter from markdown string.
/// Returns the frontmatter and the rest of the content (page body)
pub fn parse<T: serde::de::DeserializeOwned>(
    content: &str,
) -> Result<ParsedFrontmatter<'_, T>, Error> {
    let split = split(content)?;
    let (Some(format), Some(matter_str)) = (split.format, split.frontmatter) else {
        return Ok(ParsedFrontmatter {
            body: split.body,
            format: None,
            frontmatter: None,
        });
    };

    let matter = format.parse(matter_str)?;
    Ok(ParsedFrontmatter {
        body: split.body,
        format: Some(format),
        frontmatter: Some(matter),
    })
}

impl FrontmatterFormat {
    const VARIANTS: [Self; 3] = [Self::Json, Self::Toml, Self::Yaml];

    /// Detects frontmatter, returns `None` if the document doesn't have one
    fn detect(first_line: &str) -> Option<Self> {
        Self::VARIANTS
            .into_iter()
            .find(|&variant| first_line == variant.delimiter().0)
    }

    #[cfg(feature = "serde")]
    fn parse<T: serde::de::DeserializeOwned>(&self, matter_str: &str) -> Result<T, Error> {
        match self {
            #[cfg(feature = "json")]
            Self::Json => serde_json::from_str(matter_str)
                .map_err(|e| Error::InvalidJson(e, matter_str.to_string())),
            #[cfg(not(feature = "json"))]
            Self::Json => Err(Error::DisabledFormat(Self::Json)),

            #[cfg(feature = "toml")]
            Self::Toml => toml::from_str(matter_str)
                .map_err(|e| Error::InvalidToml(e, matter_str.to_string())),
            #[cfg(not(feature = "toml"))]
            Self::Toml => Err(Error::DisabledFormat(Self::Toml)),

            #[cfg(feature = "yaml")]
            Self::Yaml => serde_yml::from_str(matter_str)
                .map_err(|e| Error::InvalidYaml(e, matter_str.to_string())),
            #[cfg(not(feature = "yaml"))]
            Self::Yaml => Err(Error::DisabledFormat(Self::Yaml)),
        }
    }

    fn delimiter(&self) -> (&'static str, &'static str) {
        match self {
            Self::Json => ("{", "}"),
            Self::Toml => ("+++", "+++"),
            Self::Yaml => ("---", "---"),
        }
    }
}

struct LineSpan<'a> {
    pub start: usize,
    pub next_start: usize,
    pub line: &'a str,
}

impl<'a> LineSpan<'a> {
    fn new(s: &'a str) -> impl Iterator<Item = LineSpan<'a>> + 'a {
        let bytes = s.as_bytes();
        let mut pos = 0;
        std::iter::from_fn(move || {
            if pos >= bytes.len() {
                return None;
            }
            let start = pos;
            let mut i = start;
            while i < bytes.len() && bytes[i] != b'\n' && bytes[i] != b'\r' {
                i += 1;
            }
            let line_end = i;
            if i < bytes.len() && bytes[i] == b'\r' {
                i += 1;
                if i < bytes.len() && bytes[i] == b'\n' {
                    i += 1;
                }
            } else if i < bytes.len() && bytes[i] == b'\n' {
                i += 1;
            }
            let line = &s[start..line_end];
            let next_start = i;
            pos = i;
            Some(LineSpan {
                start,
                next_start,
                line,
            })
        })
    }
}

impl<'a> SplitFrontmatter<'a> {
    fn empty(body: &'a str) -> SplitFrontmatter<'a> {
        Self {
            body,
            format: None,
            frontmatter: None,
        }
    }
}
