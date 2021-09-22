use druid::text::{Formatter, Selection, Validation, ValidationError};

// Formatter that sets the maximum length of the text.
pub struct MaxCharactersFormatter {
    max_chars: usize,
}

impl MaxCharactersFormatter {
    pub fn new(max_chars: usize) -> Self {
        Self { max_chars }
    }
}

impl Formatter<String> for MaxCharactersFormatter {
    fn format(&self, value: &String) -> String {
        value.to_owned()
    }

    fn value(&self, input: &str) -> Result<String, ValidationError> {
        Ok(input.to_owned())
    }

    fn validate_partial_input(&self, input: &str, _sel: &Selection) -> Validation {
        if input.chars().count() > self.max_chars {
            Validation::success().change_text(
                input[..input
                    .chars()
                    .map(|c| c.len_utf8())
                    .take(self.max_chars)
                    .sum()]
                    .to_string(),
            )
        } else {
            Validation::success()
        }
    }
}
