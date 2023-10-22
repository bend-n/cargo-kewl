use crate::ui::*;
use crate::{cargo::TestEvent, test::TestState};

pub fn progress<B: Backend>(f: &mut Frame<B>, state: &TestState, chunk: Rect) {
    let size =
        |n| (n as f32 / state.test_count as f32 * f32::from(chunk.width)).round() as usize * 3;
    const LINE: &str = "──────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────";
    let mut passing = 0;
    let mut ignored = 0;
    let mut failing = 0;
    let mut running = 0;
    for test in &state.tests {
        match test {
            TestEvent::Ok { .. } => passing += 1,
            TestEvent::Ignored { .. } => ignored += 1,
            TestEvent::Failed { .. } | TestEvent::Timeout { .. } => failing += 1,
            TestEvent::Started { .. } => running += 1,
        }
    }
    let progress = Paragraph::new(ctext!(
        "{:cyan}{:green}{:red}{:yellow}",
        &LINE[..size(ignored)],
        &LINE[..size(passing)],
        &LINE[..size(failing)],
        &LINE[..size(running)],
    ));
    f.render_widget(
        progress.block(Block::default().borders(Borders::ALL).border_type(Rounded)),
        chunk,
    );
}
