use super::dialogue::{DialogueLine, Speaker};

pub struct BriefingState {
    pub lines: Vec<DialogueLine>,
    pub current_line_index: usize,
    pub chars_revealed: usize,
    pub typing_timer: f64,
    pub finished: bool,
    pub fast_forward: bool,
    /// Grace period: ignore advance() calls for this many seconds after creation
    /// or after moving to a new line. Prevents the Enter key from the previous
    /// state transition from instantly revealing/skipping text.
    pub input_cooldown: f64,
}

impl BriefingState {
    pub fn new(lines: Vec<DialogueLine>) -> Self {
        let finished = lines.is_empty();
        Self {
            lines,
            current_line_index: 0,
            chars_revealed: 0,
            typing_timer: 0.0,
            finished,
            fast_forward: false,
            input_cooldown: 0.3, // ignore input for 300ms after creation
        }
    }

    /// Advance the typewriter effect by dt seconds.
    pub fn update(&mut self, dt: f64) {
        if self.finished {
            return;
        }

        // Tick down input cooldown
        if self.input_cooldown > 0.0 {
            self.input_cooldown -= dt;
        }

        if self.current_line_index >= self.lines.len() {
            self.finished = true;
            return;
        }

        let line = &self.lines[self.current_line_index];
        let speed = if self.fast_forward {
            line.typing_speed * 5.0
        } else {
            line.typing_speed
        };

        self.typing_timer += dt;
        let target_chars = (self.typing_timer * speed) as usize;
        let total_chars = line.text.chars().count();
        self.chars_revealed = target_chars.min(total_chars);
    }

    /// Advance to the next line. If the current line is still typing,
    /// complete it instantly. If it is fully revealed, move to the next.
    /// If all lines are done, mark finished.
    /// Returns false if input was ignored due to cooldown.
    pub fn advance(&mut self) -> bool {
        if self.finished {
            return false;
        }

        // Ignore input during cooldown (prevents Enter held from previous screen)
        if self.input_cooldown > 0.0 {
            return false;
        }

        if self.current_line_index >= self.lines.len() {
            self.finished = true;
            return true;
        }

        let total_chars = self.lines[self.current_line_index].text.chars().count();

        if self.chars_revealed < total_chars {
            // Reveal the rest of the current line instantly.
            self.chars_revealed = total_chars;
        } else {
            // Move to next line.
            self.current_line_index += 1;
            self.chars_revealed = 0;
            self.typing_timer = 0.0;
            self.input_cooldown = 0.15; // small cooldown between lines too

            if self.current_line_index >= self.lines.len() {
                self.finished = true;
            }
        }
        true
    }

    /// Enable or disable fast-forward (5x typing speed).
    pub fn set_fast_forward(&mut self, on: bool) {
        self.fast_forward = on;
    }

    /// Returns the speaker and visible portion of the current line.
    pub fn get_current_display(&self) -> Option<(Speaker, &str)> {
        if self.finished || self.current_line_index >= self.lines.len() {
            return None;
        }

        let line = &self.lines[self.current_line_index];
        let byte_idx = byte_index_for_chars(&line.text, self.chars_revealed);
        let visible = &line.text[..byte_idx];
        Some((line.speaker, visible))
    }

    /// Returns true if all lines have been displayed and dismissed.
    pub fn is_all_done(&self) -> bool {
        self.finished
    }
}

/// Find the byte index corresponding to `n` chars in a UTF-8 string.
fn byte_index_for_chars(s: &str, n: usize) -> usize {
    s.char_indices()
        .nth(n)
        .map(|(i, _)| i)
        .unwrap_or(s.len())
}
