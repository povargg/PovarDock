use super::status_options::StatusItem;

/// Blocks in the dock's saved row order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Section {
    Launcher,
    Apps,
    Tray,
    Status,
    Icon(StatusItem),
    Clock,
}

pub const DEFAULT_ORDER: [Section; 9] = [
    Section::Launcher,
    Section::Apps,
    Section::Tray,
    Section::Status,
    Section::Icon(StatusItem::Sound),
    Section::Icon(StatusItem::Battery),
    Section::Icon(StatusItem::Wifi),
    Section::Icon(StatusItem::Bluetooth),
    Section::Clock,
];

impl Section {
    pub fn name(self) -> &'static str {
        match self {
            Self::Launcher => "launcher",
            Self::Apps => "apps",
            Self::Tray => "tray",
            Self::Status => "status",
            Self::Icon(item) => item.name(),
            Self::Clock => "clock",
        }
    }
}

/// Unknown and duplicate names disappear. An omitted block goes after the block
/// it follows by default, so a section added in a later version starts in its usual place.
pub fn section_order<'a>(names: impl IntoIterator<Item = &'a str>) -> [Section; 9] {
    let mut order = Vec::with_capacity(9);
    for section in names
        .into_iter()
        .filter_map(|name| DEFAULT_ORDER.into_iter().find(|section| section.name() == name))
    {
        if !order.contains(&section) {
            order.push(section);
        }
    }
    for (i, section) in DEFAULT_ORDER.into_iter().enumerate() {
        if !order.contains(&section) {
            let at = match i {
                0 => 0,
                _ => order.iter().position(|&item| item == DEFAULT_ORDER[i - 1]).unwrap() + 1,
            };
            order.insert(at, section);
        }
    }
    order.try_into().unwrap()
}

pub fn move_section(order: &mut [Section; 9], section: Section, target: Section) {
    let from = order.iter().position(|&item| item == section).unwrap();
    let to = order.iter().position(|&item| item == target).unwrap();
    if from < to {
        order[from..=to].rotate_left(1);
    } else {
        order[to..=from].rotate_right(1);
    }
}
