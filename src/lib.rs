#![cfg_attr(docsrs, feature(doc_auto_cfg))]
#![doc = include_str!("../README.md")]

/// The format of the frontmatter.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FrontmatterFormat {
    /// JSON frontmatter, denoted by `{...}`.
    Json,
    /// TOML frontmatter, denoted by `+++...+++`.
    Toml,
    /// YAML frontmatter, denoted by `---...---`.
    Yaml,
}

/// The result of splitting a document into frontmatter and body.
#[derive(Debug, Clone, Copy)]
pub struct SplitFrontmatter<'a> {
    /// The body of the document.
    pub body: &'a str,
    /// The format of the frontmatter, if any.
    pub format: Option<FrontmatterFormat>,
    /// The frontmatter, if any.
    pub frontmatter: Option<&'a str>,
}

/// The result of parsing a document's frontmatter.
#[derive(Debug, Clone, Copy)]
pub struct ParsedFrontmatter<'a, T> {
    /// The body of the document.
    pub body: &'a str,
    /// The format of the frontmatter, if any.
    pub format: Option<FrontmatterFormat>,
    /// The parsed frontmatter, if any.
    pub frontmatter: Option<T>,
}

/// The crates error type
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Frontmatter format is disabled.
    #[error("disabled format {0:?}, enable corresponding cargo feature")]
    DisabledFormat(FrontmatterFormat),
    /// Closing delimiter is absent.
    #[error("absent closing {0:?} delimiter")]
    AbsentClosingDelimiter(FrontmatterFormat),

    #[cfg(feature = "yaml")]
    /// Invalid YAML syntax.
    #[error("invalid YAML syntax")]
    InvalidYaml(#[source] serde_yml::Error),
    #[cfg(feature = "json")]
    /// Invalid JSON syntax.
    #[error("invalid JSON syntax")]
    InvalidJson(#[source] serde_json::Error),
    #[cfg(feature = "toml")]
    /// Invalid TOML syntax.
    #[error("invalid TOML syntax")]
    InvalidToml(#[source] toml::de::Error),

    #[cfg(feature = "yaml")]
    /// Couldn't deserialize YAML into the target type.
    #[error("couldn't deserialize YAML")]
    DeserializeYaml(#[source] serde_yml::Error),
    #[cfg(feature = "json")]
    /// Couldn't deserialize JSON into the target type.
    #[error("couldn't deserialize JSON")]
    DeserializeJson(#[source] serde_json::Error),
    #[cfg(feature = "toml")]
    /// Couldn't deserialize TOML into the target type.
    #[error("couldn't deserialize TOML")]
    DeserializeToml(#[source] toml::de::Error),
}

/// Splits a document into frontmatter and body, returning the raw frontmatter
/// string and the body of the document.
///
/// # Arguments
///
/// * `content` - The content of the document to split.
///
/// # Examples
///
/// ```
/// use markdown_frontmatter::{split, FrontmatterFormat};
///
/// let doc = r#"---
/// title: Hello
/// ---
/// World
/// "#;
///
/// let result = split(doc).unwrap();
/// assert_eq!(result.format, Some(FrontmatterFormat::Yaml));
/// assert_eq!(result.frontmatter, Some("title: Hello\n"));
/// assert_eq!(result.body, "World\n");
/// ```
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

    let matter_start = match format {
        FrontmatterFormat::Json => span.start, // include opening curly bracket,
        FrontmatterFormat::Toml | FrontmatterFormat::Yaml => span.next_start,
    };

    let closing_delimiter = format.delimiter().1;
    for span in lines {
        if span.line != closing_delimiter {
            continue;
        }
        let (matter, body) = match format {
            FrontmatterFormat::Json => (
                &content[matter_start..span.next_start], // include closing curly bracket
                &content[span.next_start..],
            ),
            FrontmatterFormat::Toml | FrontmatterFormat::Yaml => (
                &content[matter_start..span.start], // exclude closing delimiter
                &content[span.next_start..],
            ),
        };
        return Ok(SplitFrontmatter {
            body,
            format: Some(format),
            frontmatter: Some(matter),
        });
    }
    Err(Error::AbsentClosingDelimiter(format))
}

#[cfg(any(feature = "json", feature = "toml", feature = "yaml"))]
/// Parses frontmatter from a markdown string, deserializing it into a given
/// type and returning the parsed frontmatter and the body of the document.
///
/// # Arguments
///
/// * `content` - The content of the document to parse.
///
/// # Examples
///
/// ```
/// use markdown_frontmatter::{parse, FrontmatterFormat};
/// use serde::Deserialize;
///
/// #[derive(Deserialize)]
/// struct MyFrontmatter {
///     title: String,
/// }
///
/// let doc = r#"---
/// title: Hello
/// ---
/// World
/// "#;
///
/// let result = parse::<MyFrontmatter>(doc).unwrap();
/// assert_eq!(result.format, Some(FrontmatterFormat::Yaml));
/// assert_eq!(result.frontmatter.unwrap().title, "Hello");
/// assert_eq!(result.body, "World\n");
/// ```
pub fn parse<T: serde::de::DeserializeOwned>(
    content: &str,
) -> Result<ParsedFrontmatter<'_, T>, Error> {
    let parts = split(content)?;
    let (Some(format), Some(matter_str)) = (parts.format, parts.frontmatter) else {
        return Ok(ParsedFrontmatter {
            body: parts.body,
            format: None,
            frontmatter: None,
        });
    };

    let matter = format.parse(matter_str)?;
    Ok(ParsedFrontmatter {
        body: parts.body,
        format: Some(format),
        frontmatter: Some(matter),
    })
}

impl FrontmatterFormat {
    const VARIANTS: [Self; 3] = [Self::Json, Self::Toml, Self::Yaml];

    /// Detects the frontmatter format from the first line of a document.
    fn detect(first_line: &str) -> Option<Self> {
        Self::VARIANTS
            .into_iter()
            .find(|&variant| first_line == variant.delimiter().0)
    }

    #[cfg(any(feature = "json", feature = "toml", feature = "yaml"))]
    fn parse<T: serde::de::DeserializeOwned>(&self, matter_str: &str) -> Result<T, Error> {
        match self {
            #[cfg(feature = "json")]
            Self::Json => {
                let json: serde_json::Value =
                    serde_json::from_str(matter_str).map_err(Error::InvalidJson)?;
                serde_json::from_value(json).map_err(Error::DeserializeJson)
            }
            #[cfg(not(feature = "json"))]
            Self::Json => Err(Error::DisabledFormat(Self::Json)),

            #[cfg(feature = "toml")]
            Self::Toml => {
                let toml: toml::Value = toml::from_str(matter_str).map_err(Error::InvalidToml)?;
                toml.try_into().map_err(Error::DeserializeToml)
            }
            #[cfg(not(feature = "toml"))]
            Self::Toml => Err(Error::DisabledFormat(Self::Toml)),

            #[cfg(feature = "yaml")]
            Self::Yaml => {
                let yaml: serde_yml::Value =
                    serde_yml::from_str(matter_str).map_err(Error::InvalidYaml)?;
                serde_yml::from_value(yaml).map_err(Error::DeserializeYaml)
            }
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

#[cfg(test)]
mod test_line_span {
    use super::*;

    #[test]
    fn line_span() {
        let input = "line 1\r\nline 2\nline 3";
        let mut lines = LineSpan::new(input);

        let line1 = lines.next().unwrap();
        assert_eq!(line1.line, "line 1");
        assert_eq!(line1.start, 0);
        assert_eq!(line1.next_start, 8);

        let line2 = lines.next().unwrap();
        assert_eq!(line2.line, "line 2");
        assert_eq!(line2.start, 8);
        assert_eq!(line2.next_start, 15);

        let line3 = lines.next().unwrap();
        assert_eq!(line3.line, "line 3");
        assert_eq!(line3.start, 15);
        assert_eq!(line3.next_start, 21);

        assert!(lines.next().is_none());
    }
}

#[cfg(test)]
mod test_split {
    use super::*;

    #[test]
    fn empty_document() {
        let input = "";
        let result = split(input).unwrap();
        assert!(result.frontmatter.is_none());
        assert!(result.format.is_none());
        assert_eq!(result.body, "");
    }

    #[test]
    fn no_frontmatter() {
        let input = "hello world";
        let result = split(input).unwrap();
        assert!(result.frontmatter.is_none());
        assert!(result.format.is_none());
        assert_eq!(result.body, "hello world");
    }

    #[test]
    fn unclosed_json() {
        let input = "{\n\t\"foo\": \"bar\"\n";
        let result = split(input);
        assert!(matches!(
            result.unwrap_err(),
            Error::AbsentClosingDelimiter(FrontmatterFormat::Json)
        ));
    }

    #[test]
    fn unclosed_toml() {
        let input = "+++\nfoo = \"bar\"";
        let result = split(input);
        assert!(matches!(
            result.unwrap_err(),
            Error::AbsentClosingDelimiter(FrontmatterFormat::Toml)
        ));
    }

    #[test]
    fn unclosed_yaml() {
        let input = "---\nfoo: bar";
        let result = split(input);
        assert!(matches!(
            result.unwrap_err(),
            Error::AbsentClosingDelimiter(FrontmatterFormat::Yaml)
        ));
    }

    #[test]
    fn json_singleline() {
        let input = "{\n\t\"foo\": \"bar\"\n}\nhello world";
        let result = split(input).unwrap();
        assert_eq!(result.frontmatter.unwrap(), "{\n\t\"foo\": \"bar\"\n}\n");
        assert_eq!(result.format.unwrap(), FrontmatterFormat::Json);
        assert_eq!(result.body, "hello world");
    }

    #[test]
    fn json_multiline() {
        let input = "{\n\t\"foo\": \"bar\",\n\t\"baz\": 1\n}\nhello world";
        let result = split(input).unwrap();
        assert_eq!(
            result.frontmatter.unwrap(),
            "{\n\t\"foo\": \"bar\",\n\t\"baz\": 1\n}\n"
        );
        assert_eq!(result.format.unwrap(), FrontmatterFormat::Json);
        assert_eq!(result.body, "hello world");
    }

    #[test]
    fn toml_singleline() {
        let input = "+++\nfoo = \"bar\"\n+++\nhello world";
        let result = split(input).unwrap();
        assert_eq!(result.frontmatter.unwrap(), "foo = \"bar\"\n");
        assert_eq!(result.format.unwrap(), FrontmatterFormat::Toml);
        assert_eq!(result.body, "hello world");
    }

    #[test]
    fn toml_multiline() {
        let input = "+++\nfoo = \"bar\"\nbaz = 1\n+++\nhello world";
        let result = split(input).unwrap();
        assert_eq!(result.frontmatter.unwrap(), "foo = \"bar\"\nbaz = 1\n");
        assert_eq!(result.format.unwrap(), FrontmatterFormat::Toml);
        assert_eq!(result.body, "hello world");
    }

    #[test]
    fn yaml_singleline() {
        let input = "---\nfoo: bar\n---\nhello world";
        let result = split(input).unwrap();
        assert_eq!(result.frontmatter.unwrap(), "foo: bar\n");
        assert_eq!(result.format.unwrap(), FrontmatterFormat::Yaml);
        assert_eq!(result.body, "hello world");
    }

    #[test]
    fn yaml_multiline() {
        let input = "---\nfoo: bar\nbaz: 1\n---\nhello world";
        let result = split(input).unwrap();
        assert_eq!(result.frontmatter.unwrap(), "foo: bar\nbaz: 1\n");
        assert_eq!(result.format.unwrap(), FrontmatterFormat::Yaml);
        assert_eq!(result.body, "hello world");
    }
}

#[cfg(all(test, any(feature = "json", feature = "toml", feature = "yaml")))]
mod test_parse {
    use serde::Deserialize;

    use super::*;

    #[derive(Debug, PartialEq, Deserialize)]
    struct Frontmatter {
        foo: String,
        baz: i32,
    }

    #[test]
    fn empty_document() {
        let input = "";
        let result = parse::<Frontmatter>(input).unwrap();
        assert!(result.frontmatter.is_none());
        assert!(result.format.is_none());
        assert_eq!(result.body, "");
    }

    #[test]
    fn no_frontmatter() {
        let input = "hello world";
        let result = parse::<Frontmatter>(input).unwrap();
        assert!(result.frontmatter.is_none());
        assert!(result.format.is_none());
        assert_eq!(result.body, "hello world");
    }

    #[cfg(feature = "json")]
    mod json {
        use super::*;

        #[test]
        fn valid() {
            let input = "{\n\t\"foo\": \"bar\",\n\t\"baz\": 1\n}\nhello world";
            let result = parse::<Frontmatter>(input).unwrap();
            assert_eq!(
                result.frontmatter.unwrap(),
                Frontmatter {
                    foo: "bar".to_string(),
                    baz: 1
                }
            );
            assert_eq!(result.format.unwrap(), FrontmatterFormat::Json);
            assert_eq!(result.body, "hello world");
        }

        #[test]
        fn invalid_syntax() {
            let input = "{\n\t\"foo\": \"bar\",\n\t\"baz\": 1,\n}\nhello world";
            let result = parse::<Frontmatter>(input);
            assert!(matches!(result.unwrap_err(), Error::InvalidJson(..)));
        }

        #[test]
        fn invalid_type() {
            let input = "{\n\t\"foo\": \"bar\",\n\t\"baz\": \"not a number\"\n}\nhello world";
            let result = parse::<Frontmatter>(input);
            assert!(matches!(result.unwrap_err(), Error::DeserializeJson(..)));
        }
    }

    #[cfg(feature = "toml")]
    mod toml {
        use super::*;

        #[test]
        fn valid() {
            let input = "+++\nfoo = \"bar\"\nbaz = 1\n+++\nhello world";
            let result = parse::<Frontmatter>(input).unwrap();
            assert_eq!(
                result.frontmatter.unwrap(),
                Frontmatter {
                    foo: "bar".to_string(),
                    baz: 1
                }
            );
            assert_eq!(result.format.unwrap(), FrontmatterFormat::Toml);
            assert_eq!(result.body, "hello world");
        }

        #[test]
        fn invalid_syntax() {
            let input = "+++\nfoo = \"bar\"\nbaz = 1a\n+++\nhello world";
            let result = parse::<Frontmatter>(input);
            assert!(matches!(result.unwrap_err(), Error::InvalidToml(..)));
        }

        #[test]
        fn invalid_type() {
            let input = "+++\nfoo = \"bar\"\nbaz = \"not a number\"\n+++\nhello world";
            let result = parse::<Frontmatter>(input);
            assert!(matches!(result.unwrap_err(), Error::DeserializeToml(..)));
        }
    }

    #[cfg(feature = "yaml")]
    mod yaml {
        use super::*;

        #[test]
        fn valid() {
            let input = "---\nfoo: bar\nbaz: 1\n---\nhello world";
            let result = parse::<Frontmatter>(input).unwrap();
            assert_eq!(
                result.frontmatter.unwrap(),
                Frontmatter {
                    foo: "bar".to_string(),
                    baz: 1
                }
            );
            assert_eq!(result.format.unwrap(), FrontmatterFormat::Yaml);
            assert_eq!(result.body, "hello world");
        }

        #[test]
        fn invalid_syntax() {
            let input = "---\nfoo: bar\nbaz: 1\n- item\n---\nhello world";
            let result = parse::<Frontmatter>(input);
            assert!(matches!(result.unwrap_err(), Error::InvalidYaml(..)));
        }

        #[test]
        fn invalid_type() {
            let input = "---\nfoo: bar\nbaz: [1, 2, 3]\n---\nhello world";
            let result = parse::<Frontmatter>(input);
            assert!(matches!(result.unwrap_err(), Error::DeserializeYaml(..)));
        }
    }
}
