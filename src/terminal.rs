use alacritty_terminal::term::{Term, TermMode};
use alacritty_terminal::event::EventListener;
use alacritty_terminal::grid::Dimensions;
use alacritty_terminal::vte::ansi::Processor;

pub struct EventHandler;
impl EventListener for EventHandler {
    fn send_event(&self, _event: alacritty_terminal::event::Event) {}
}

pub struct Terminal {
    pub term: Term<EventHandler>,
    pub processor: Processor,
}

struct Size {
    cols: usize,
    rows: usize,
}

impl Dimensions for Size {
    fn columns(&self) -> usize { self.cols }
    fn screen_lines(&self) -> usize { self.rows }
    fn total_lines(&self) -> usize { self.rows }
}

impl Terminal {
    pub fn new(rows: u16, cols: u16) -> Self {
        let size = Size {
            cols: cols as usize,
            rows: rows as usize,
        };
        let term = Term::new(Default::default(), &size, EventHandler);
        let processor = Processor::new();
        Self { term, processor }
    }

    pub fn process_input(&mut self, data: Vec<u8>) {
        self.processor.advance(&mut self.term, &data);
    }

    pub fn resize(&mut self, rows: u16, cols: u16) {
        let size = Size {
            cols: cols as usize,
            rows: rows as usize,
        };
        self.term.resize(size);
    }

    pub fn is_app_cursor_mode(&self) -> bool {
        self.term.mode().contains(TermMode::APP_CURSOR)
    }
}
