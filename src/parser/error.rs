#[derive(Debug)]
pub struct ParseError {
    pub message: String,
    pub span: std::ops::Range<usize>,
}

impl ParseError {
    pub fn new(message: impl Into<String>, span: std::ops::Range<usize>) -> Self {
        Self {
            message: message.into(),
            span,
        }
    }
}
