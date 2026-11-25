//! Data-only loading and validation for declarative sbx2d game packs.
//!
//! The parser deliberately accepts a small Lua-shaped notation without using a
//! Lua VM, so loading a pack can never execute its contents.

use std::{
    fmt, fs,
    path::{Path, PathBuf},
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Diagnostic {
    pub code: &'static str,
    pub path: PathBuf,
    pub span: Option<Span>,
    pub field_path: Option<String>,
    pub message: String,
    pub help: String,
}
impl Diagnostic {
    fn new(
        path: &Path,
        span: Option<Span>,
        code: &'static str,
        message: impl Into<String>,
        help: impl Into<String>,
    ) -> Self {
        Self {
            code,
            path: path.into(),
            span,
            field_path: None,
            message: message.into(),
            help: help.into(),
        }
    }
    fn field(mut self, field: impl Into<String>) -> Self {
        self.field_path = Some(field.into());
        self
    }
    #[must_use]
    pub fn render(&self, source: Option<&str>) -> String {
        let location = match (self.span, source) {
            (Some(span), Some(source)) => {
                let before = &source[..span.start.min(source.len())];
                format!(
                    ":{}:{}",
                    before.bytes().filter(|byte| *byte == b'\n').count() + 1,
                    before
                        .rsplit('\n')
                        .next()
                        .map_or(1, |line| line.chars().count() + 1)
                )
            }
            _ => String::new(),
        };
        let field = self
            .field_path
            .as_deref()
            .map_or(String::new(), |field| format!(" ({field})"));
        format!(
            "{}{}: error[{}]{}: {}\n  help: {}",
            self.path.display(),
            location,
            self.code,
            field,
            self.message,
            self.help
        )
    }
}
impl fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.code, self.message)
    }
}
impl std::error::Error for Diagnostic {}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Manifest {
    pub pack_version: u32,
    pub name: String,
    pub start_scene: SceneReference,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SceneReference {
    pub namespace: String,
    pub name: String,
}
impl fmt::Display for SceneReference {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.namespace, self.name)
    }
}

/// Loads `game.lua` from a pack directory, or parses the supplied game.lua path.
pub fn load_manifest(input: impl AsRef<Path>) -> Result<Manifest, Diagnostic> {
    let input = input.as_ref();
    let path = if input.is_dir() {
        input.join("game.lua")
    } else {
        input.into()
    };
    let source = fs::read_to_string(&path).map_err(|error| {
        Diagnostic::new(
            &path,
            None,
            "SBX001",
            format!("could not read manifest: {error}"),
            "pass a pack directory containing game.lua or a game.lua path",
        )
    })?;
    parse_manifest(path, &source)
}
/// Parses and validates a manifest source string. Useful to editor integrations.
pub fn parse_manifest(path: impl AsRef<Path>, source: &str) -> Result<Manifest, Diagnostic> {
    let path = path.as_ref();
    let table = Parser::new(path, source).parse()?;
    validate(path, table)
}

#[derive(Clone, Debug)]
enum Value {
    String(String, Span),
    Integer(i64, Span),
    Boolean(Span),
    Nil(Span),
    Table(Span),
}
impl Value {
    fn span(&self) -> Span {
        match self {
            Self::String(_, span)
            | Self::Integer(_, span)
            | Self::Boolean(span)
            | Self::Nil(span)
            | Self::Table(span) => *span,
        }
    }
    fn name(&self) -> &'static str {
        match self {
            Self::String(..) => "a string",
            Self::Integer(..) => "an integer",
            Self::Boolean(..) => "a boolean",
            Self::Nil(..) => "nil",
            Self::Table(..) => "a table",
        }
    }
}
#[derive(Clone, Debug)]
struct Table {
    fields: Vec<(String, Value)>,
    span: Span,
}

fn validate(path: &Path, table: Table) -> Result<Manifest, Diagnostic> {
    let mut version = None;
    let mut name = None;
    let mut scene = None;
    for (key, value) in table.fields {
        let slot = match key.as_str() {
            "pack_version" => &mut version,
            "name" => &mut name,
            "start_scene" => &mut scene,
            _ => {
                return Err(Diagnostic::new(
                    path,
                    Some(value.span()),
                    "SBX105",
                    format!("unknown manifest field `{key}`"),
                    "use only `pack_version`, `name`, and `start_scene` in format version 1",
                )
                .field(key));
            }
        };
        if slot.is_some() {
            return Err(Diagnostic::new(
                path,
                Some(value.span()),
                "SBX104",
                format!("duplicate field `{key}`"),
                "remove the duplicate field",
            )
            .field(key));
        }
        *slot = Some(value);
    }
    let version = required(path, table.span, "pack_version", version)?;
    let Value::Integer(version, span) = version else {
        return Err(expected(
            path,
            version,
            "pack_version",
            "a positive integer",
        ));
    };
    if !(1..=i64::from(u32::MAX)).contains(&version) {
        return Err(Diagnostic::new(
            path,
            Some(span),
            "SBX102",
            "expected a positive integer",
            "change `pack_version` to `1`",
        )
        .field("pack_version"));
    }
    if version != 1 {
        return Err(Diagnostic::new(
            path,
            Some(span),
            "SBX106",
            format!("unsupported pack format version {version}"),
            "use `pack_version = 1`",
        )
        .field("pack_version"));
    }
    let name = required(path, table.span, "name", name)?;
    let Value::String(name, name_span) = name else {
        return Err(expected(path, name, "name", "a string identifier"));
    };
    identifier(path, name_span, "name", &name)?;
    let scene = required(path, table.span, "start_scene", scene)?;
    let Value::String(scene, scene_span) = scene else {
        return Err(expected(
            path,
            scene,
            "start_scene",
            "a qualified scene reference such as `forest:entrance`",
        ));
    };
    let start_scene = scene_reference(path, scene_span, &scene)?;
    Ok(Manifest {
        pack_version: u32::try_from(version).expect("range was validated above"),
        name,
        start_scene,
    })
}
fn required(
    path: &Path,
    span: Span,
    field: &str,
    value: Option<Value>,
) -> Result<Value, Diagnostic> {
    value.ok_or_else(|| {
        Diagnostic::new(
            path,
            Some(span),
            "SBX103",
            format!("missing required field `{field}`"),
            format!("add `{field}` to the returned manifest table"),
        )
        .field(field)
    })
}
fn expected(path: &Path, value: Value, field: &str, expected: &str) -> Diagnostic {
    Diagnostic::new(
        path,
        Some(value.span()),
        "SBX102",
        format!("expected {expected}, received {}", value.name()),
        format!("change `{field}` to {expected}"),
    )
    .field(field)
}
fn identifier(path: &Path, span: Span, field: &str, value: &str) -> Result<(), Diagnostic> {
    let valid = !value.is_empty()
        && value.bytes().enumerate().all(|(index, byte)| match byte {
            b'a'..=b'z' => true,
            b'0'..=b'9' | b'-' | b'_' => index > 0,
            _ => false,
        });
    if valid {
        Ok(())
    } else {
        Err(Diagnostic::new(
            path,
            Some(span),
            "SBX107",
            format!("`{value}` is not a valid identifier"),
            "start with lowercase ASCII letter; then use lowercase letters, digits, `-`, or `_`",
        )
        .field(field))
    }
}
fn scene_reference(path: &Path, span: Span, value: &str) -> Result<SceneReference, Diagnostic> {
    let Some((namespace, name)) = value.split_once(':') else {
        return Err(Diagnostic::new(
            path,
            Some(span),
            "SBX108",
            format!("`{value}` is not a qualified scene reference"),
            "use `namespace:name`, such as `forest:entrance`",
        )
        .field("start_scene"));
    };
    if value.matches(':').count() != 1 {
        return Err(Diagnostic::new(
            path,
            Some(span),
            "SBX108",
            format!("`{value}` is not a qualified scene reference"),
            "use exactly one colon: `namespace:name`",
        )
        .field("start_scene"));
    }
    identifier(path, span, "start_scene namespace", namespace)?;
    identifier(path, span, "start_scene name", name)?;
    Ok(SceneReference {
        namespace: namespace.into(),
        name: name.into(),
    })
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum TokenKind {
    Return,
    True,
    False,
    Nil,
    Identifier(String),
    String(String),
    Integer(i64),
    Left,
    Right,
    Equals,
    Comma,
    Semicolon,
    Eof,
}
#[derive(Clone, Debug)]
struct Token {
    kind: TokenKind,
    span: Span,
}
struct Parser<'a> {
    path: &'a Path,
    source: &'a str,
    cursor: usize,
    current: Token,
}
impl<'a> Parser<'a> {
    fn new(path: &'a Path, source: &'a str) -> Self {
        let mut parser = Self {
            path,
            source,
            cursor: 0,
            current: Token {
                kind: TokenKind::Eof,
                span: Span { start: 0, end: 0 },
            },
        };
        parser.bump().expect("initial lexing cannot fail");
        parser
    }
    fn parse(mut self) -> Result<Table, Diagnostic> {
        self.expect(
            TokenKind::Return,
            "`return`",
            "a pack must start with `return { ... }`",
        )?;
        let table = self.table()?;
        if self.current.kind != TokenKind::Eof {
            return Err(self.error(
                "unexpected content after returned table",
                "a pack may contain only one `return { ... }` data value",
            ));
        }
        Ok(table)
    }
    fn table(&mut self) -> Result<Table, Diagnostic> {
        let start = self.current.span.start;
        self.expect(TokenKind::Left, "`{`", "start the returned data with `{`")?;
        let mut fields = Vec::new();
        while self.current.kind != TokenKind::Right {
            if self.current.kind == TokenKind::Eof {
                return Err(self.error("unterminated table", "add a closing `}`"));
            }
            let token = self.current.clone();
            let TokenKind::Identifier(key) = token.kind else {
                return Err(self.error("expected a named table field", "use `field_name = value`; computed keys and arrays are not supported in the v1 manifest"));
            };
            self.bump()?;
            self.expect(
                TokenKind::Equals,
                "`=`",
                "separate a field name and value with `=`",
            )?;
            fields.push((key, self.value()?));
            if matches!(self.current.kind, TokenKind::Comma | TokenKind::Semicolon) {
                self.bump()?;
            } else if self.current.kind != TokenKind::Right {
                return Err(self.error(
                    "expected `,` or `}` after table field",
                    "separate table fields with a comma",
                ));
            }
        }
        let end = self.current.span.end;
        self.bump()?;
        Ok(Table {
            fields,
            span: Span { start, end },
        })
    }
    fn value(&mut self) -> Result<Value, Diagnostic> {
        let token = self.current.clone();
        let value = match token.kind { TokenKind::String(text) => Value::String(text, token.span), TokenKind::Integer(number) => Value::Integer(number, token.span), TokenKind::True | TokenKind::False => Value::Boolean(token.span), TokenKind::Nil => Value::Nil(token.span), TokenKind::Left => { let table = self.table()?; return Ok(Value::Table(table.span)); }, _ => return Err(self.error("expected a data literal", "only tables, strings, integers, booleans, and nil are allowed; functions, expressions, and calls are rejected")), };
        self.bump()?;
        Ok(value)
    }
    fn expect(&mut self, kind: TokenKind, expected: &str, help: &str) -> Result<(), Diagnostic> {
        if self.current.kind == kind {
            self.bump()
        } else {
            Err(self.error(&format!("expected {expected}"), help))
        }
    }
    fn error(&self, message: &str, help: &str) -> Diagnostic {
        Diagnostic::new(self.path, Some(self.current.span), "SBX101", message, help)
    }
    fn bump(&mut self) -> Result<(), Diagnostic> {
        self.current = self.lex()?;
        Ok(())
    }
    fn lex(&mut self) -> Result<Token, Diagnostic> {
        self.skip();
        let start = self.cursor;
        let Some(byte) = self.peek() else {
            return Ok(Token {
                kind: TokenKind::Eof,
                span: Span { start, end: start },
            });
        };
        let kind = match byte {
            b'{' => {
                self.cursor += 1;
                TokenKind::Left
            }
            b'}' => {
                self.cursor += 1;
                TokenKind::Right
            }
            b'=' => {
                self.cursor += 1;
                TokenKind::Equals
            }
            b',' => {
                self.cursor += 1;
                TokenKind::Comma
            }
            b';' => {
                self.cursor += 1;
                TokenKind::Semicolon
            }
            b'\'' | b'\"' => self.string()?,
            b'0'..=b'9' | b'-' => self.integer()?,
            b'a'..=b'z' | b'A'..=b'Z' | b'_' => self.word(),
            _ => {
                return Err(Diagnostic::new(
                    self.path,
                    Some(Span {
                        start,
                        end: start + 1,
                    }),
                    "SBX101",
                    format!("unsupported Lua character `{}`", byte as char),
                    "only restricted data literals are allowed",
                ));
            }
        };
        Ok(Token {
            kind,
            span: Span {
                start,
                end: self.cursor,
            },
        })
    }
    fn skip(&mut self) {
        loop {
            while self.peek().is_some_and(|byte| byte.is_ascii_whitespace()) {
                self.cursor += 1;
            }
            if self.source.as_bytes().get(self.cursor..self.cursor + 2) == Some(b"--") {
                self.cursor += 2;
                while self.peek().is_some_and(|byte| byte != b'\n') {
                    self.cursor += 1;
                }
            } else {
                return;
            }
        }
    }
    fn string(&mut self) -> Result<TokenKind, Diagnostic> {
        let quote = self.peek().expect("quote");
        let start = self.cursor;
        self.cursor += 1;
        let mut text = String::new();
        while let Some(byte) = self.peek() {
            self.cursor += 1;
            if byte == quote {
                return Ok(TokenKind::String(text));
            }
            if byte == b'\\' {
                let Some(escaped) = self.peek() else { break };
                self.cursor += 1;
                text.push(match escaped {
                    b'n' => '\n',
                    b'r' => '\r',
                    b't' => '\t',
                    b'\\' => '\\',
                    b'\'' => '\'',
                    b'\"' => '\"',
                    _ => {
                        return Err(Diagnostic::new(
                            self.path,
                            Some(Span {
                                start: self.cursor - 2,
                                end: self.cursor,
                            }),
                            "SBX101",
                            "unsupported string escape",
                            "use standard simple escapes",
                        ));
                    }
                });
            } else {
                text.push(byte as char);
            }
        }
        Err(Diagnostic::new(
            self.path,
            Some(Span {
                start,
                end: self.cursor,
            }),
            "SBX101",
            "unterminated string",
            "close the string with its opening quote",
        ))
    }
    fn integer(&mut self) -> Result<TokenKind, Diagnostic> {
        let start = self.cursor;
        if self.peek() == Some(b'-') {
            self.cursor += 1;
        }
        let digits = self.cursor;
        while self.peek().is_some_and(|byte| byte.is_ascii_digit()) {
            self.cursor += 1;
        }
        if digits == self.cursor {
            return Err(Diagnostic::new(
                self.path,
                Some(Span {
                    start,
                    end: self.cursor,
                }),
                "SBX101",
                "expected digits after `-`",
                "use an integer data literal",
            ));
        }
        self.source[start..self.cursor]
            .parse()
            .map(TokenKind::Integer)
            .map_err(|_| {
                Diagnostic::new(
                    self.path,
                    Some(Span {
                        start,
                        end: self.cursor,
                    }),
                    "SBX101",
                    "integer is out of range",
                    "use a 64-bit signed integer",
                )
            })
    }
    fn word(&mut self) -> TokenKind {
        let start = self.cursor;
        while self
            .peek()
            .is_some_and(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
        {
            self.cursor += 1;
        }
        match &self.source[start..self.cursor] {
            "return" => TokenKind::Return,
            "true" => TokenKind::True,
            "false" => TokenKind::False,
            "nil" => TokenKind::Nil,
            text => TokenKind::Identifier(text.into()),
        }
    }
    fn peek(&self) -> Option<u8> {
        self.source.as_bytes().get(self.cursor).copied()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn parse(source: &str) -> Result<Manifest, Diagnostic> {
        parse_manifest("game.lua", source)
    }
    const VALID: &str =
        "return { pack_version = 1, name = 'tiny-game', start_scene = 'forest:entrance' }";
    #[test]
    fn accepts_manifest() {
        assert_eq!(
            parse(VALID).unwrap().start_scene.to_string(),
            "forest:entrance"
        );
    }
    #[test]
    fn reports_missing_field() {
        let error = parse("return { pack_version = 1, name = 'tiny-game' }").unwrap_err();
        assert_eq!(error.code, "SBX103");
        assert_eq!(error.field_path.as_deref(), Some("start_scene"));
    }
    #[test]
    fn rejects_function() {
        let error =
            parse("return { pack_version = 1, name = 'tiny-game', start_scene = function() end }")
                .unwrap_err();
        assert_eq!(error.code, "SBX101");
    }
    #[test]
    fn rejects_call() {
        assert_eq!(parse("return { pack_version = 1, name = 'tiny-game', start_scene = 'forest:entrance' }; os.execute('bad')").unwrap_err().code, "SBX101");
    }
    #[test]
    fn reports_location() {
        assert!(
            parse("return { name = 'tiny-game' }")
                .unwrap_err()
                .render(Some("return { name = 'tiny-game' }"))
                .contains("game.lua:1:")
        );
    }
}
