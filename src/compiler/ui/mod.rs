use super::Crate;
use super::FINISHED;
use crate::cargo;
use crate::ui::*;

pub fn ui<B: Backend>(f: &mut Frame<B>, state: &mut super::State, meta: &cargo::Metadata) {
    let chunks = Layout::default()
        .direction(Vertical)
        .constraints([Length(3), Min(1), Length(1)])
        .split(f.size());
    f.render_widget(
        if state.failed {
            Paragraph::new(ctext!(
                "{green}compiling {:bold_red}{reset}",
                meta.package.name
            ))
        } else {
            Paragraph::new(ctext!(
                "{green}compiling {:bold_cyan}{reset}",
                meta.package.name
            ))
        }
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(Rounded)
                .style(Style::default()),
        ),
        chunks[0],
    );
    let mut l = Vec::with_capacity(state.crates.len());
    for Crate { state, pid, .. } in &state.crates {
        let name = pid.repr.split(' ').next().unwrap();
        if state & FINISHED != 0 {
            l.pt(ctext!("{green}built    {:blue}", name));
        } else {
            l.pt(ctext!("{yellow}building {:blue}", name));
        }
    }
    let l = List::new(l)
        .highlight_style(Style::default().on_light_green().italic())
        .highlight_symbol("> ")
        .block(Block::default().borders(Borders::ALL));
    if state.diagnostics.is_empty() {
        f.render_stateful_widget(l, chunks[1], &mut state.compiled.state);
    } else {
        let chunks = Layout::default()
            .direction(Horizontal)
            .constraints([Percentage(60), Percentage(40)])
            .split(chunks[1]);
        f.render_stateful_widget(l, chunks[0], &mut state.compiled.state);
        let o = state.diagnostics.concat();
        let lines = o.lines().count() as u16;
        f.render_widget(
            Paragraph::new(o)
                .scroll((lines.saturating_sub(chunks[1].height), 0))
                .block(Block::default().title("diagnostics").borders(Borders::ALL)),
            chunks[1],
        );
    }

    let footer_chunks = Layout::default()
        .direction(Horizontal)
        .constraints([Percentage(50), Percentage(50)])
        .split(chunks[2]);
    let usage = Paragraph::new(ctext!(
        "press {green}up{reset} or {red}down{reset} to change selection"
    ));
    f.render_widget(usage, footer_chunks[0]);
    let status = match (|| state.crates.get(state.compiled.state.selected()?))() {
        Some(c) => Paragraph::new(ctext!(
            "viewing crate {:blue}",
            c.pid.repr.split(' ').next().unwrap()
        )),
        None => Paragraph::new("listing crates"),
    };
    f.render_widget(status, footer_chunks[1]);
}
