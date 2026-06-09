//! Form modal helper for multi-field input forms.
//!
//! Composes [`TextInput`] and [`Dropdown`] fields into a vertically stacked
//! form inside a [`Modal`] overlay. Manages field focus cycling (Tab / Shift+Tab),
//! submit / cancel handling, and themed rendering.

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
};

/// Result of a form interaction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FormResult {
    /// User submitted the form (Enter on last field or explicit submit).
    Submitted,
    /// User cancelled the form (Escape).
    Cancelled,
    /// Key consumed but form still active (typing, cursor movement, field cycling).
    Pending,
}

/// A single field in a form.
pub enum FormField {
    /// A text input field.
    Text(TextInput),
    /// A dropdown selector field.
    Select(Dropdown),
}

impl FormField {
    /// The label for this field.
    #[must_use]
    pub fn label(&self) -> &'static str {
        match self {
            FormField::Text(t) => t.label(),
            FormField::Select(d) => d.label(),
        }
    }
}

/// A form modal containing an ordered list of fields with focus management.
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
        self.fields.push(FormField::Text(input));
        self
    }

    /// Add a dropdown selector field.
    #[must_use]
    pub fn select_field(mut self, dropdown: Dropdown) -> Self {
        self.fields.push(FormField::Select(dropdown));
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
            FormField::Text(t) => Some(t.get_value()),
            FormField::Select(_) => None,
        }
    }

    /// Get the selected option of a dropdown field by index. Returns `None`
    /// if the field is not a dropdown or the index is out of bounds.
    #[must_use]
    pub fn select_value(&self, index: usize) -> Option<&'static str> {
        match self.fields.get(index)? {
            FormField::Text(_) => None,
            FormField::Select(d) => Some(d.value()),
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
    pub fn handle_key(&mut self, code: KeyCode) -> FormResult {
        if self.fields.is_empty() {
            if matches!(code, KeyCode::Esc) {
                return FormResult::Cancelled;
            }
            return FormResult::Submitted;
        }

        let action = match &mut self.fields[self.focus] {
            FormField::Text(t) => t.handle_key(code),
            FormField::Select(d) => d.handle_key(code),
        };

        match action {
            InputAction::Cancel => FormResult::Cancelled,
            InputAction::Submit => {
                // Enter on last field submits; otherwise moves to next field.
                if self.focus == self.fields.len() - 1 {
                    FormResult::Submitted
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

        let action = match &mut self.fields[self.focus] {
            FormField::Text(t) => t.handle_key(code),
            FormField::Select(d) => d.handle_key(code),
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

    // ── Rendering ───────────────────────────────────────────────────────────

    /// Render all form fields vertically within the given area.
    ///
    /// Each field gets a 3-row-tall slot (border + content + border) with a
    /// 1-row gap between fields.
    pub fn render(&mut self, frame: &mut Frame, area: Rect, p: Palette) {
        if self.fields.is_empty() {
            return;
        }

        let field_h: u16 = 3; // border + content + border
        let gap: u16 = 1;
        let n = self.fields.len();

        let constraints: Vec<Constraint> = (0..n)
            .flat_map(|i| {
                let mut cs = vec![Constraint::Length(field_h)];
                if i < n - 1 {
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

            match field {
                FormField::Text(t) => t.render(frame, field_area, p, focused),
                FormField::Select(d) => d.render(frame, field_area, p, focused),
            }

            rect_idx += 2; // skip field + gap
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

    /// Calculate the total form height needed (fields + gaps).
    #[must_use]
    pub fn total_height(&self) -> u16 {
        if self.fields.is_empty() {
            return 0;
        }
        let field_h: u16 = 3;
        let gap: u16 = 1;
        (self.fields.len() as u16) * field_h + (self.fields.len().saturating_sub(1) as u16) * gap
    }

    /// Reset all field values and focus.
    pub fn reset(&mut self) {
        // Re-create empty fields with same labels and widths
        self.focus = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_form() -> FormModal {
        FormModal::new(40)
            .text_field(TextInput::new("Name", 30))
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
    fn enter_on_last_field_submits() {
        let mut form = sample_form();
        // Navigate to last field
        form.focus = 2;
        let result = form.handle_key(KeyCode::Enter);
        assert_eq!(result, FormResult::Submitted);
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
        let form = sample_form(); // 3 fields
        // 3 * 3 (field heights) + 2 * 1 (gaps) = 11
        assert_eq!(form.total_height(), 11);
    }

    #[test]
    fn total_height_empty() {
        let form = FormModal::new(40);
        assert_eq!(form.total_height(), 0);
    }
}
