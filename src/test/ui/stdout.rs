#[derive(Default)]
pub struct Stdout {
    pub scroll: u16,
    pub lines: u16,
}

impl Stdout {
    pub fn incr(&mut self) {
        self.scroll = std::cmp::min(self.lines, self.scroll + 1);
    }

    pub fn decr(&mut self) {
        self.scroll = self.scroll.saturating_sub(1);
    }
}
