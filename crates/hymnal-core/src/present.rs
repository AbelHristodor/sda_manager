//! Pure presentation state for the Control tab: which hymn is loaded, which
//! slide is current, and whether output is blanked. No I/O, no UI — fully
//! unit-testable. The GUI mirrors this into the presenter view and projector.

#[derive(Debug, Clone, Default, PartialEq)]
pub struct PresentationState {
    pub number: Option<String>,
    pub title: String,
    pub slides: Vec<String>,
    pub index: usize,
    pub blank: bool,
}

impl PresentationState {
    /// Load a hymn for presentation: reset to the first slide, unblank.
    pub fn load_hymn(&mut self, number: Option<String>, title: String, slides: Vec<String>) {
        self.number = number;
        self.title = title;
        self.slides = slides;
        self.index = 0;
        self.blank = false;
    }

    pub fn slide_count(&self) -> usize {
        self.slides.len()
    }

    /// The current slide's text, or None if no hymn is loaded.
    pub fn current_slide(&self) -> Option<&str> {
        self.slides.get(self.index).map(|s| s.as_str())
    }

    /// A peek at the next slide, or None if on the last (or no) slide.
    pub fn next_slide(&self) -> Option<&str> {
        self.slides.get(self.index + 1).map(|s| s.as_str())
    }

    /// Advance one slide, clamped at the last slide (no playlist roll-over).
    pub fn next(&mut self) {
        if self.index + 1 < self.slides.len() {
            self.index += 1;
        }
    }

    /// Go back one slide, clamped at the first.
    pub fn prev(&mut self) {
        self.index = self.index.saturating_sub(1);
    }

    pub fn toggle_blank(&mut self) {
        self.blank = !self.blank;
    }
}
