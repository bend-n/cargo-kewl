use crate::cargo::TestEvent;
use crate::test::TestState;
use crate::ui::*;
use std::time::Duration;
#[derive(Default)]
pub struct TestList {
    a: SList,
    b: SList,
    c: SList,
}

impl TestList {
    fn has(&mut self, n: usize) {
        self.all().map(|a| a.has(n));
    }

    fn all(&mut self) -> [&mut SList; 3] {
        [&mut self.a, &mut self.b, &mut self.c]
    }

    pub fn next(&mut self) {
        self.all().map(SList::next);
    }

    pub fn prev(&mut self) {
        self.all().map(SList::prev);
    }

    pub fn selects<'a>(&'a self, state: &'a TestState) -> Option<&TestEvent> {
        state.tests.get(self.a.state.selected()?)
    }

    pub fn stdout<'a>(&'a self, state: &'a TestState) -> Option<&str> {
        self.selects(state)?.stdout()
    }
}

pub fn test_list<B: Backend>(f: &mut Frame<B>, state: &mut TestState, chunk: Rect) {
    let mut tests = Vec::<ListItem>::new();
    let mut test_side1 = Vec::<ListItem>::new();
    let mut test_side2 = Vec::<ListItem>::new();
    fn time<'v>(secs: f32) -> Line<'v> {
        let dur = Duration::from_secs_f32(secs);
        let time = humantime::format_duration(dur).to_string();
        match (secs / 16.).round() as usize {
            0 => Line::styled(time, Style::default().green()),
            1 => Line::styled(time, Style::default().yellow()),
            _ => Line::styled(time, Style::default().red()),
        }
    }
    for test in &state.tests {
        match test {
            TestEvent::Started { name } => {
                tests.pl(name.bold().yellow());
                test_side1.pl("in progress".yellow().italic());
                test_side2.pl("");
            }
            TestEvent::Ok {
                name, exec_time, ..
            } => {
                tests.pl(name.bold().green());
                test_side1.pl("passed".green().italic());
                test_side2.pl(time(*exec_time));
            }
            TestEvent::Failed {
                name, exec_time, ..
            } => {
                tests.pl(name.bold().red());
                test_side1.pl("failed".red().bold().italic());
                test_side2.pl(time(*exec_time));
            }
            TestEvent::Timeout { name } => {
                tests.pl(name.bold().red());
                test_side1.pl("timed out".red().bold().italic());
                test_side2.pl("");
            }
            TestEvent::Ignored { name } => {
                tests.pl(name.bold().yellow());
                test_side1.pl("ignored".yellow().italic());
                test_side2.pl("");
            }
        }
    }
    let sides = Layout::default()
        .direction(Horizontal)
        .constraints([Percentage(80), Percentage(10), Percentage(10)])
        .split(chunk);
    let hl = Style::default().on_light_green().italic();
    state.test_list.has(tests.len());
    f.render_stateful_widget(
        List::new(tests)
            .highlight_style(hl)
            .highlight_symbol("> ")
            .block(Block::default().borders(Borders::LEFT | Borders::TOP | Borders::BOTTOM)),
        sides[0],
        &mut state.test_list.a.state,
    );
    f.render_stateful_widget(
        List::new(test_side1)
            .highlight_style(hl)
            .block(Block::default().borders(Borders::TOP | Borders::BOTTOM)),
        sides[1],
        &mut state.test_list.b.state,
    );
    f.render_stateful_widget(
        List::new(test_side2)
            .highlight_style(hl)
            .block(Block::default().borders(Borders::TOP | Borders::BOTTOM | Borders::RIGHT)),
        sides[2],
        &mut state.test_list.c.state,
    );
}
