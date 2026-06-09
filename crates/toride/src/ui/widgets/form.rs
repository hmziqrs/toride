//! Form modal helper for multi-field input forms.
//!
//! Composes [`TextInput`] and [`Dropdown`] fields into a vertically stacked
//! form inside a [`Modal`] overlay. Manages field focus cycling (Tab / Shift+Tab),
//! validation on submit, error display, and themed rendering.

use crossterm::event::KeyCode;
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    text::Line,
    widgets::Paragraph,
};

use crate::ui::theme::Palette;

use super::{
    Dropdown, Modal, TextInput,
    text_input::InputAction,
    validate::Validator,
};

/// Result of a form interaction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FormResult {
    /// User submitted the form (Enter on last field or explicit submit) and
    /// all field validations passed.
    Submitted,
    /// User cancelled the form (Escape).
    Cancelled,
    /// Key consumed but form still active (typing, cursor movement, field
    /// cycling, or validation failed on submit).
    Pending,
}

// ── FormField ─────────────────────────────────────────────────────────────────

/// The underlying widget kind for a form field.
enum FieldKind {
    Text(TextInput),
    Select(Dropdown),
}

/// A single field in a form, with optional validation and error state.
pub struct FormField {
    kind: FieldKind,
    validators: Vec<Box<dyn Validator>>,
    /// Current validation error message, if any.
    error: Option<String>,
}

impl FormField {
    /// Create a text field wrapping the given [`TextInput`].
    fn text(input: TextInput) -> Self {
        let required = input.is_required();
        let mut field = Self {
            kind: FieldKind::Text(input),
            validators: Vec::new(),
            error: None,
        };
        if required {
            use super::validate::Required;
            field.validators.push(Box::new(Required));
        }
        field
    }

    /// Create a select field wrapping the given [`Dropdown`].
    fn select(dropdown: Dropdown) -> Self {
        let required = dropdown.is_required();
        let mut field = Self {
            kind: FieldKind::Select(dropdown),
            validators: Vec::new(),
            error: None,
        };
        if required {
            use super::validate::Required;
            field.validators.push(Box::new(Required));
        }
        field
    }

    /// Add a validator to this field.
    fn add_validator(&mut self, v: Box<dyn Validator>) {
        self.validators.push(v);
    }

    /// The label for this field.
    #[must_use]
    pub fn label(&self) -> &'static str {
        match &self.kind {
            FieldKind::Text(t) => t.label(),
            FieldKind::Select(d) => d.label(),
        }
    }

    /// Run all validators and return the first error message, if any.
    /// Updates `self.error` with the result.
    fn validate(&mut self) -> Option<&str> {
        let value = match &self.kind {
            FieldKind::Text(t) => t.get_value().to_string(),
            FieldKind::Select(d) => d.value().to_string(),
        };
        for v in &self.validators {
            if let Some(err) = v.validate(&value) {
                self.error = Some(err.message.clone());
                return Some(self.error.as_deref().unwrap_or(""));
            }
        }
        self.error = None;
        None
    }

    /// Clear the current error (e.g. when the user starts editing the field).
    fn clear_error(&mut self) {
        self.error = None;
    }

    /// Whether this field currently has a validation error.
    #[must_use]
    pub fn has_error(&self) -> bool {
        self.error.is_some()
    }
}

// ── FormModal ─────────────────────────────────────────────────────────────────

/// A form modal containing an ordered list of fields with focus management,
/// validation, and error display.
///
/// Construct with [`FormModal::new`], add fields with builder methods, then
/// call [`render`](Self::render) inside a modal content closure.
pub struct FormModal {
    fields: Vec<FormField>,
    /// Index of the currently focused field.
    focus: usize,
    /// Visual width available for the form content.
    width: u16,
}

impl FormModal {
    /// Create a new form with the given content width.
    #[must_use]
    pub fn new(width: u16) -> Self {
        Self {
            fields: Vec::new(),
            focus: 0,
            width,
        }
    }

    /// Add a text input field.
    #[must_use]
    pub fn text_field(mut self, input: TextInput) -> Self {
        self.fields.push(FormField::text(input));
        self
    }

    /// Add a text input field with an additional custom validator.
    pub fn text_field_validated(mut self, input: TextInput, validator: Box<dyn Validator>) -> Self {
        let mut field = FormField::text(input);
        field.add_validator(validator);
        self.fields.push(field);
        self
    }

    /// Add a dropdown selector field.
    #[must_use]
    pub fn select_field(mut self, dropdown: Dropdown) -> Self {
        self.fields.push(FormField::select(dropdown));
        self
    }

    /// Number of fields in the form.
    #[must_use]
    pub fn field_count(&self) -> usize {
        self.fields.len()
    }

    /// Get the value of a text field by index. Returns `None` if the field
    /// is not a text field or the index is out of bounds.
    #[must_use]
    pub fn text_value(&self, index: usize) -> Option<&str> {
        match self.fields.get(index)? {
            f => match &f.kind {
                FieldKind::Text(t) => Some(t.get_value()),
                FieldKind::Select(_) => None,
            },
        }
    }

    /// Get the selected option of a dropdown field by index. Returns `None`
    /// if the field is not a dropdown or the index is out of bounds.
    #[must_use]
    pub fn select_value(&self, index: usize) -> Option<&'static str> {
        match self.fields.get(index)? {
            f => match &f.kind {
                FieldKind::Text(_) => None,
                FieldKind::Select(d) => Some(d.value()),
            },
        }
    }

    /// Get the current focus index.
    #[must_use]
    pub fn focus(&self) -> usize {
        self.focus
    }

    /// Whether the form has any fields.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.fields.is_empty()
    }

    // ── Key handling ────────────────────────────────────────────────────────

    /// Handle a key event, routing it to the currently focused field.
    ///
    /// On submit (Enter on last field), runs validation on all fields. If any
    /// fail, sets error state, focuses the first invalid field, and returns
    /// `FormResult::Pending`. Otherwise returns `FormResult::Submitted`.
    pub fn handle_key(&mut self, code: KeyCode) -> FormResult {
        if self.fields.is_empty() {
            if matches!(code, KeyCode::Esc) {
                return FormResult::Cancelled;
            }
            return FormResult::Submitted;
        }

        // Clear error on the current field when the user interacts with it.
        self.fields[self.focus].clear_error();

        let action = match &mut self.fields[self.focus].kind {
            FieldKind::Text(t) => t.handle_key(code),
            FieldKind::Select(d) => d.handle_key(code),
        };

        match action {
            InputAction::Cancel => FormResult::Cancelled,
            InputAction::Submit => {
                // Enter on last field → validate all; otherwise move to next.
                if self.focus == self.fields.len() - 1 {
                    if self.validate_all() {
                        FormResult::Submitted
                    } else {
                        FormResult::Pending
                    }
                } else {
                    self.focus = (self.focus + 1) % self.fields.len();
                    FormResult::Pending
                }
            }
            InputAction::NextField => {
                self.focus = (self.focus + 1) % self.fields.len();
                FormResult::Pending
            }
            InputAction::PrevField => {
                if self.focus == 0 {
                    self.focus = self.fields.len() - 1;
                } else {
                    self.focus -= 1;
                }
                FormResult::Pending
            }
            InputAction::None => FormResult::Pending,
        }
    }

    /// Handle a key event but never submit — only cycle fields.
    /// Returns `true` if the key was consumed.
    pub fn handle_key_cycle(&mut self, code: KeyCode) -> bool {
        if self.fields.is_empty() {
            return false;
        }

        let action = match &mut self.fields[self.focus].kind {
            FieldKind::Text(t) => t.handle_key(code),
            FieldKind::Select(d) => d.handle_key(code),
        };

        match action {
            InputAction::NextField => {
                self.focus = (self.focus + 1) % self.fields.len();
                true
            }
            InputAction::PrevField => {
                if self.focus == 0 {
                    self.focus = self.fields.len() - 1;
                } else {
                    self.focus -= 1;
                }
                true
            }
            InputAction::Cancel => true,
            InputAction::Submit => true,
            InputAction::None => true,
        }
    }

    /// Validate all fields. Returns `true` if all pass.
    /// On failure, focuses the first invalid field.
    fn validate_all(&mut self) -> bool {
        let mut all_valid = true;
        let mut first_invalid = None;

        for (i, field) in self.fields.iter_mut().enumerate() {
            if field.validate().is_some() {
                if first_invalid.is_none() {
                    first_invalid = Some(i);
                }
                all_valid = false;
            }
        }

        if let Some(idx) = first_invalid {
            self.focus = idx;
        }

        all_valid
    }

    // ── Rendering ───────────────────────────────────────────────────────────

    /// Render all form fields vertically within the given area.
    ///
    /// Each field gets a 3-row-tall slot (border + content + border), plus an
    /// extra 1-row error line if the field has a validation error, with a
    /// 1-row gap between fields.
    pub fn render(&mut self, frame: &mut Frame, area: Rect, p: Palette) {
        if self.fields.is_empty() {
            return;
        }

        let field_h: u16 = 3; // border + content + border
        let error_h: u16 = 1; // error text below field
        let gap: u16 = 1;

        // Build dynamic constraints: each field is 3 rows, plus 1 if it has
        // an error, plus 1-row gap between fields.
        let constraints: Vec<Constraint> = (0..self.fields.len())
            .flat_map(|i| {
                let field_rows = if self.fields[i].has_error() {
                    field_h + error_h
                } else {
                    field_h
                };
                let mut cs = vec![Constraint::Length(field_rows)];
                if i < self.fields.len() - 1 {
                    cs.push(Constraint::Length(gap));
                }
                cs
            })
            .collect();

        let rects = Layout::vertical(constraints).split(area);

        let mut rect_idx = 0;
        for (i, field) in self.fields.iter_mut().enumerate() {
            let field_area = rects[rect_idx];
            let focused = i == self.focus;
            let error_msg = field.error.clone();

            match &field.kind {
                FieldKind::Text(t) => {
                    t.render(frame, field_area, p, focused, error_msg.as_deref());
                }
                FieldKind::Select(d) => {
                    d.render(frame, field_area, p, focused, error_msg.as_deref());
                }
            }

            // Render error row if present.
            if let Some(ref err) = error_msg {
                let error_y = field_area.y + field_h; // below the 3-row box
                if error_y < field_area.bottom() {
                    let error_area = Rect::new(field_area.x, error_y, field_area.width, error_h);
                    TextInput::render_error(frame, error_area, p, err);
                }
            }

            rect_idx += 2; // skip field chunk + gap
        }
    }

    /// Render the form inside a [`Modal`] with the given title and dimensions.
    pub fn render_in_modal(
        &mut self,
        frame: &mut Frame,
        p: Palette,
        title: &'static str,
        modal_w: u16,
        modal_h: u16,
    ) {
        Modal::new(title)
            .dimensions(modal_w, modal_h)
            .render(frame, p, |frame, content_area| {
                self.render(frame, content_area, p);
            });
    }

    /// Render the form inside a [`Modal`] with an additional instruction line
    /// above the fields.
    pub fn render_in_modal_with_hint(
        &mut self,
        frame: &mut Frame,
        p: Palette,
        title: &'static str,
        modal_w: u16,
        modal_h: u16,
        hint: &str,
    ) {
        Modal::new(title)
            .dimensions(modal_w, modal_h)
            .render(frame, p, |frame, content_area| {
                let [hint_area, _, form_area] = Layout::vertical([
                    Constraint::Length(1),
                    Constraint::Length(1),
                    Constraint::Min(0),
                ])
                .areas(content_area);

                frame.render_widget(
                    Paragraph::new(Line::from(hint).centered()),
                    hint_area,
                );

                self.render(frame, form_area, p);
            });
    }

    /// Calculate the total form height needed (fields + error rows + gaps).
    #[must_use]
    pub fn total_height(&self) -> u16 {
        if self.fields.is_empty() {
            return 0;
        }
        let field_h: u16 = 3;
        let error_h: u16 = 1;
        let gap: u16 = 1;
        let n = self.fields.len() as u16;
        let error_count = self.fields.iter().filter(|f| f.has_error()).count() as u16;
        n * field_h + error_count * error_h + n.saturating_sub(1) * gap
    }

    /// Calculate the maximum possible form height (all fields with errors).
    #[must_use]
    pub fn max_height(&self) -> u16 {
        if self.fields.is_empty() {
            return 0;
        }
        let field_h: u16 = 3;
        let error_h: u16 = 1;
        let gap: u16 = 1;
        let n = self.fields.len() as u16;
        n * (field_h + error_h) + n.saturating_sub(1) * gap
    }

    /// Reset focus to the first field (does NOT reset field values or errors).
    pub fn reset(&mut self) {
        self.focus = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::validate::{Required, MinLength, Port};

    fn sample_form() -> FormModal {
        FormModal::new(40)
            .text_field(TextInput::new("Name", 30).required())
            .select_field(Dropdown::new("Type", vec!["Ed25519", "RSA 4096"], 20))
            .text_field(TextInput::new("Comment", 30).placeholder("user@host"))
    }

    #[test]
    fn new_form_is_empty() {
        let form = FormModal::new(40);
        assert!(form.is_empty());
        assert_eq!(form.field_count(), 0);
    }

    #[test]
    fn sample_form_has_three_fields() {
        let form = sample_form();
        assert_eq!(form.field_count(), 3);
        assert!(!form.is_empty());
    }

    #[test]
    fn focus_starts_at_zero() {
        let form = sample_form();
        assert_eq!(form.focus(), 0);
    }

    #[test]
    fn text_value_returns_value() {
        let mut form = FormModal::new(40).text_field(
            TextInput::new("Name", 30).value("id_new"),
        );
        assert_eq!(form.text_value(0), Some("id_new"));
    }

    #[test]
    fn text_value_returns_none_for_dropdown() {
        let form = sample_form();
        assert_eq!(form.text_value(1), None);
    }

    #[test]
    fn select_value_returns_current() {
        let form = sample_form();
        assert_eq!(form.select_value(1), Some("Ed25519"));
    }

    #[test]
    fn select_value_returns_none_for_text() {
        let form = sample_form();
        assert_eq!(form.select_value(0), None);
    }

    #[test]
    fn out_of_bounds_returns_none() {
        let form = sample_form();
        assert_eq!(form.text_value(99), None);
        assert_eq!(form.select_value(99), None);
    }

    #[test]
    fn tab_cycles_forward() {
        let mut form = sample_form();
        form.handle_key(KeyCode::Tab);
        assert_eq!(form.focus(), 1);
        form.handle_key(KeyCode::Tab);
        assert_eq!(form.focus(), 2);
        form.handle_key(KeyCode::Tab);
        assert_eq!(form.focus(), 0); // wraps
    }

    #[test]
    fn backtab_cycles_backward() {
        let mut form = sample_form();
        form.handle_key(KeyCode::BackTab);
        assert_eq!(form.focus(), 2); // wraps to last
        form.handle_key(KeyCode::BackTab);
        assert_eq!(form.focus(), 1);
    }

    #[test]
    fn enter_on_last_field_validates() {
        // Use a form where all fields are filled / not required
        let mut form = FormModal::new(40)
            .text_field(TextInput::new("Name", 30).required().value("my-key"))
            .text_field(TextInput::new("Comment", 30).placeholder("optional"));
        // Focus last field and submit — Name has a value, should pass validation
        form.focus = 1;
        let result = form.handle_key(KeyCode::Enter);
        assert_eq!(result, FormResult::Submitted);
    }

    #[test]
    fn enter_on_last_field_fails_validation_when_required_empty() {
        let mut form = sample_form();
        // Name (field 0) is required but empty. Navigate to last field and submit.
        form.focus = 2;
        let result = form.handle_key(KeyCode::Enter);
        // Validation runs on ALL fields. Name is empty → fails.
        assert_eq!(result, FormResult::Pending);
        // Focus should jump to the first invalid field (Name = index 0)
        assert_eq!(form.focus(), 0);
        assert!(form.fields[0].has_error());
    }

    #[test]
    fn enter_on_non_last_field_moves_to_next() {
        let mut form = sample_form();
        form.focus = 0;
        let result = form.handle_key(KeyCode::Enter);
        assert_eq!(form.focus(), 1);
        assert_eq!(result, FormResult::Pending);
    }

    #[test]
    fn esc_cancels() {
        let mut form = sample_form();
        let result = form.handle_key(KeyCode::Esc);
        assert_eq!(result, FormResult::Cancelled);
    }

    #[test]
    fn typing_in_text_field() {
        let mut form = FormModal::new(40).text_field(TextInput::new("Name", 30));
        assert_eq!(form.handle_key(KeyCode::Char('a')), FormResult::Pending);
        assert_eq!(form.handle_key(KeyCode::Char('b')), FormResult::Pending);
        assert_eq!(form.text_value(0), Some("ab"));
    }

    #[test]
    fn dropdown_cycling_in_form() {
        let mut form = sample_form();
        form.focus = 1; // Focus the dropdown
        form.handle_key(KeyCode::Down);
        assert_eq!(form.select_value(1), Some("RSA 4096"));
    }

    #[test]
    fn total_height_calculation() {
        let form = sample_form(); // 3 fields, no errors
        // 3 * 3 (field heights) + 2 * 1 (gaps) = 11
        assert_eq!(form.total_height(), 11);
    }

    #[test]
    fn total_height_empty() {
        let form = FormModal::new(40);
        assert_eq!(form.total_height(), 0);
    }

    #[test]
    fn validation_with_custom_validator() {
        let mut form = FormModal::new(40)
            .text_field_validated(
                TextInput::new("Port", 10).required(),
                Box::new(Port),
            );
        // Type an invalid port
        for ch in "abc".chars() {
            form.handle_key(KeyCode::Char(ch));
        }
        // Navigate to last field and submit
        form.focus = 0;
        let result = form.handle_key(KeyCode::Enter);
        // "abc" is not a valid port → validation should fail
        assert_eq!(result, FormResult::Pending);
        assert!(form.fields[0].has_error());
    }

    #[test]
    fn validation_passes_with_valid_data() {
        let mut form = FormModal::new(40)
            .text_field(TextInput::new("Name", 30).required().value("my-key"));
        // Submit (only 1 field, so we're on last)
        let result = form.handle_key(KeyCode::Enter);
        assert_eq!(result, FormResult::Submitted);
    }

    #[test]
    fn error_clears_on_typing() {
        let mut form = sample_form();
        // Force an error by submitting with empty Name
        form.focus = 2;
        form.handle_key(KeyCode::Enter);
        assert!(form.fields[0].has_error());

        // Now focus the Name field and type
        form.focus = 0;
        form.handle_key(KeyCode::Char('a'));
        assert!(!form.fields[0].has_error());
    }
}
