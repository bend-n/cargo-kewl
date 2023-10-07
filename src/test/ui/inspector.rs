use ratatui::{
    layout::Direction::Vertical,
    prelude::*,
    style::Stylize,
    widgets::{Block, BorderType::*, Borders, Paragraph, Wrap},
    Frame,
};

use crate::{
    cargo::TestResult,
    ctext,
    test::{Screen, Test, TestState},
};

pub fn inspector<B: Backend>(f: &mut Frame<B>, state: &TestState, chunk: Rect) {
    let Some(t) = state.test_list.selects(state) else {
        return;
    };
    let b = Block::default().title("inspect test").borders(Borders::ALL);
    let stdblock = || {
        let b = Block::default().borders(Borders::ALL).title("stdout");
        if state.screen == Screen::Stdout {
            return b.border_type(Thick).title_style(Style::default().italic());
        }
        b
    };
    match t {
        Test::Ignored { name } => {
            f.render_widget(
                Paragraph::new(ctext!("test {:bold_yellow} was ignored", name))
                    .alignment(Alignment::Center)
                    .block(b)
                    .wrap(Wrap { trim: true }),
                chunk,
            );
        }
        Test::Failed(TestResult { name, stdout, .. }) => {
            if let Some(stdout) = stdout {
                let chunks = Layout::new()
                    .direction(Vertical)
                    .constraints([Constraint::Percentage(10), Constraint::Percentage(90)])
                    .split(chunk);
                f.render_widget(
                    Paragraph::new(ctext!("test {:bold_red} failed", name))
                        .alignment(Alignment::Center)
                        .block(b),
                    chunks[0],
                );
                f.render_widget(
                    Paragraph::new(<String as ansi_to_tui::IntoText>::into_text(stdout).unwrap())
                        .block(stdblock())
                        .scroll((state.stdout.scroll, 0)),
                    chunks[1],
                );
            } else {
                f.render_widget(
                    Paragraph::new(ctext!("test {:bold_red} failed", name))
                        .alignment(Alignment::Center)
                        .block(b)
                        .wrap(Wrap { trim: true }),
                    chunk,
                );
            }
        }
        Test::Succeeded(TestResult { name, stdout, .. }) => {
            if let Some(stdout) = stdout {
                let chunks = Layout::new()
                    .direction(Vertical)
                    .constraints([Constraint::Percentage(10), Constraint::Percentage(90)])
                    .split(chunk);
                f.render_widget(
                    Paragraph::new(ctext!("test {:bold_green} passed", name))
                        .alignment(Alignment::Center)
                        .block(b),
                    chunks[0],
                );
                f.render_widget(
                    Paragraph::new(<String as ansi_to_tui::IntoText>::into_text(stdout).unwrap())
                        .block(stdblock())
                        .scroll((state.stdout.scroll, 0)),
                    chunks[1],
                );
            } else {
                f.render_widget(
                    Paragraph::new(ctext!("test {:bold_green} passed", name))
                        .alignment(Alignment::Center)
                        .block(b)
                        .wrap(Wrap { trim: true }),
                    chunk,
                );
            }
        }
        Test::InProgress { name } => {
            f.render_widget(
                Paragraph::new(ctext!("test {:bold_yellow} in progress", name))
                    .alignment(Alignment::Center)
                    .block(b)
                    .wrap(Wrap { trim: true }),
                chunk,
            );
        }
    }
}
