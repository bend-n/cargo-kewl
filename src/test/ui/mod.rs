mod inspector;
mod progress;
pub mod stdout;
pub mod test_list;
use super::Screen;
use crate::cargo;
use crate::ui::*;

pub fn ui<B: Backend>(f: &mut Frame<B>, state: &mut super::TestState, meta: &cargo::Metadata) {
    let chunks = Layout::default()
        .direction(Vertical)
        .constraints([Length(3), Min(1), Length(1)])
        .split(f.size());
    let title_chunks = Layout::default()
        .direction(Horizontal)
        .constraints([Percentage(10), Percentage(80)])
        .split(chunks[0]);
    f.render_widget(
        Paragraph::new(ctext!(
            "{green}testing {:bold_cyan}{reset}",
            meta.package.name
        ))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(Rounded)
                .style(Style::default()),
        ),
        title_chunks[0],
    );
    progress::progress(f, state, title_chunks[1]);
    if state.test_list.selects(state).is_some() {
        let main_panels = match state.screen {
            Screen::Main => Layout::default()
                .direction(Horizontal)
                .constraints([Percentage(80), Percentage(20)])
                .split(chunks[1]),
            Screen::Stdout => Layout::default()
                .direction(Horizontal)
                .constraints([Percentage(60), Percentage(40)])
                .split(chunks[1]),
        };
        test_list::test_list(f, state, main_panels[0]);
        inspector::inspector(f, state, main_panels[1]);
    } else {
        test_list::test_list(f, state, chunks[1]);
    }
    let footer_chunks = Layout::default()
        .direction(Horizontal)
        .constraints([Percentage(50), Percentage(50)])
        .split(chunks[2]);
    let usage = match state.screen {
        Screen::Main => match state.test_list.selects(state) {
            Some(t) if t.stdout().is_some() => {
                Paragraph::new(ctext!("press {green}right{reset} to view the stdout"))
            }
            _ => Paragraph::new(ctext!(
                "press {green}up{reset} or {red}down{reset} to change selection"
            )),
        },
        Screen::Stdout => {
            Paragraph::new(ctext!("press {blue}left{reset} to go back to tests | press {green}up{reset} or {red}down{reset} to scroll stdout"))
        }
    };
    f.render_widget(usage, footer_chunks[0]);
    let status = match state.screen {
        Screen::Main => match state.test_list.selects(state) {
            Some(t) => Paragraph::new(ctext!("viewing test {:blue}", t.name())),
            None => Paragraph::new("listing tests"),
        },
        Screen::Stdout => Paragraph::new(ctext!(
            "viewing stdout of test {:blue}",
            state.test_list.selects(state).unwrap().name()
        )),
    };
    f.render_widget(status, footer_chunks[1]);
}
