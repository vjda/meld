//! The window-local shortcut editor and its draft state.

use iced::keyboard::{self, Key, key::Named};
use iced::widget::{
    button, column, container, mouse_area, opaque, row, scrollable, text, text_input,
};
use iced::{Element, Length};

use crate::command::{ACTIONS, Alternative, BindingError, Bindings, Chord, Context};
use crate::shared::ShortcutEditError;
use crate::ui;

#[derive(Default)]
pub struct Editor {
    query: String,
    expanded: Option<&'static str>,
    draft: Option<Draft>,
    feedback: Option<String>,
}

struct Draft {
    index: Option<usize>,
    chord: Option<Chord>,
}

#[derive(Debug, Clone)]
pub enum Message {
    Search(String),
    Toggle(&'static str),
    Record(Option<usize>),
    Captured(Chord),
    Unsupported,
    Save,
    Cancel,
    Remove(usize),
    Restore(usize),
    Reset,
    Close,
}

pub struct Edit {
    pub id: &'static str,
    pub alternatives: Option<Vec<Alternative>>,
}

pub fn search_id() -> iced::widget::Id {
    iced::widget::Id::new("shortcut-editor-search")
}

impl Editor {
    pub fn update(&mut self, message: Message, bindings: &Bindings) -> Option<Edit> {
        match message {
            Message::Search(query) => {
                self.query = query;
                self.expanded = None;
                self.clear_draft();
            }
            Message::Toggle(id) => {
                self.expanded = (self.expanded != Some(id)).then_some(id);
                self.clear_draft();
            }
            Message::Record(index) if self.expanded.is_some() => {
                self.draft = Some(Draft { index, chord: None });
                self.feedback = None;
            }
            Message::Captured(chord) if self.recording() => {
                self.draft.as_mut()?.chord = Some(chord);
                self.feedback = None;
            }
            Message::Unsupported if self.recording() => {
                self.feedback =
                    Some(tr!("That key cannot be recorded. Try another shortcut.").into());
            }
            Message::Cancel => self.clear_draft(),
            Message::Save | Message::Remove(_) | Message::Restore(_) | Message::Reset => {
                let id = self.expanded?;
                let action = ACTIONS.iter().find(|a| a.id == id)?;
                let mut values = bindings.alternatives(action);
                match message {
                    Message::Save => {
                        let draft = self.draft.as_ref()?;
                        let chord = Alternative::Custom(draft.chord.clone()?);
                        if let Some(index) = draft.index {
                            *values.get_mut(index)? = chord;
                        } else {
                            values.push(chord);
                        }
                    }
                    Message::Remove(index) if self.draft.is_none() && index < values.len() => {
                        values.remove(index);
                    }
                    Message::Restore(index) if self.draft.is_none() => {
                        let value = Alternative::Default(index);
                        if !action.defaults().contains(&value) || values.contains(&value) {
                            return None;
                        }
                        values.push(value);
                    }
                    Message::Reset if self.draft.is_none() => {
                        return Some(Edit {
                            id,
                            alternatives: None,
                        });
                    }
                    _ => return None,
                }
                return Some(Edit {
                    id,
                    alternatives: Some(values),
                });
            }
            _ => {}
        }
        None
    }

    fn clear_draft(&mut self) {
        self.draft = None;
        self.feedback = None;
    }

    pub fn finished(&mut self, result: Result<(), ShortcutEditError>) {
        match result {
            Ok(()) => self.clear_draft(),
            Err(error) => self.feedback = Some(edit_error(error)),
        }
    }

    pub fn recording(&self) -> bool {
        self.draft
            .as_ref()
            .is_some_and(|draft| draft.chord.is_none())
    }

    pub fn record_event(&self, event: &keyboard::Event) -> Option<Message> {
        if !self.recording() {
            return None;
        }
        let keyboard::Event::KeyPressed {
            key,
            modifiers,
            repeat: false,
            ..
        } = event
        else {
            return None;
        };
        match key {
            Key::Named(Named::Escape) => Some(Message::Cancel),
            Key::Named(
                Named::Shift
                | Named::Control
                | Named::Alt
                | Named::AltGraph
                | Named::Super
                | Named::Meta
                | Named::Hyper,
            ) => None,
            _ => Some(
                Chord::from_event(key, *modifiers).map_or(Message::Unsupported, Message::Captured),
            ),
        }
    }

    pub fn view(&self, bindings: &Bindings) -> Element<'_, Message> {
        let query = self.query.trim().to_lowercase();
        let mut sections = column![].spacing(14);
        let mut has_results = false;
        for group in ACTIONS.chunk_by(|a, b| (a.group)() == (b.group)()) {
            let heading = (group[0].group)();
            let mut list = column![].spacing(6);
            let mut group_matches = false;
            for action in group {
                let (id, label) = (action.id, (action.label)());
                let values = bindings.alternatives(action);
                let keys = values
                    .iter()
                    .filter_map(|a| action.label_for(a))
                    .collect::<Vec<_>>()
                    .join(", ");
                let customized = bindings.customized(id);
                let status = if values.is_empty() {
                    tr!("Unbound")
                } else if customized {
                    tr!("Customized")
                } else {
                    ""
                };
                let expanded = self.expanded == Some(id);
                if !expanded
                    && ![heading, label, keys.as_str(), status]
                        .iter()
                        .any(|s| s.to_lowercase().contains(&query))
                {
                    continue;
                }
                group_matches = true;
                let mut summary = column![text(label).size(14)];
                for detail in [&keys, status] {
                    if !detail.is_empty() {
                        summary =
                            summary.push(text(detail.to_string()).size(13).style(ui::dim_text));
                    }
                }
                list = list.push(
                    row![
                        summary.width(Length::Fill),
                        control(
                            if expanded { tr!("Done") } else { tr!("Edit") },
                            Message::Toggle(id)
                        )
                    ]
                    .spacing(8)
                    .align_y(iced::Center),
                );
                if !expanded {
                    continue;
                }
                let mut controls = column![].spacing(6);
                if let Some(draft) = &self.draft {
                    if let Some(chord) = &draft.chord {
                        controls = controls
                            .push(
                                text(tr!("New shortcut: {}", chord.label()))
                                    .size(14)
                                    .font(ui::bold()),
                            )
                            .push(
                                row![
                                    button(text(tr!("Save")).size(13)).on_press(Message::Save),
                                    control(tr!("Record again"), Message::Record(draft.index)),
                                    control(tr!("Cancel"), Message::Cancel),
                                ]
                                .spacing(6)
                                .wrap(),
                            );
                    } else {
                        controls = controls
                            .push(
                                text(tr!("Press a shortcut now. Escape cancels recording."))
                                    .size(14)
                                    .font(ui::bold()),
                            )
                            .push(control(tr!("Cancel"), Message::Cancel));
                    }
                } else {
                    for (i, value) in values.iter().enumerate() {
                        controls = controls.push(
                            row![
                                text(action.label_for(value).unwrap_or_default())
                                    .size(13)
                                    .width(Length::Fill),
                                control(tr!("Change"), Message::Record(Some(i))),
                                control(tr!("Remove"), Message::Remove(i)),
                            ]
                            .spacing(6)
                            .align_y(iced::Center),
                        );
                    }
                    let mut actions =
                        row![control(tr!("Add shortcut"), Message::Record(None))].spacing(6);
                    if customized {
                        actions = actions.push(control(tr!("Reset action"), Message::Reset));
                    }
                    controls = controls.push(actions.wrap());
                    for value in action
                        .defaults()
                        .into_iter()
                        .filter(|a| !values.contains(a))
                    {
                        if let Alternative::Default(index) = value {
                            controls = controls.push(control(
                                tr!("Restore {}", action.label_for(&value).unwrap_or_default()),
                                Message::Restore(index),
                            ));
                        }
                    }
                }
                if let Some(feedback) = &self.feedback {
                    controls =
                        controls.push(text(feedback).size(13).style(|theme: &iced::Theme| {
                            text::Style {
                                color: Some(theme.extended_palette().danger.base.color),
                            }
                        }));
                }
                list = list.push(container(controls).width(Length::Fill).padding([6, 8]));
            }
            if group_matches {
                has_results = true;
                sections = sections
                    .push(column![text(heading).size(15).font(ui::bold()), list].spacing(4));
            }
        }
        if !has_results {
            sections = sections.push(text(tr!("No matching shortcuts")).style(ui::dim_text));
        }
        let header = container(if self.recording() {
            column![
                text(tr!("Keyboard Shortcuts")).size(18).font(ui::bold()),
                text(tr!("Recording… Press a shortcut. Escape cancels.")).size(14),
            ]
            .spacing(8)
        } else {
            column![
                text(tr!("Keyboard Shortcuts")).size(18).font(ui::bold()),
                row![
                    text_input(tr!("Search actions or shortcuts"), &self.query)
                        .id(search_id())
                        .size(14)
                        .on_input(Message::Search),
                    button(text(tr!("Clear")).size(13))
                        .style(button::secondary)
                        .on_press_maybe(
                            (!self.query.is_empty()).then(|| Message::Search(String::new()))
                        ),
                ]
                .spacing(6)
                .align_y(iced::Center),
            ]
            .spacing(8)
        })
        .padding([0, 19]);
        let footer = container(button(text(tr!("Close"))).on_press(Message::Close))
            .align_right(Length::Fill)
            .padding([0, 19]);
        // Preserve the full-width rail and separate text gutter from the T1 frame.
        let viewport = scrollable(container(sections).padding([0, 20]))
            .direction(scrollable::Direction::Vertical(
                scrollable::Scrollbar::new().spacing(0),
            ))
            .width(Length::Fill)
            .height(Length::Fill);
        let frame = container(column![header, viewport, footer].spacing(16))
            .width(600)
            .height(600)
            .padding([20, 1])
            .style(ui::popover);
        mouse_area(container(opaque(frame)).padding(16))
            .on_press(Message::Close)
            .into()
    }
}

fn control<'a>(label: impl Into<String>, message: Message) -> iced::widget::Button<'a, Message> {
    button(text(label.into()).size(13))
        .style(button::secondary)
        .on_press(message)
}

fn edit_error(error: ShortcutEditError) -> String {
    let scope = |context| match context {
        Context::New => tr!("New comparison tab"),
        Context::File => tr!("File comparison"),
        Context::Folder => tr!("Folder comparison"),
        Context::Vc => tr!("Version control comparison"),
        Context::Git => tr!("Git Compare"),
        Context::GitDiff => tr!("Git Compare diff"),
        Context::Image => tr!("Image comparison"),
    };
    match error {
        ShortcutEditError::Binding(BindingError::Conflict { action, context }) => {
            let label = ACTIONS
                .iter()
                .find(|a| a.id == action)
                .map_or(action, |a| (a.label)());
            tr!(
                "Already used by {} in {}. No changes saved.",
                label,
                scope(context)
            )
        }
        ShortcutEditError::Binding(BindingError::Reserved { context }) => {
            tr!(
                "Reserved for editing, navigation or dismissal in {}. No changes saved.",
                scope(context)
            )
        }
        ShortcutEditError::Binding(_) => tr!("Invalid shortcut. No changes saved.").into(),
        ShortcutEditError::Save(error) => {
            log::warn!("could not save shortcut: {error}");
            tr!("Could not save the shortcut. The previous bindings are still active.").into()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::command::Chord;
    use iced::keyboard::{
        Key, Location, Modifiers,
        key::{self, Named},
    };
    use meld_config::Settings;

    struct Fixture {
        editor: Editor,
        settings: Settings,
        bindings: Bindings,
        dir: tempfile::TempDir,
    }

    impl Fixture {
        fn new() -> Self {
            Self {
                editor: Editor::default(),
                settings: Settings::default(),
                bindings: Bindings::default(),
                dir: tempfile::tempdir().unwrap(),
            }
        }

        fn send(&mut self, message: Message) {
            if let Some(edit) = self.editor.update(message, &self.bindings) {
                let result = self
                    .bindings
                    .edited(edit.id, edit.alternatives)
                    .map_err(ShortcutEditError::Binding)
                    .and_then(|candidate| {
                        let mut next = self.settings.clone();
                        next.shortcuts = candidate.overrides();
                        next.save_to(self.dir.path().join("settings.toml"))
                            .map_err(ShortcutEditError::Save)?;
                        self.settings = next;
                        self.bindings = candidate;
                        Ok(())
                    });
                self.editor.finished(result);
            }
        }

        fn press(&mut self, key: Named, repeat: bool) {
            let event = keyboard::Event::KeyPressed {
                key: Key::Named(key),
                modified_key: Key::Named(key),
                physical_key: key::Physical::Unidentified(key::NativeCode::Unidentified),
                location: Location::Standard,
                modifiers: Modifiers::empty(),
                text: None,
                repeat,
            };
            if let Some(message) = self.editor.record_event(&event) {
                self.send(message);
            }
        }
    }

    #[test]
    fn shortcut_editor_escape_cancel_and_save() {
        let mut f = Fixture::new();
        f.send(Message::Toggle("help"));
        f.send(Message::Record(None));
        f.press(Named::Shift, false);
        f.press(Named::F13, false);
        f.press(Named::F6, true);
        f.send(Message::Save);
        assert!(f.editor.recording());
        assert!(f.settings.shortcuts.is_empty());
        f.press(Named::Escape, false);
        assert!(!f.editor.recording());
        assert_eq!(f.editor.expanded, Some("help"));
        assert!(f.settings.shortcuts.is_empty());

        f.send(Message::Record(None));
        f.press(Named::F6, false);
        f.send(Message::Cancel);
        f.send(Message::Save);
        assert!(!f.dir.path().join("settings.toml").exists());

        f.send(Message::Record(None));
        f.press(Named::F6, false);
        assert_eq!(f.bindings.labels("help").as_deref(), Some("F1"));
        f.send(Message::Save);
        assert_eq!(f.bindings.labels("help").as_deref(), Some("F1, F6"));
        let reloaded = Settings::load_from(f.dir.path().join("settings.toml"));
        assert_eq!(
            Bindings::load(&reloaded.shortcuts).overrides(),
            f.bindings.overrides()
        );
    }

    #[test]
    fn shortcut_editor_conflict_keeps_prior_binding_and_settings() {
        let mut f = Fixture::new();
        f.send(Message::Toggle("help"));
        f.send(Message::Record(Some(0)));
        f.press(Named::F6, false);
        f.send(Message::Save);
        let path = f.dir.path().join("settings.toml");
        let saved = std::fs::read(&path).unwrap();

        f.send(Message::Record(Some(0)));
        f.press(Named::F10, false);
        f.send(Message::Save);
        let error = f.editor.feedback.as_deref().unwrap();
        assert!(error.contains("Open menu") && error.contains("New comparison"));
        assert_eq!(f.bindings.labels("help").as_deref(), Some("F6"));
        assert_eq!(std::fs::read(&path).unwrap(), saved);

        f.send(Message::Record(Some(0)));
        f.press(Named::F7, false);
        f.send(Message::Save);
        assert_eq!(f.bindings.labels("help").as_deref(), Some("F7"));
        assert!(f.editor.feedback.is_none());
    }

    #[test]
    fn shortcut_editor_failed_save_keeps_draft_and_live_binding() {
        let mut f = Fixture::new();
        std::fs::create_dir(f.dir.path().join("settings.toml")).unwrap();
        f.send(Message::Toggle("help"));
        f.send(Message::Record(Some(0)));
        f.press(Named::F6, false);
        f.send(Message::Save);
        assert_eq!(f.bindings.labels("help").as_deref(), Some("F1"));
        assert!(f.settings.shortcuts.is_empty());
        assert_eq!(f.editor.draft.as_ref().unwrap().chord, Chord::parse("F6"));
        assert!(
            f.editor
                .feedback
                .as_deref()
                .unwrap()
                .contains("Could not save")
        );
    }

    #[test]
    fn shortcut_editor_conflicting_restore_keeps_action_unchanged() {
        let mut f = Fixture::new();
        f.send(Message::Toggle("refresh"));
        f.send(Message::Remove(1));
        f.send(Message::Toggle("help"));
        f.send(Message::Record(None));
        f.press(Named::F5, false);
        f.send(Message::Save);
        assert_eq!(f.bindings.labels("help").as_deref(), Some("F1, F5"));
        let prior = f.settings.shortcuts.clone();
        f.send(Message::Toggle("refresh"));
        f.send(Message::Restore(1));
        assert_eq!(f.settings.shortcuts, prior);
        assert_eq!(f.bindings.overrides(), prior);
        let error = f.editor.feedback.as_deref().unwrap();
        assert!(error.contains("Show help") && error.contains("File comparison"));
    }

    #[test]
    fn blank_comparison_shortcuts_follow_new_comparison_and_start_unbound() {
        let index = ACTIONS.iter().position(|a| a.id == "new-tab").unwrap();
        let ids = [
            "new-blank-file-2-way",
            "new-blank-file-3-way",
            "new-blank-folder-2-way",
            "new-blank-folder-3-way",
        ];
        let blank_actions = &ACTIONS[index + 1..][..ids.len()];
        let mut f = Fixture::new();

        for (action, id) in blank_actions.iter().zip(ids) {
            assert_eq!(action.id, id);
            assert_eq!((action.group)(), (ACTIONS[index].group)());
            assert!(f.bindings.alternatives(action).is_empty());

            f.send(Message::Toggle(id));
            f.send(Message::Record(None));
            f.press(Named::F6, false);
            f.send(Message::Save);
            let stored = Settings::load_from(f.dir.path().join("settings.toml"));
            assert_eq!(
                Bindings::load(&stored.shortcuts).labels(id).as_deref(),
                Some("F6")
            );

            f.send(Message::Reset);
            let stored = Settings::load_from(f.dir.path().join("settings.toml"));
            assert!(!stored.shortcuts.contains_key(id));
            assert_eq!(f.bindings.labels(id), None);
        }
    }

    #[test]
    fn shortcut_editor_restore_one_unbind_and_reset() {
        let mut f = Fixture::new();
        f.send(Message::Toggle("refresh"));
        f.send(Message::Remove(0));
        f.send(Message::Record(None));
        f.press(Named::F6, false);
        f.send(Message::Save);
        assert_eq!(f.bindings.labels("refresh").as_deref(), Some("F5, F6"));
        f.send(Message::Restore(0));
        let action = ACTIONS.iter().find(|a| a.id == "refresh").unwrap();
        assert_eq!(
            f.bindings.alternatives(action),
            vec![
                Alternative::Default(1),
                Alternative::Custom(Chord::parse("F6").unwrap()),
                Alternative::Default(0),
            ]
        );
        for index in [2, 1, 0] {
            f.send(Message::Remove(index));
        }
        assert_eq!(f.settings.shortcuts.get("refresh"), Some(&vec![]));
        assert_eq!(f.bindings.labels("refresh"), None);
        f.send(Message::Reset);
        assert_eq!(f.bindings.alternatives(action), action.defaults());
        assert!(!f.settings.shortcuts.contains_key("refresh"));
    }
}
