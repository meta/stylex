//! Structured native compiler diagnostics. Rendering code frames belongs to the host.
use serde::{Deserialize, Serialize};
use std::fmt;
use swc_core::common::Span;

#[cfg(any(test, feature = "node"))]
pub(crate) const ENVELOPE_PREFIX: &str = "STYLEX_DIAGNOSTIC:";
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiagnosticRange {
    /// UTF-8 offsets in the exact input source, excluding SWC's one-based file origin.
    pub start: usize,
    pub end: usize,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Diagnostic {
    pub name: String,
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub range: Option<DiagnosticRange>,
}
impl Diagnostic {
    pub fn syntax(message: impl Into<String>, span: Span) -> Self {
        Self {
            name: "SyntaxError".to_owned(),
            message: message.into(),
            kind: None,
            range: (!span.is_dummy()).then(|| DiagnosticRange {
                start: span.lo.0.saturating_sub(1) as usize,
                end: span.hi.0.saturating_sub(1) as usize,
            }),
        }
    }
    pub fn plain(message: impl Into<String>) -> Self {
        Self {
            name: "Error".to_owned(),
            message: message.into(),
            kind: None,
            range: None,
        }
    }
    pub fn parse(message: impl Into<String>, range: DiagnosticRange) -> Self {
        Self {
            name: "SyntaxError".to_owned(),
            message: message.into(),
            kind: Some("parse".to_owned()),
            range: Some(range),
        }
    }
    pub fn into_error(self) -> anyhow::Error {
        anyhow::Error::new(DiagnosticError(self))
    }
    #[cfg(any(test, feature = "node"))]
    pub fn envelope(&self) -> String {
        format!(
            "{ENVELOPE_PREFIX}{}",
            serde_json::to_string(self).expect("serializable compiler diagnostic")
        )
    }
}
#[derive(Debug)]
pub struct DiagnosticError(pub Diagnostic);
impl fmt::Display for DiagnosticError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0.message)
    }
}
impl std::error::Error for DiagnosticError {}
#[cfg(test)]
pub(crate) fn syntax_error(message: impl Into<String>, span: Span) -> anyhow::Error {
    Diagnostic::syntax(message, span).into_error()
}
#[cfg(test)]
pub(crate) fn plain_error(message: impl Into<String>) -> anyhow::Error {
    Diagnostic::plain(message).into_error()
}

pub fn from_error(error: &anyhow::Error) -> Diagnostic {
    error
        .downcast_ref::<DiagnosticError>()
        .map(|error| error.0.clone())
        .unwrap_or_else(|| Diagnostic::plain(error.to_string()))
}

#[cfg(test)]
mod tests {
    use super::{from_error, Diagnostic, ENVELOPE_PREFIX};
    use swc_core::common::{BytePos, Span};

    #[test]
    fn native_diagnostic_envelope_preserves_range_and_error_kind() {
        let diagnostic =
            Diagnostic::syntax("invalid StyleX call", Span::new(BytePos(4), BytePos(12)));
        let error = diagnostic.clone().into_error();
        assert_eq!(from_error(&error), diagnostic);
        let envelope = from_error(&error).envelope();
        let decoded: Diagnostic =
            serde_json::from_str(envelope.strip_prefix(ENVELOPE_PREFIX).unwrap()).unwrap();
        assert_eq!(decoded, diagnostic);
        assert_eq!(decoded.range.unwrap().start, 3);
        assert_eq!(from_error(&anyhow::anyhow!("ordinary error")).name, "Error");
        let parsed = Diagnostic::parse(
            "invalid source",
            super::DiagnosticRange { start: 2, end: 3 },
        );
        let envelope = parsed.envelope();
        let decoded: Diagnostic =
            serde_json::from_str(envelope.strip_prefix(ENVELOPE_PREFIX).unwrap()).unwrap();
        assert_eq!(decoded.kind.as_deref(), Some("parse"));
        assert_eq!(decoded.name, "SyntaxError");
    }
}
