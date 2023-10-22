use ratatui::widgets::ListState;

#[derive(Default)]
pub struct SList {
    pub state: ListState,
    pub itemc: usize,
}

pub const fn incr(what: usize, cap: usize) -> usize {
    if what > cap - 1 {
        0
    } else {
        what + 1
    }
}

pub const fn decr(what: usize, cap: usize) -> usize {
    if what == 0 {
        cap - 1
    } else {
        what - 1
    }
}

impl SList {
    pub fn next(&mut self) {
        let i = self.state.selected().map_or(0, |x| incr(x, self.itemc));
        self.state.select(Some(i));
    }

    pub fn prev(&mut self) {
        let i = self.state.selected().map_or(0, |x| decr(x, self.itemc));
        self.state.select(Some(i));
    }
    pub fn has(&mut self, n: usize) {
        self.itemc = n;
    }
}
