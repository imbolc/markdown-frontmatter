# markdown-frontmatter

[![License](https://img.shields.io/crates/l/markdown-frontmatter.svg)](https://choosealicense.com/licenses/mit/)
[![Crates.io](https://img.shields.io/crates/v/markdown-frontmatter.svg)](https://crates.io/crates/markdown-frontmatter)
[![Docs.rs](https://docs.rs/markdown-frontmatter/badge.svg)](https://docs.rs/markdown-frontmatter)

A type-safe parser for Markdown frontmatter.

This crate provides a simple and efficient way to split and parse frontmatter
from Markdown documents.

## Supported Frontmatter Formats

The crate supports the following frontmatter formats and their corresponding
delimiters:

- **JSON**: Delimited by `{` on the first line and `}` on a closing line. The
  enclosed JSON content must be indented to be parsed correctly.
  ```text
  {
    "title": "JSON Frontmatter"
  }
  ```
- **TOML**: Delimited by `+++` on opening and closing lines.
  ```text
  +++
  title = "TOML Frontmatter"
  +++
  ```
- **YAML**: Delimited by `---` on opening and closing lines.
  ```text
  ---
  title: YAML Frontmatter
  ---
  ```

## Usage

Add the crate to your dependencies with the required features. For example, to
enable JSON parsing, run:

```sh
cargo add markdown-frontmatter -F json
```

### Parsing Frontmatter

To parse the frontmatter, enable the corresponding feature (`json`, `toml`, or
`yaml`) and use the `parse` function:

```rust
use markdown_frontmatter::{parse, FrontmatterFormat};

#[derive(serde::Deserialize)]
struct MyFrontmatter {
    title: String,
}

let doc = r#"---
title: Hello
---
World
"#;

let result = parse::<MyFrontmatter>(doc).unwrap();
assert_eq!(result.format, Some(FrontmatterFormat::Yaml));
assert_eq!(result.frontmatter.unwrap().title, "Hello");
assert_eq!(result.body, "World\n");
```

### Splitting Frontmatter

If you only need to split the frontmatter from the body without deserializing
it, you can use the `split` function:

```rust
use markdown_frontmatter::{split, FrontmatterFormat};

let doc = r#"---
title: Hello
---
World
"#;

let result = split(doc).unwrap();
assert_eq!(result.format, Some(FrontmatterFormat::Yaml));
assert_eq!(result.frontmatter, Some("title: Hello\n"));
assert_eq!(result.body, "World\n");
```

## Features

This crate has the following Cargo features:

- `json`: Enables JSON frontmatter parsing.
- `toml`: Enables TOML frontmatter parsing.
- `yaml`: Enables YAML frontmatter parsing.

By default, no features are enabled.

## Contributing

Before submitting a pull request, please run the [.pre-commit.sh] script:

```sh
./.pre-commit.sh
```

## License

This project is licensed under the [MIT license][license].

[.pre-commit.sh]:
  https://github.com/imbolc/markdown-frontmatter/blob/main/.pre-commit.sh
[license]: https://github.com/imbolc/markdown-frontmatter/blob/main/LICENSE
