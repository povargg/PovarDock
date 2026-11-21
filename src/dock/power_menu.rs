//! The dock's themed power menu and its confirmation state.

use windows::Win32::Foundation::HWND;

use crate::commands::power::{self, Power};

pub const WIDTH: f32 = 218.0;
pub const PAD: f32 = 7.0;
pub const ROW: f32 = 34.0;
pub const HEIGHT: f32 = 2.0 * PAD + 3.0 * ROW;

#[derive(Clone, Copy)]
pub enum Item {
    Action(Power),
    Prompt(Power),
    Cancel,
    Confirm(Power),
}

impl Item {
    pub fn title(self) -> &'static str {
        match self {
            Self::Action(action) | Self::Confirm(action) => action.title(),
            Self::Prompt(Power::Restart) => "Restart this PC?",
            Self::Prompt(_) => "Shut down this PC?",
            Self::Cancel => "Cancel",
        }
    }

    pub fn glyph(self) -> &'static str {
        match self {
            Self::Action(action) | Self::Confirm(action) => action.glyph(),
            Self::Prompt(_) => "\u{E7BA}",
            Self::Cancel => "\u{E711}",
        }
    }
}

#[derive(Default)]
pub struct Menu {
    confirm: Option<Power>,
    pub selection: Option<usize>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum Choice {
    Stay,
    Close,
    Run(Power),
}

impl Menu {
    pub fn items(&self) -> [Item; 3] {
        if let Some(action) = self.confirm {
            [Item::Prompt(action), Item::Cancel, Item::Confirm(action)]
        } else {
            Power::DOCK_MENU.map(Item::Action)
        }
    }

    pub fn hit(&self, x: f32, y: f32) -> Option<usize> {
        if !(PAD..WIDTH - PAD).contains(&x) || !(PAD..HEIGHT - PAD).contains(&y) {
            return None;
        }
        let index = ((y - PAD) / ROW) as usize;
        (self.confirm.is_none() || index != 0).then_some(index)
    }

    pub fn navigate(&mut self, forward: bool) {
        let first = usize::from(self.confirm.is_some());
        self.selection = Some(match (self.selection, forward) {
            (Some(i), true) if i < 2 => i + 1,
            (Some(i), false) if i > first => i - 1,
            (_, true) => first,
            (_, false) => 2,
        });
    }

    pub fn choose(&mut self, index: Option<usize>) -> Choice {
        match index.and_then(|i| self.items().get(i).copied()) {
            Some(Item::Action(action)) if action.needs_confirmation() => {
                self.confirm = Some(action);
                self.selection = Some(1);
                Choice::Stay
            }
            Some(Item::Action(action) | Item::Confirm(action)) => Choice::Run(action),
            Some(Item::Cancel) => Choice::Close,
            _ => Choice::Stay,
        }
    }
}

pub fn run(owner: HWND, action: Power) {
    if let Err(error) = power::run(action) {
        let title = format!("Could not {}", action.title().to_lowercase());
        crate::tray::notify(owner, &title, &error, true);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn destructive_actions_start_with_cancel_selected() {
        let mut menu = Menu::default();
        assert_eq!(menu.choose(Some(0)), Choice::Stay);
        assert_eq!(menu.selection, Some(1));
        assert_eq!(menu.hit(12.0, PAD + 1.0), None);
        assert_eq!(menu.choose(menu.selection), Choice::Close);
        menu.navigate(true);
        assert_eq!(menu.choose(menu.selection), Choice::Run(Power::ShutDown));
    }
}
