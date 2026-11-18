//! Live shortcuts. Inherited events still go through the original matcher;
//! displayed defaults identify its alternatives, not its modifier language.
use super::{Command, from_key};
use iced::keyboard::{Key, Modifiers, key::Named};
use meld_config::settings::ShortcutOverrides;
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Context {
    New,
    File,
    Folder,
    Vc,
    Git,
    GitDiff,
    Image,
}

const CONTEXTS: [Context; 7] = [
    Context::New,
    Context::File,
    Context::Folder,
    Context::Vc,
    Context::Git,
    Context::GitDiff,
    Context::Image,
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Scope {
    Window,
    Comparison,
    Changes,
    Panes,
    FileFolder,
    Search,
    File,
    Folder,
    Vc,
    Git,
    Stop,
}

impl Scope {
    fn contains(self, context: Context) -> bool {
        use Context::*;
        match self {
            Self::Window => true,
            Self::Comparison => context != New,
            Self::Changes | Self::Search => matches!(context, File | Folder | Vc | GitDiff),
            Self::Panes => matches!(context, File | Folder | Image | GitDiff),
            Self::FileFolder => matches!(context, File | Folder | GitDiff),
            Self::File => matches!(context, File | GitDiff),
            Self::Folder => context == Folder,
            Self::Vc => context == Vc,
            Self::Git => matches!(context, Git | GitDiff),
            Self::Stop => !matches!(context, New | Image),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Owner {
    App,
    Search,
}

pub struct Action {
    pub id: &'static str,
    pub group: fn() -> &'static str,
    pub label: fn() -> &'static str,
    command: Command,
    scope: Scope,
    owner: Owner,
    // Identities are indexes into this append-only list. Do not reorder.
    defaults: &'static [&'static str],
}

pub const FOLDER_DELETE: &[&str] = if cfg!(target_os = "macos") {
    &["Delete", "Primary+Backspace"]
} else {
    &["Delete"]
};

macro_rules! actions {
    ($($group:literal { $($id:literal: $label:literal, $command:expr, $scope:ident, $owner:ident, $defaults:expr;)* })*) => {
        pub static ACTIONS: &[Action] = &[$($(Action {
            id: $id, group: || trc!("shortcut window", $group), label: || trc!("shortcut window", $label),
            command: $command, scope: Scope::$scope, owner: Owner::$owner, defaults: $defaults,
        },)*)*];
    };
}

use Command::*;
actions! {
    "Windows and Tabs" {
        "new-tab": "New comparison", NewTab, Window, App, &["Primary+N"];
        "new-blank-file-2-way": "New blank file comparison (2-way)", NewBlankFile2Way, Window, App, &[];
        "new-blank-file-3-way": "New blank file comparison (3-way)", NewBlankFile3Way, Window, App, &[];
        "close-tab": "Close a comparison", CloseTab, Window, App, &["Primary+W"];
        "quit": "Quit Meld", Quit, Window, App, &["Primary+Q"];
        "previous-tab": "Go to previous tab", PreviousTab, Window, App, &["Primary+PgUp", "Primary+Shift+Tab"];
        "next-tab": "Go to next tab", NextTab, Window, App, &["Primary+PgDn", "Primary+Tab"];
        "select-tab-1": "Switch to tab 1", SelectTab(0), Window, App, &["Alt+1"];
        "select-tab-2": "Switch to tab 2", SelectTab(1), Window, App, &["Alt+2"];
        "select-tab-3": "Switch to tab 3", SelectTab(2), Window, App, &["Alt+3"];
        "select-tab-4": "Switch to tab 4", SelectTab(3), Window, App, &["Alt+4"];
        "select-tab-5": "Switch to tab 5", SelectTab(4), Window, App, &["Alt+5"];
        "select-tab-6": "Switch to tab 6", SelectTab(5), Window, App, &["Alt+6"];
        "select-tab-7": "Switch to tab 7", SelectTab(6), Window, App, &["Alt+7"];
        "select-tab-8": "Switch to tab 8", SelectTab(7), Window, App, &["Alt+8"];
        "select-tab-9": "Switch to tab 9", SelectTab(8), Window, App, &["Alt+9"];
        "select-tab-10": "Switch to tab 10", SelectTab(9), Window, App, &["Alt+0"];
    }
    "General" {
        "help": "Show help", Help, Window, App, &["F1"];
        "preferences": "Show preferences", Preferences, Window, App, &["Primary+,"];
        "gear-menu": "Open menu", GearMenu, Window, App, &["F10"];
        "fullscreen": "Fullscreen", Fullscreen, Window, App, &["F11"];
    }
    "Common Actions" {
        "stop": "Stop the current action", Stop, Stop, App, &["Escape"];
        "refresh": "Refresh comparison", Refresh, Comparison, App, &["Primary+R", "F5"];
        "open-external": "Open externally", OpenExternal, Panes, App, &["Primary+Shift+O"];
        "filter": "Filters", Filter, FileFolder, App, &["F8"];
        "zoom-in": "Zoom in", ZoomIn, Window, App, &["Primary+="];
        "zoom-out": "Zoom out", ZoomOut, Window, App, &["Primary+-"];
        "zoom-reset": "Reset zoom", ZoomReset, Window, App, &["Primary+0"];
    }
    "Panes" {
        "previous-pane": "Previous comparison pane", PreviousPane, Panes, App, &["Alt+PgUp"];
        "next-pane": "Next comparison pane", NextPane, Panes, App, &["Alt+PgDn"];
        "swap-panes": "Swap left and right panes", SwapPanes, FileFolder, App, &["Alt+\\"];
    }
    "Changes" {
        "previous-change": "Go to previous change", PreviousChange, Changes, App, &["Alt+Up", "Primary+E"];
        "next-change": "Go to next change", NextChange, Changes, App, &["Alt+Down", "Primary+D"];
    }
    "Editing" {
        "undo": "Undo", Undo, File, App, &["Primary+Z"];
        "redo": "Redo", Redo, File, App, &["Primary+Shift+Z"];
        "find": "Find", Find, Search, App, &["Primary+F"];
        "find-next": "Find Next", FindNext, File, App, &["Primary+G", "F3"];
        "find-previous": "Find Previous", FindPrevious, File, App, &["Primary+Shift+G", "Shift+F3"];
        "replace": "Replace", FindReplace, File, App, &["Primary+H"];
        "go-to-line": "Go to line", GoToLine, File, App, &["Primary+I"];
    }
    "File comparison" {
        "save": "Save current file", Save, File, App, &["Primary+S"];
        "save-as": "Save current file to new path", SaveAs, File, App, &["Primary+Shift+S"];
        "save-all": "Save all files in comparison", SaveAll, File, App, &["Primary+Shift+L"];
        "previous-conflict": "Previous conflict", PreviousConflict, File, App, &["Primary+J"];
        "next-conflict": "Next conflict", NextConflict, File, App, &["Primary+K"];
        "push-left": "Push change to left", PushLeft, File, App, &["Alt+Left"];
        "push-right": "Push change to right", PushRight, File, App, &["Alt+Right"];
        "pull-left": "Pull change from left", PullLeft, File, App, &["Alt+Shift+Right"];
        "pull-right": "Pull change from right", PullRight, File, App, &["Alt+Shift+Left"];
        "copy-left-up": "Copy change above left", CopyLeftUp, File, App, &["Alt+["];
        "copy-left-down": "Copy change below left", CopyLeftDown, File, App, &["Alt+;"];
        "copy-right-up": "Copy change above right", CopyRightUp, File, App, &["Alt+]"];
        "copy-right-down": "Copy change below right", CopyRightDown, File, App, &["Alt+'"];
        "delete-change": "Delete change", DeleteChange, File, App, &["Alt+Delete"];
        "overview-map": "Show overview map", TogglePanel, File, App, &["F9"];
    }
    "Folder comparison" {
        "folder-compare": "Compare selected", Activate, Folder, App, &["Return"];
        "folder-copy-left": "Copy to left", PushLeft, Folder, App, &["Alt+Left"];
        "folder-copy-right": "Copy to right", PushRight, Folder, App, &["Alt+Right"];
        "folder-delete": "Delete selected", Delete, Folder, App, FOLDER_DELETE;
        "folder-find-next": "Find Next", FindNext, Folder, Search, &["Primary+G"];
        "folder-find-previous": "Find Previous", FindPrevious, Folder, Search, &["Primary+Shift+G"];
    }
    "Version control comparison" {
        "vc-compare": "Compare selected", Activate, Vc, App, &["Return"];
        "vc-delete": "Delete selected", Delete, Vc, App, &["Delete"];
        "vc-open-external": "Open externally", OpenExternal, Vc, App, &["Primary+Shift+O"];
        "vc-commit": "Commit to version control", Commit, Vc, App, &["Primary+M"];
        "vc-console": "Show/hide console output", TogglePanel, Vc, App, &["F9"];
        "vc-find-next": "Find Next", FindNext, Vc, Search, &["Primary+G"];
        "vc-find-previous": "Find Previous", FindPrevious, Vc, Search, &["Primary+Shift+G"];
    }
    "Git Compare" {
        "next-file": "Next changed file", NextFile, Git, App, &["Alt+F7"];
        "previous-file": "Previous changed file", PreviousFile, Git, App, &["Alt+Shift+F7"];
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Chord {
    key: Key,
    modifiers: Modifiers,
}

const NAMED_KEYS: &[(&str, Named)] = &[
    ("Escape", Named::Escape),
    ("Return", Named::Enter),
    ("Delete", Named::Delete),
    ("Backspace", Named::Backspace),
    ("Tab", Named::Tab),
    ("Space", Named::Space),
    ("Up", Named::ArrowUp),
    ("Down", Named::ArrowDown),
    ("Left", Named::ArrowLeft),
    ("Right", Named::ArrowRight),
    ("PgUp", Named::PageUp),
    ("PgDn", Named::PageDown),
    ("Home", Named::Home),
    ("End", Named::End),
    ("Insert", Named::Insert),
    ("F1", Named::F1),
    ("F2", Named::F2),
    ("F3", Named::F3),
    ("F4", Named::F4),
    ("F5", Named::F5),
    ("F6", Named::F6),
    ("F7", Named::F7),
    ("F8", Named::F8),
    ("F9", Named::F9),
    ("F10", Named::F10),
    ("F11", Named::F11),
    ("F12", Named::F12),
];

impl Chord {
    pub fn from_event(key: &Key, modifiers: Modifiers) -> Option<Self> {
        let key = match key {
            Key::Character(c) if c.len() == 1 && c.as_bytes()[0].is_ascii_graphic() => {
                Key::Character(c.to_lowercase().into())
            }
            Key::Named(n) if NAMED_KEYS.iter().any(|(_, candidate)| candidate == n) => key.clone(),
            _ => return None,
        };
        Some(Self { key, modifiers })
    }

    pub fn parse(mut text: &str) -> Option<Self> {
        let mut modifiers = Modifiers::empty();
        while let Some((prefix, rest)) = text.split_once('+') {
            if prefix.is_empty() {
                break;
            }
            let modifier = match prefix.trim().to_ascii_lowercase().as_str() {
                "primary" => Modifiers::COMMAND,
                "control" | "ctrl" => Modifiers::CTRL,
                "cmd" | "super" | "logo" => Modifiers::LOGO,
                "alt" | "option" => Modifiers::ALT,
                "shift" => Modifiers::SHIFT,
                _ => return None,
            };
            if modifiers.intersects(modifier) {
                return None;
            }
            modifiers |= modifier;
            text = rest;
        }
        let text = text.trim();
        let key = NAMED_KEYS
            .iter()
            .find(|(name, _)| name.eq_ignore_ascii_case(text))
            .map(|(_, named)| Key::Named(*named))
            .unwrap_or_else(|| Key::Character(text.into()));
        Self::from_event(&key, modifiers)
    }

    pub fn label(&self) -> String {
        self.format(false)
    }

    fn format(&self, storage: bool) -> String {
        let mut result = String::new();
        for (bit, label) in [
            (Modifiers::CTRL, if storage { "Control+" } else { "Ctrl+" }),
            (
                Modifiers::LOGO,
                if cfg!(target_os = "macos") && !storage {
                    "Cmd+"
                } else {
                    "Super+"
                },
            ),
            (Modifiers::ALT, "Alt+"),
            (Modifiers::SHIFT, "Shift+"),
        ] {
            if self.modifiers.contains(bit) {
                result.push_str(label);
            }
        }
        match &self.key {
            Key::Named(n) => {
                result.push_str(NAMED_KEYS.iter().find(|(_, key)| key == n).unwrap().0)
            }
            Key::Character(c) => result.push_str(&c.to_uppercase()),
            _ => unreachable!("normalized chord"),
        }
        result
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Alternative {
    Default(usize),
    Custom(Chord),
}

impl Action {
    pub fn defaults(&self) -> Vec<Alternative> {
        (0..self.defaults.len()).map(Alternative::Default).collect()
    }

    fn default_chord(&self, index: usize) -> Option<Chord> {
        Chord::parse(self.defaults.get(index)?)
    }

    pub fn label_for(&self, alternative: &Alternative) -> Option<String> {
        match alternative {
            Alternative::Default(index) => {
                let chord = self.default_chord(*index)?;
                // The Mac keycap says Delete although the event is Backspace.
                Some(
                    if self.id == "folder-delete"
                        && cfg!(target_os = "macos")
                        && chord.key == Key::Named(Named::Backspace)
                    {
                        "Cmd+Delete".into()
                    } else {
                        chord.label()
                    },
                )
            }
            Alternative::Custom(chord) => Some(chord.label()),
        }
    }

    fn default_matches(&self, index: usize, key: &Key, modifiers: Modifiers) -> bool {
        let Some(default) = self.default_chord(index) else {
            return false;
        };
        if self.owner == Owner::Search
            || self.id == "folder-delete" && default.key == Key::Named(Named::Backspace)
        {
            return Chord::from_event(key, modifiers).as_ref() == Some(&default);
        }
        if from_key(key, modifiers) != Some(self.command) {
            return false;
        }
        match (key, &default.key) {
            (Key::Character(c), Key::Character(d)) => {
                c.eq_ignore_ascii_case(d) || self.command == ZoomIn && c == "+"
            }
            _ => key == &default.key && modifiers.shift() == default.modifiers.shift(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BindingError {
    UnknownAction,
    InvalidAlternative,
    Conflict {
        action: &'static str,
        context: Context,
    },
    Reserved {
        context: Context,
    },
}

#[derive(Debug, Clone, Default)]
pub struct Bindings(BTreeMap<&'static str, Vec<Alternative>>);

impl Bindings {
    pub fn load(overrides: &ShortcutOverrides) -> Self {
        let mut map = Self::default();
        for (id, values) in overrides {
            let Some(action) = ACTIONS.iter().find(|action| action.id == id) else {
                continue;
            };
            let alternatives: Option<Vec<_>> = values
                .iter()
                .map(|value| {
                    if let Some(index) = value.strip_prefix('@') {
                        let index = index.parse().ok()?;
                        action.default_chord(index)?;
                        Some(Alternative::Default(index))
                    } else {
                        Chord::parse(value).map(Alternative::Custom)
                    }
                })
                .collect();
            if let Some(alternatives) = alternatives {
                map.0.insert(action.id, alternatives);
            } else {
                log::warn!("Ignoring invalid shortcut alternatives for '{id}'");
            }
        }
        // Validate against the whole effective map; reinstating an invalid
        // entry's defaults can expose another conflict, so recheck.
        while let Some(id) = map
            .0
            .iter()
            .find_map(|(&id, alternatives)| map.validate(id, alternatives).is_err().then_some(id))
        {
            log::warn!("Ignoring conflicting shortcut override for '{id}'");
            map.0.remove(id);
        }
        map
    }

    pub fn overrides(&self) -> ShortcutOverrides {
        self.0
            .iter()
            .map(|(id, alternatives)| {
                (
                    (*id).to_owned(),
                    alternatives
                        .iter()
                        .map(|a| match a {
                            Alternative::Default(index) => format!("@{index}"),
                            Alternative::Custom(chord) => chord.format(true),
                        })
                        .collect(),
                )
            })
            .collect()
    }

    pub fn alternatives(&self, action: &Action) -> Vec<Alternative> {
        self.0
            .get(action.id)
            .cloned()
            .unwrap_or_else(|| action.defaults())
    }

    pub fn customized(&self, id: &str) -> bool {
        self.0.contains_key(id)
    }

    /// `None` resets the action; `Some(vec![])` unbinds it.
    pub fn edited(
        &self,
        id: &str,
        alternatives: Option<Vec<Alternative>>,
    ) -> Result<Self, BindingError> {
        let action = ACTIONS
            .iter()
            .find(|action| action.id == id)
            .ok_or(BindingError::UnknownAction)?;
        let mut candidate = self.clone();
        match alternatives {
            Some(values) if values != action.defaults() => {
                candidate.0.insert(action.id, values);
            }
            _ => {
                candidate.0.remove(id);
            }
        }
        candidate.validate(id, &candidate.alternatives(action))?;
        Ok(candidate)
    }

    pub fn labels(&self, id: &str) -> Option<String> {
        let action = ACTIONS.iter().find(|a| a.id == id)?;
        let labels: Vec<_> = self
            .alternatives(action)
            .iter()
            .filter_map(|a| action.label_for(a))
            .collect();
        (!labels.is_empty()).then(|| labels.join(", "))
    }

    pub fn tip(&self, label: &str, id: &str) -> String {
        self.labels(id)
            .map_or_else(|| label.to_owned(), |keys| format!("{label} ({keys})"))
    }

    pub fn resolve(&self, context: Context, key: &Key, modifiers: Modifiers) -> Option<Command> {
        let legacy = from_key(key, modifiers);
        if self.0.is_empty() {
            return legacy;
        }
        if let Some(command) = self.overridden(context, Owner::App, key, modifiers) {
            return Some(command);
        }
        let command = legacy?;
        let overridden = ACTIONS.iter().any(|a| {
            a.owner == Owner::App
                && a.command == command
                && a.scope.contains(context)
                && self.customized(a.id)
        });
        (!overridden).then_some(command)
    }

    fn overridden(
        &self,
        context: Context,
        owner: Owner,
        key: &Key,
        modifiers: Modifiers,
    ) -> Option<Command> {
        let chord = Chord::from_event(key, modifiers);
        ACTIONS.iter().find_map(|a| {
            if a.owner != owner || !a.scope.contains(context) {
                return None;
            }
            self.0
                .get(a.id)?
                .iter()
                .any(|alternative| match alternative {
                    Alternative::Default(index) => {
                        !(a.id == "folder-delete" && key == &Key::Named(Named::Backspace))
                            && a.default_matches(*index, key, modifiers)
                    }
                    Alternative::Custom(custom) => Some(custom) == chord.as_ref(),
                })
                .then_some(a.command)
        })
    }

    pub fn mac_folder_delete(&self, key: &Key, modifiers: Modifiers) -> bool {
        cfg!(target_os = "macos")
            && key == &Key::Named(Named::Backspace)
            && modifiers == Modifiers::COMMAND
            && self
                .0
                .get("folder-delete")
                .is_none_or(|values| values.contains(&Alternative::Default(1)))
    }

    pub fn search_step(
        &self,
        context: Context,
        key: &Key,
        modifiers: Modifiers,
    ) -> Option<Command> {
        if let Some(command) = self.overridden(context, Owner::Search, key, modifiers) {
            return Some(command);
        }
        let command = match key {
            Key::Character(c)
                if c.eq_ignore_ascii_case("g")
                    && modifiers.command()
                    && !crate::ui::unsupported_tree_modifiers(modifiers) =>
            {
                if modifiers.shift() {
                    FindPrevious
                } else {
                    FindNext
                }
            }
            _ => return None,
        };
        ACTIONS
            .iter()
            .any(|a| {
                a.owner == Owner::Search
                    && a.scope.contains(context)
                    && a.command == command
                    && !self.customized(a.id)
            })
            .then_some(command)
    }

    fn validate(&self, id: &str, values: &[Alternative]) -> Result<(), BindingError> {
        let action = ACTIONS.iter().find(|action| action.id == id).unwrap();
        for (i, value) in values.iter().enumerate() {
            let chord = match value {
                Alternative::Default(index) => action
                    .default_chord(*index)
                    .ok_or(BindingError::InvalidAlternative)?,
                Alternative::Custom(chord) => chord.clone(),
            };
            for context in CONTEXTS.into_iter().filter(|c| action.scope.contains(*c)) {
                if matches!(value, Alternative::Custom(_))
                    && reserved(context, action.owner, &chord)
                {
                    return Err(BindingError::Reserved { context });
                }
                for other in ACTIONS.iter().filter(|a| a.scope.contains(context)) {
                    let alternatives = if other.id == id {
                        values[..i].to_vec()
                    } else {
                        self.alternatives(other)
                    };
                    for candidate in alternatives {
                        let conflict = match (&candidate, value) {
                            (Alternative::Custom(a), Alternative::Custom(b)) => a == b,
                            (Alternative::Default(index), Alternative::Custom(b)) => {
                                other.default_matches(*index, &b.key, b.modifiers)
                            }
                            (Alternative::Custom(a), Alternative::Default(index)) => {
                                action.default_matches(*index, &a.key, a.modifiers)
                            }
                            (Alternative::Default(a), Alternative::Default(b)) => {
                                other.id == id && a == b
                            }
                        };
                        if conflict {
                            return Err(BindingError::Conflict {
                                action: other.id,
                                context,
                            });
                        }
                    }
                }
            }
        }
        Ok(())
    }
}

fn reserved(context: Context, owner: Owner, chord: &Chord) -> bool {
    let modifiers = chord.modifiers;
    let plain = !modifiers
        .intersects(Modifiers::COMMAND | Modifiers::ALT | Modifiers::CTRL | Modifiers::LOGO);
    match &chord.key {
        // Escape is also dismissal; Tab moves focus and can change tabs.
        Key::Named(Named::Escape | Named::Tab) => true,
        Key::Named(
            Named::Enter | Named::Backspace | Named::Delete | Named::Insert | Named::Space,
        ) => true,
        Key::Named(
            Named::ArrowUp
            | Named::ArrowDown
            | Named::ArrowLeft
            | Named::ArrowRight
            | Named::PageUp
            | Named::PageDown
            | Named::Home
            | Named::End,
        ) => {
            // With no Alt, pane motion can take these even when Control
            // is literal (not Command). A focused entry also owns motion.
            !modifiers.alt()
                || owner == Owner::Search && !crate::ui::unsupported_tree_modifiers(modifiers)
        }
        Key::Character(c) => {
            // Text inputs also retain their own clipboard and history keys.
            plain || modifiers.command() && matches!(c.as_str(), "a" | "c" | "v" | "x" | "y" | "z")
                // Alt produces text on macOS; it is not a reliable new text chord.
                || cfg!(target_os = "macos") && modifiers.alt() && !modifiers.command() && !modifiers.control()
                || matches!(context, Context::File | Context::GitDiff) && modifiers.control() && matches!(c.as_str(), "a" | "b" | "d" | "e" | "f" | "h" | "k" | "n" | "p" | "u" | "w")
        }
        _ => false,
    }
}
