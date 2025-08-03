#![cfg_attr(docsrs, feature(doc_auto_cfg))]
#![doc = include_str!("../README.md")]

/// The format of the frontmatter.
#[derive(Debug, Clone, Copy, PartialEq)]
enum FrontmatterFormat {
    /// JSON frontmatter, denoted by `{...}`.
    Json,
    /// TOML frontmatter, denoted by `+++...+++`.
    Toml,
    /// YAML frontmatter, denoted by `---...---`.
    Yaml,
}

impl From<FrontmatterFormat> for &'static str {
    fn from(format: FrontmatterFormat) -> Self {
        match format {
            FrontmatterFormat::Json => "JSON",
            FrontmatterFormat::Toml => "TOML",
            FrontmatterFormat::Yaml => "YAML",
        }
    }
}

/// The crate's error type
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Frontmatter format is disabled.
    #[error("disabled format {0}, enable corresponding cargo feature")]
    DisabledFormat(&'static str),
    /// Closing delimiter is absent.
    #[error("absent closing {0} delimiter")]
    AbsentClosingDelimiter(&'static str),

    #[cfg(feature = "json")]
    /// Invalid JSON syntax.
    #[error("invalid JSON syntax")]
    InvalidJson(#[source] serde_json::Error),
    #[cfg(feature = "toml")]
    /// Invalid TOML syntax.
    #[error("invalid TOML syntax")]
    InvalidToml(#[source] toml::de::Error),
    #[cfg(feature = "yaml")]
    /// Invalid YAML syntax.
    #[error("invalid YAML syntax")]
    InvalidYaml(#[source] serde_yaml::Error),

    #[cfg(feature = "json")]
    /// Couldn't deserialize JSON into the target type.
    #[error("couldn't deserialize JSON")]
    DeserializeJson(#[source] serde_json::Error),
    #[cfg(feature = "toml")]
    /// Couldn't deserialize TOML into the target type.
    #[error("couldn't deserialize TOML")]
    DeserializeToml(#[source] toml::de::Error),
    #[cfg(feature = "yaml")]
    /// Couldn't deserialize YAML into the target type.
    #[error("couldn't deserialize YAML")]
    DeserializeYaml(#[source] serde_yaml::Error),
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
/// use markdown_frontmatter::parse;
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
/// let (frontmatter, body) = parse::<MyFrontmatter>(doc).unwrap();
/// assert_eq!(frontmatter.title, "Hello");
/// assert_eq!(body, "World\n");
/// ```
pub fn parse<T: serde::de::DeserializeOwned>(content: &str) -> Result<(T, &str), Error> {
    let (maybe_frontmatter, body) = split(content)?;
    let SplitFrontmatter(format, matter_str) = maybe_frontmatter.unwrap_or_default();
    let frontmatter = format.parse(matter_str)?;
    Ok((frontmatter, body))
}

#[derive(Debug, Clone, Copy)]
struct SplitFrontmatter<'a>(FrontmatterFormat, &'a str);

#[cfg(any(feature = "json", feature = "toml", feature = "yaml"))]
impl Default for SplitFrontmatter<'_> {
    fn default() -> Self {
        #[cfg(feature = "json")]
        {
            Self(FrontmatterFormat::Json, "{}")
        }
        #[cfg(all(not(feature = "json"), feature = "toml"))]
        {
            Self(FrontmatterFormat::Toml, "")
        }
        #[cfg(all(not(any(feature = "json", feature = "toml")), feature = "yaml"))]
        {
            Self(FrontmatterFormat::Yaml, "{}")
        }
    }
}

/// Splits a document into frontmatter and body, returning the raw frontmatter
/// string and the body of the document.
fn split(content: &str) -> Result<(Option<SplitFrontmatter<'_>>, &str), Error> {
    let content = content.trim_start();
    let mut lines = LineSpan::new(content);

    let Some(span) = lines.next() else {
        // Empty document
        return Ok((None, content));
    };

    let Some(format) = FrontmatterFormat::detect(span.line) else {
        // No frontmatter
        return Ok((None, content));
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
        return Ok((Some(SplitFrontmatter(format, matter)), body));
    }
    Err(Error::AbsentClosingDelimiter(format.into()))
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
            Self::Json => Err(Error::DisabledFormat(Self::Json.into())),

            #[cfg(feature = "toml")]
            Self::Toml => {
                let toml: toml::Value = toml::from_str(matter_str).map_err(Error::InvalidToml)?;
                toml.try_into().map_err(Error::DeserializeToml)
            }
            #[cfg(not(feature = "toml"))]
            Self::Toml => Err(Error::DisabledFormat(Self::Toml.into())),

            #[cfg(feature = "yaml")]
            Self::Yaml => {
                let yaml: serde_yaml::Value =
                    serde_yaml::from_str(matter_str).map_err(Error::InvalidYaml)?;
                serde_yaml::from_value(yaml).map_err(Error::DeserializeYaml)
            }
            #[cfg(not(feature = "yaml"))]
            Self::Yaml => Err(Error::DisabledFormat(Self::Yaml.into())),
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
        let (frontmatter, body) = split(input).unwrap();
        assert!(frontmatter.is_none());
        assert_eq!(body, "");
    }

    #[test]
    fn no_frontmatter() {
        let input = "hello world";
        let (frontmatter, body) = split(input).unwrap();
        assert!(frontmatter.is_none());
        assert_eq!(body, "hello world");
    }

    #[test]
    fn unclosed_json() {
        let input = "{\n\t\"foo\": \"bar\"\n";
        let result = split(input);
        assert!(matches!(
            result.unwrap_err(),
            Error::AbsentClosingDelimiter("JSON")
        ));
    }

    #[test]
    fn unclosed_toml() {
        let input = "+++\nfoo = \"bar\"";
        let result = split(input);
        assert!(matches!(
            result.unwrap_err(),
            Error::AbsentClosingDelimiter("TOML")
        ));
    }

    #[test]
    fn unclosed_yaml() {
        let input = "---\nfoo: bar";
        let result = split(input);
        assert!(matches!(
            result.unwrap_err(),
            Error::AbsentClosingDelimiter("YAML")
        ));
    }

    #[test]
    fn json_singleline() {
        let input = "{\n\t\"foo\": \"bar\"\n}\nhello world";
        let (frontmatter, body) = split(input).unwrap();
        assert_eq!(frontmatter.unwrap().1, "{\n\t\"foo\": \"bar\"\n}\n");
        assert_eq!(frontmatter.unwrap().0, FrontmatterFormat::Json);
        assert_eq!(body, "hello world");
    }

    #[test]
    fn json_multiline() {
        let input = "{\n\t\"foo\": \"bar\",\n\t\"baz\": 1\n}\nhello world";
        let (frontmatter, body) = split(input).unwrap();
        assert_eq!(
            frontmatter.unwrap().1,
            "{\n\t\"foo\": \"bar\",\n\t\"baz\": 1\n}\n"
        );
        assert_eq!(frontmatter.unwrap().0, FrontmatterFormat::Json);
        assert_eq!(body, "hello world");
    }

    #[test]
    fn toml_singleline() {
        let input = "+++\nfoo = \"bar\"\n+++\nhello world";
        let (frontmatter, body) = split(input).unwrap();
        assert_eq!(frontmatter.unwrap().1, "foo = \"bar\"\n");
        assert_eq!(frontmatter.unwrap().0, FrontmatterFormat::Toml);
        assert_eq!(body, "hello world");
    }

    #[test]
    fn toml_multiline() {
        let input = "+++\nfoo = \"bar\"\nbaz = 1\n+++\nhello world";
        let (frontmatter, body) = split(input).unwrap();
        assert_eq!(frontmatter.unwrap().1, "foo = \"bar\"\nbaz = 1\n");
        assert_eq!(frontmatter.unwrap().0, FrontmatterFormat::Toml);
        assert_eq!(body, "hello world");
    }

    #[test]
    fn yaml_singleline() {
        let input = "---\nfoo: bar\n---\nhello world";
        let (frontmatter, body) = split(input).unwrap();
        assert_eq!(frontmatter.unwrap().1, "foo: bar\n");
        assert_eq!(frontmatter.unwrap().0, FrontmatterFormat::Yaml);
        assert_eq!(body, "hello world");
    }

    #[test]
    fn yaml_multiline() {
        let input = "---\nfoo: bar\nbaz: 1\n---\nhello world";
        let (frontmatter, body) = split(input).unwrap();
        assert_eq!(frontmatter.unwrap().1, "foo: bar\nbaz: 1\n");
        assert_eq!(frontmatter.unwrap().0, FrontmatterFormat::Yaml);
        assert_eq!(body, "hello world");
    }
}

#[cfg(all(test, any(feature = "json", feature = "toml", feature = "yaml")))]
mod test_parse {
    use serde::Deserialize;

    use super::*;

    #[derive(Debug, PartialEq, Deserialize)]
    struct OptionalFrontmatter {
        foo: Option<bool>,
    }

    #[derive(Debug, PartialEq, Deserialize)]
    struct RequiredFrontmatter {
        foo: bool,
    }

    #[derive(Debug, PartialEq, Deserialize)]
    struct EmptyFrontmatter {}

    const EMPTY_DOCUMENT: &str = "";
    const DOCUMENT_WITHOUT_FRONTMATTER: &str = "hello world";

    const EMPTY_FRONTMATTER: EmptyFrontmatter = EmptyFrontmatter {};
    const OPTIONAL_FRONTMATTER_SOME: OptionalFrontmatter = OptionalFrontmatter { foo: Some(true) };
    const OPTIONAL_FRONTMATTER_NONE: OptionalFrontmatter = OptionalFrontmatter { foo: None };
    const REQUIRED_FRONTMATTER: RequiredFrontmatter = RequiredFrontmatter { foo: true };

    #[cfg(feature = "json")]
    mod json {
        use super::*;

        const VALID_DOCUMENT: &str = "{\n\t\"foo\": true\n}\nhello world";
        const INVALID_SYNTAX: &str = "{\n1\n}";
        const INVALID_TYPE: &str = "{\n\t\"foo\": 0\n}";

        #[test]
        fn empty_frontmatter_in_empty_document() {
            let (frontmatter, body) = parse::<EmptyFrontmatter>(EMPTY_DOCUMENT).unwrap();
            assert_eq!(frontmatter, EmptyFrontmatter {});
            assert_eq!(body, "");
        }

        #[test]
        fn optional_frontmatter_in_empty_document() {
            let (frontmatter, body) = parse::<OptionalFrontmatter>(EMPTY_DOCUMENT).unwrap();
            assert_eq!(frontmatter.foo, None);
            assert_eq!(body, "");
        }

        #[test]
        fn required_frontmatter_in_empty_document() {
            let result = parse::<RequiredFrontmatter>(EMPTY_DOCUMENT);
            assert!(matches!(result.unwrap_err(), Error::DeserializeJson(..)));
        }

        #[test]
        fn empty_frontmatter_in_document_without_frontmatter() {
            let (frontmatter, body) =
                parse::<EmptyFrontmatter>(DOCUMENT_WITHOUT_FRONTMATTER).unwrap();
            assert_eq!(frontmatter, EMPTY_FRONTMATTER);
            assert_eq!(body, DOCUMENT_WITHOUT_FRONTMATTER);
        }

        #[test]
        fn optional_frontmatter_in_document_without_frontmatter() {
            let (frontmatter, body) =
                parse::<OptionalFrontmatter>(DOCUMENT_WITHOUT_FRONTMATTER).unwrap();
            assert_eq!(frontmatter, OPTIONAL_FRONTMATTER_NONE);
            assert_eq!(body, DOCUMENT_WITHOUT_FRONTMATTER);
        }

        #[test]
        fn required_frontmatter_in_document_without_frontmatter() {
            let result = parse::<RequiredFrontmatter>(DOCUMENT_WITHOUT_FRONTMATTER);
            assert!(matches!(result.unwrap_err(), Error::DeserializeJson(..)));
        }

        #[test]
        fn optional_frontmatter_in_valid_document() {
            let (frontmatter, body) = parse::<OptionalFrontmatter>(VALID_DOCUMENT).unwrap();
            assert_eq!(frontmatter, OPTIONAL_FRONTMATTER_SOME);
            assert_eq!(body, "hello world");
        }

        #[test]
        fn required_frontmatter_in_valid_document() {
            let (frontmatter, body) = parse::<RequiredFrontmatter>(VALID_DOCUMENT).unwrap();
            assert_eq!(frontmatter, REQUIRED_FRONTMATTER);
            assert_eq!(body, "hello world");
        }

        #[test]
        fn optional_frontmatter_invalid_syntax() {
            let result = parse::<OptionalFrontmatter>(INVALID_SYNTAX);
            assert!(matches!(result.unwrap_err(), Error::InvalidJson(..)));
        }

        #[test]
        fn required_frontmatter_invalid_syntax() {
            let result = parse::<RequiredFrontmatter>(INVALID_SYNTAX);
            assert!(matches!(result.unwrap_err(), Error::InvalidJson(..)));
        }

        #[test]
        fn optional_frontmatter_invalid_type() {
            let result = parse::<OptionalFrontmatter>(INVALID_TYPE);
            assert!(matches!(result.unwrap_err(), Error::DeserializeJson(..)));
        }

        #[test]
        fn required_frontmatter_invalid_type() {
            let result = parse::<RequiredFrontmatter>(INVALID_TYPE);
            assert!(matches!(result.unwrap_err(), Error::DeserializeJson(..)));
        }
    }

    #[cfg(feature = "toml")]
    mod toml {
        use super::*;

        const VALID_DOCUMENT: &str = "+++\nfoo = true\n+++\nhello world";
        const INVALID_SYNTAX: &str = "+++\nfoobar\n+++\n";
        const INVALID_TYPE: &str = "+++\nfoo = 123\n+++\n";

        #[cfg(not(any(feature = "json", feature = "yaml")))]
        mod only {
            use super::*;

            #[test]
            fn empty_frontmatter_in_empty_document() {
                let (frontmatter, body) = parse::<EmptyFrontmatter>(EMPTY_DOCUMENT).unwrap();
                assert_eq!(frontmatter, EmptyFrontmatter {});
                assert_eq!(body, "");
            }

            #[test]
            fn optional_frontmatter_in_empty_document() {
                let (frontmatter, body) = parse::<OptionalFrontmatter>(EMPTY_DOCUMENT).unwrap();
                assert_eq!(frontmatter.foo, None);
                assert_eq!(body, "");
            }

            #[test]
            fn required_frontmatter_in_empty_document() {
                let result = parse::<RequiredFrontmatter>(EMPTY_DOCUMENT);
                assert!(matches!(result.unwrap_err(), Error::DeserializeToml(..)));
            }

            #[test]
            fn empty_frontmatter_in_document_without_frontmatter() {
                let (frontmatter, body) =
                    parse::<EmptyFrontmatter>(DOCUMENT_WITHOUT_FRONTMATTER).unwrap();
                assert_eq!(frontmatter, EMPTY_FRONTMATTER);
                assert_eq!(body, DOCUMENT_WITHOUT_FRONTMATTER);
            }

            #[test]
            fn optional_frontmatter_in_document_without_frontmatter() {
                let (frontmatter, body) =
                    parse::<OptionalFrontmatter>(DOCUMENT_WITHOUT_FRONTMATTER).unwrap();
                assert_eq!(frontmatter, OPTIONAL_FRONTMATTER_NONE);
                assert_eq!(body, DOCUMENT_WITHOUT_FRONTMATTER);
            }

            #[test]
            fn required_frontmatter_in_document_without_frontmatter() {
                let result = parse::<RequiredFrontmatter>(DOCUMENT_WITHOUT_FRONTMATTER);
                assert!(matches!(result.unwrap_err(), Error::DeserializeToml(..)));
            }
        }

        #[test]
        fn optional_frontmatter_in_valid_document() {
            let (frontmatter, body) = parse::<OptionalFrontmatter>(VALID_DOCUMENT).unwrap();
            assert_eq!(frontmatter, OPTIONAL_FRONTMATTER_SOME);
            assert_eq!(body, "hello world");
        }

        #[test]
        fn required_frontmatter_in_valid_document() {
            let (frontmatter, body) = parse::<RequiredFrontmatter>(VALID_DOCUMENT).unwrap();
            assert_eq!(frontmatter, REQUIRED_FRONTMATTER);
            assert_eq!(body, "hello world");
        }

        #[test]
        fn optional_frontmatter_invalid_syntax() {
            let result = parse::<OptionalFrontmatter>(INVALID_SYNTAX);
            assert!(matches!(result.unwrap_err(), Error::InvalidToml(..)));
        }

        #[test]
        fn required_frontmatter_invalid_syntax() {
            let result = parse::<RequiredFrontmatter>(INVALID_SYNTAX);
            assert!(matches!(result.unwrap_err(), Error::InvalidToml(..)));
        }

        #[test]
        fn optional_frontmatter_invalid_type() {
            let result = parse::<OptionalFrontmatter>(INVALID_TYPE);
            assert!(matches!(result.unwrap_err(), Error::DeserializeToml(..)));
        }

        #[test]
        fn required_frontmatter_invalid_type() {
            let result = parse::<RequiredFrontmatter>(INVALID_TYPE);
            assert!(matches!(result.unwrap_err(), Error::DeserializeToml(..)));
        }
    }

    #[cfg(feature = "yaml")]
    mod yaml {
        use super::*;

        const VALID_DOCUMENT: &str = "---\nfoo: true\n---\nhello world";
        const INVALID_SYNTAX: &str = "---\n:\n---\n";
        const INVALID_TYPE: &str = "---\nfoo: 123\n---\n";

        #[cfg(not(any(feature = "json", feature = "toml")))]
        mod only {
            use super::*;

            #[test]
            fn empty_frontmatter_in_empty_document() {
                let (frontmatter, body) = parse::<EmptyFrontmatter>(EMPTY_DOCUMENT).unwrap();
                assert_eq!(frontmatter, EmptyFrontmatter {});
                assert_eq!(body, "");
            }

            #[test]
            fn optional_frontmatter_in_empty_document() {
                let (frontmatter, body) = parse::<OptionalFrontmatter>(EMPTY_DOCUMENT).unwrap();
                assert_eq!(frontmatter.foo, None);
                assert_eq!(body, "");
            }

            #[test]
            fn required_frontmatter_in_empty_document() {
                let result = parse::<RequiredFrontmatter>(EMPTY_DOCUMENT);
                assert!(matches!(result.unwrap_err(), Error::DeserializeYaml(..)));
            }

            #[test]
            fn empty_frontmatter_in_document_without_frontmatter() {
                let (frontmatter, body) =
                    parse::<EmptyFrontmatter>(DOCUMENT_WITHOUT_FRONTMATTER).unwrap();
                assert_eq!(frontmatter, EMPTY_FRONTMATTER);
                assert_eq!(body, DOCUMENT_WITHOUT_FRONTMATTER);
            }

            #[test]
            fn optional_frontmatter_in_document_without_frontmatter() {
                let (frontmatter, body) =
                    parse::<OptionalFrontmatter>(DOCUMENT_WITHOUT_FRONTMATTER).unwrap();
                assert_eq!(frontmatter, OPTIONAL_FRONTMATTER_NONE);
                assert_eq!(body, DOCUMENT_WITHOUT_FRONTMATTER);
            }

            #[test]
            fn required_frontmatter_in_document_without_frontmatter() {
                let result = parse::<RequiredFrontmatter>(DOCUMENT_WITHOUT_FRONTMATTER);
                assert!(matches!(result.unwrap_err(), Error::DeserializeYaml(..)));
            }
        }

        #[test]
        fn optional_frontmatter_in_valid_document() {
            let (frontmatter, body) = parse::<OptionalFrontmatter>(VALID_DOCUMENT).unwrap();
            assert_eq!(frontmatter, OPTIONAL_FRONTMATTER_SOME);
            assert_eq!(body, "hello world");
        }

        #[test]
        fn required_frontmatter_in_valid_document() {
            let (frontmatter, body) = parse::<RequiredFrontmatter>(VALID_DOCUMENT).unwrap();
            assert_eq!(frontmatter, REQUIRED_FRONTMATTER);
            assert_eq!(body, "hello world");
        }

        #[test]
        fn optional_frontmatter_invalid_syntax() {
            let result = parse::<OptionalFrontmatter>(INVALID_SYNTAX);
            assert!(matches!(result.unwrap_err(), Error::InvalidYaml(..)));
        }

        #[test]
        fn required_frontmatter_invalid_syntax() {
            let result = parse::<RequiredFrontmatter>(INVALID_SYNTAX);
            assert!(matches!(result.unwrap_err(), Error::InvalidYaml(..)));
        }

        #[test]
        fn optional_frontmatter_invalid_type() {
            let result = parse::<OptionalFrontmatter>(INVALID_TYPE);
            assert!(matches!(result.unwrap_err(), Error::DeserializeYaml(..)));
        }

        #[test]
        fn required_frontmatter_invalid_type() {
            let result = parse::<RequiredFrontmatter>(INVALID_TYPE);
            assert!(matches!(result.unwrap_err(), Error::DeserializeYaml(..)));
        }
    }
}
