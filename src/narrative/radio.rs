use super::dialogue::{DialogueLine, Speaker};
use super::script::RadioChatterData;

pub struct RadioSystem {
    pub current_line: Option<ActiveRadioLine>,
    pub pending_lines: Vec<(f64, DialogueLine)>, // (trigger_time, line)
    pub elapsed_proper_time: f64,
    pub initial_delay: f64, // no chatter for first N seconds
}

pub struct ActiveRadioLine {
    pub line: DialogueLine,
    pub chars_revealed: usize,
    pub time_since_start: f64,
    pub linger_timer: f64,
    pub finished_typing: bool,
}

impl RadioSystem {
    pub fn new() -> Self {
        Self {
            current_line: None,
            pending_lines: Vec::new(),
            elapsed_proper_time: 0.0,
            initial_delay: 5.0,
        }
    }

    /// Queue lines from a RadioChatterData payload.
    /// The delay values in the data are absolute proper-time offsets from
    /// the moment the chatter is loaded, so we add elapsed_proper_time
    /// to convert them into absolute trigger times.
    pub fn load_chatter(&mut self, data: RadioChatterData) {
        let base = self.elapsed_proper_time;
        for (delay, line) in data.lines {
            let trigger_time = base + delay;
            self.pending_lines.push((trigger_time, line));
        }
        // Sort by trigger time so earliest fires first.
        self.pending_lines.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
    }

    /// Advance the radio system by dt proper-time seconds.
    pub fn update(&mut self, dt_proper: f64) {
        self.elapsed_proper_time += dt_proper;

        // Don't start any chatter during the initial delay.
        if self.elapsed_proper_time < self.initial_delay {
            return;
        }

        // Update the active line if one is displaying.
        if let Some(ref mut active) = self.current_line {
            active.time_since_start += dt_proper;

            if !active.finished_typing {
                let speed = active.line.typing_speed;
                let target_chars = (active.time_since_start * speed) as usize;
                let total_chars = active.line.text.len();
                active.chars_revealed = target_chars.min(total_chars);

                if active.chars_revealed >= total_chars {
                    active.finished_typing = true;
                    active.linger_timer = 0.0;
                }
            } else {
                active.linger_timer += dt_proper;
                let linger_duration = Self::linger_duration(&active.line);
                if active.linger_timer >= linger_duration {
                    // Line is done lingering, remove it.
                    self.current_line = None;
                }
            }
        }

        // If no active line, try to pull the next pending line whose trigger
        // time has been reached.
        if self.current_line.is_none() {
            if let Some(idx) = self.pending_lines.iter().position(|(t, _)| *t <= self.elapsed_proper_time) {
                let (_, line) = self.pending_lines.remove(idx);
                self.current_line = Some(ActiveRadioLine {
                    line,
                    chars_revealed: 0,
                    time_since_start: 0.0,
                    linger_timer: 0.0,
                    finished_typing: false,
                });
            }
        }
    }

    /// Returns the visible portion of the current line for rendering.
    pub fn get_display_text(&self) -> Option<(Speaker, &str)> {
        self.current_line.as_ref().map(|active| {
            let visible = &active.line.text[..byte_index_for_chars(&active.line.text, active.chars_revealed)];
            (active.line.speaker, visible)
        })
    }

    /// Compute linger duration: max(3.0, text_length / typing_speed + 2.0)
    fn linger_duration(line: &DialogueLine) -> f64 {
        let reading_time = line.text.len() as f64 / line.typing_speed + 2.0;
        reading_time.max(3.0)
    }
}

/// Find the byte index corresponding to `n` chars in a UTF-8 string.
fn byte_index_for_chars(s: &str, n: usize) -> usize {
    s.char_indices()
        .nth(n)
        .map(|(i, _)| i)
        .unwrap_or(s.len())
}
