pub mod ls;
pub(crate) use crate::ctext;
pub use ls::SList;
pub use ratatui::{
    layout::{Constraint::*, Direction::*},
    prelude::*,
    widgets::{Block, BorderType::*, Borders, List, ListItem, Paragraph, Wrap},
    Frame,
};

pub trait RExt<'a> {
    fn pl(&mut self, list: impl Into<Line<'a>>);
    fn pt(&mut self, list: Text<'a>) {
        for l in list.lines {
            self.pl(l);
        }
    }
}

impl<'a> RExt<'a> for Vec<ListItem<'a>> {
    fn pl(&mut self, list: impl Into<Line<'a>>) {
        self.push(ListItem::new(list.into()));
    }
}
