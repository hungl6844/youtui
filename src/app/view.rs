/// Traits related to viewable application components.
use super::{structures::Percentage, YoutuiMutableState};
use crate::Result;
use ratatui::{
    prelude::{Constraint, Rect},
    Frame,
};
use std::{borrow::Cow, fmt::Display};

pub mod draw;

#[derive(Clone, Debug)]
pub struct TableSortCommand {
    pub column: usize,
    pub direction: SortDirection,
}

#[derive(Default, Clone, Copy, Debug, PartialEq)]
pub enum SortDirection {
    #[default]
    Asc,
    Desc,
}

#[derive(Clone, Debug)]
pub enum TableFilterCommand {
    All(Filter),
    Column { filter: Filter, column: usize },
}
#[derive(Clone, Debug)]
pub enum Filter {
    Contains(FilterString),
    NotContains(FilterString),
    Equal(FilterString),
}
#[derive(Clone, Debug)]
pub enum FilterString {
    CaseSensitive(String),
    CaseInsensitive(String),
}

impl TableFilterCommand {
    fn as_readable(&self) -> String {
        match self {
            TableFilterCommand::All(f) => format!("ALL{}", f.as_readable()),
            TableFilterCommand::Column { filter, column } => {
                format!("COL{}{}", column, filter.as_readable())
            }
        }
    }
    #[deprecated = "Temporary function to be replaced with as_readable"]
    fn as_basic_readable(&self) -> String {
        match self {
            TableFilterCommand::All(f) => match f {
                Filter::Contains(f) => match f {
                    FilterString::CaseSensitive(_) => todo!(),
                    FilterString::CaseInsensitive(s) => format!("[a-Z]*{}*", s),
                },
                Filter::NotContains(_) => todo!(),
                Filter::Equal(_) => todo!(),
            },
            TableFilterCommand::Column { .. } => todo!(),
        }
    }
}
impl Filter {
    fn as_readable(&self) -> String {
        match self {
            Filter::Contains(f) => format!("~{}", f.as_readable()),
            Filter::NotContains(f) => format!("!={}", f.as_readable()),
            Filter::Equal(f) => format!("={}", f.as_readable()),
        }
    }
}
impl FilterString {
    fn as_readable(&self) -> String {
        match self {
            FilterString::CaseSensitive(s) => format!("A:{s}"),
            FilterString::CaseInsensitive(s) => format!("a:{s}"),
        }
    }
    pub fn is_in<S: AsRef<str>>(&self, test_str: S) -> bool {
        match self {
            FilterString::CaseSensitive(s) => test_str.as_ref().contains(s),
            // XXX: Ascii lowercase may not be correct.
            FilterString::CaseInsensitive(s) => test_str
                .as_ref()
                .to_ascii_lowercase()
                .contains(s.to_ascii_lowercase().as_str()),
        }
    }
}

/// Basic wrapper around constraint to allow mixing of percentage and length.
pub enum BasicConstraint {
    Length(u16),
    Percentage(Percentage),
}

// TODO: Add more tests
/// Use basic constraints to construct dynamic column widths for a table.
pub fn basic_constraints_to_table_constraints(
    basic_constraints: &[BasicConstraint],
    length: u16,
    margin: u16,
) -> Vec<Constraint> {
    let sum_lengths = basic_constraints.iter().fold(0, |acc, c| {
        acc + match c {
            BasicConstraint::Length(l) => *l,
            BasicConstraint::Percentage(_) => 0,
        } + margin
    });
    basic_constraints
        .iter()
        .map(|bc| match bc {
            BasicConstraint::Length(l) => Constraint::Length(*l),
            BasicConstraint::Percentage(p) => {
                Constraint::Length(p.0 as u16 * length.saturating_sub(sum_lengths) / 100)
            }
        })
        .collect()
}

// A struct that is able to be "scrolled". An item will always be selected.
// XXX: Should a Scrollable also be a KeyHandler? This way, can potentially have common keybinds.
pub trait Scrollable {
    // Increment the list by the specified amount.
    fn increment_list(&mut self, amount: isize);
    fn get_selected_item(&self) -> usize;
}
/// A struct that can either be scrolled or forward scroll commands to a component.
// To allow scrolling at a top level.
pub trait MaybeScrollable {
    /// Try to increment the list by the selected amount, return true if command was handled.
    fn increment_list(&mut self, amount: isize) -> bool;
    /// Return true if a scrollable component in the application is active.
    fn scrollable_component_active(&self) -> bool;
}

/// A simple row in a table.
pub type TableItem<'a> = Box<dyn Iterator<Item = Cow<'a, str>> + 'a>;

/// A struct that we are able to draw a table from using the underlying data.
pub trait TableView: Scrollable + Loadable {
    // NOTE: Consider if the Playlist is a NonSortableTable (or Browser a SortableTable), as possible we don't want to sort the Playlist (what happens to play order, for eg).
    // Could have a "commontitle" trait to prevent the need for this in both Table and List
    fn get_title(&self) -> Cow<str>;
    fn get_layout(&self) -> &[BasicConstraint];
    // TODO: Consider if generics <T: Iterator> can be used instead of dyn Iterator.
    fn get_items(&self) -> Box<dyn ExactSizeIterator<Item = TableItem> + '_>;
    // XXX: This doesn't need to be so fancy - could return a static slice.
    fn get_headings(&self) -> Box<dyn Iterator<Item = &'static str>>;
    // Not a particularyl useful function for a sortabletableview
    fn len(&self) -> usize {
        self.get_items().len()
    }
}
pub trait SortableTableView: TableView {
    fn get_sortable_columns(&self) -> &[usize];
    fn get_sort_commands(&self) -> &[TableSortCommand];
    /// This can fail if the TableSortCommand is not within the range of sortable columns.
    fn push_sort_command(&mut self, sort_command: TableSortCommand) -> Result<()>;
    fn clear_sort_commands(&mut self);
    // Assuming a SortableTable is also Filterable.
    fn get_filterable_columns(&self) -> &[usize];
    // This can't be ExactSized as return type may be Filter<T>
    fn get_filtered_items(&self) -> Box<dyn Iterator<Item = TableItem> + '_>;
    fn get_filter_commands(&self) -> &[TableFilterCommand];
    fn push_filter_command(&mut self, filter_command: TableFilterCommand);
    fn clear_filter_commands(&mut self);
}
// A struct that we are able to draw a list from using the underlying data.
pub trait ListView: Scrollable + SortableList + Loadable {
    type DisplayItem: Display;
    fn get_title(&self) -> Cow<str>;
    fn get_items_display(&self) -> Vec<&Self::DisplayItem>;
    fn len(&self) -> usize {
        self.get_items_display().len()
    }
}
pub trait SortableList {
    fn push_sort_command(&mut self, list_sort_command: String);
    fn clear_sort_commands(&mut self);
}
pub trait FilterableList {
    fn push_filter_command(&mut self, list_filter_command: String);
    fn clear_filter_commands(&mut self);
}
// A drawable part of the application.
pub trait Drawable {
    // Helper function to draw.
    fn draw_chunk(&self, f: &mut Frame, chunk: Rect, selected: bool);
    fn draw(&self, f: &mut Frame, selected: bool) {
        self.draw_chunk(f, f.size(), selected);
    }
}
// A drawable part of the application that mutates its state on draw.
pub trait DrawableMut {
    // Helper function to draw.
    // TODO: Clean up function signature regarding mutable state.
    fn draw_mut_chunk(
        &self,
        f: &mut Frame,
        chunk: Rect,
        mutable_state: &mut YoutuiMutableState,
        selected: bool,
    );
    fn draw_mut(&self, f: &mut Frame, mutable_state: &mut YoutuiMutableState, selected: bool) {
        self.draw_mut_chunk(f, f.size(), mutable_state, selected);
    }
}
// A part of the application that can be in a Loading state.
pub trait Loadable {
    fn is_loading(&self) -> bool;
}

#[cfg(test)]
mod tests {
    use ratatui::prelude::Constraint;

    use super::{basic_constraints_to_table_constraints, BasicConstraint};
    use crate::app::structures::Percentage;

    #[test]
    fn test_constraints() {
        let basic_constraints = &[
            BasicConstraint::Length(5),
            BasicConstraint::Length(5),
            BasicConstraint::Percentage(Percentage(100)),
        ];
        let constraints = vec![
            Constraint::Length(5),
            Constraint::Length(5),
            Constraint::Length(10),
        ];
        let converted = basic_constraints_to_table_constraints(basic_constraints, 20, 0);
        assert_eq!(converted, constraints);
        let basic_constraints = &[
            BasicConstraint::Length(5),
            BasicConstraint::Length(5),
            BasicConstraint::Percentage(Percentage(50)),
            BasicConstraint::Percentage(Percentage(50)),
        ];
        let constraints = vec![
            Constraint::Length(5),
            Constraint::Length(5),
            Constraint::Length(5),
            Constraint::Length(5),
        ];
        let converted = basic_constraints_to_table_constraints(basic_constraints, 20, 0);
        assert_eq!(converted, constraints);
    }
}
