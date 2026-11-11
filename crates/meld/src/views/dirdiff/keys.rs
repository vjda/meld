//! A wrapper widget that turns key presses its content ignored into
//! messages, tracks modifier keys and reports right-click positions.
//!
//! Tab subscriptions run for every open tab, so keyboard navigation that
//! only the visible tab should see goes through the widget tree instead.

use iced::advanced::layout::{self, Layout};
use iced::advanced::widget::{Operation, Tree};
use iced::advanced::{Clipboard, Shell, Widget, mouse, overlay, renderer};
use iced::keyboard::{self, Key, Modifiers};
use iced::{Element, Event, Length, Point, Rectangle, Size, Vector};

/// Maps a key press to a message.
type KeyFn<'a, Message> = Box<dyn Fn(&Key, Modifiers) -> Option<Message> + 'a>;

pub struct KeyListener<'a, Message, Theme, Renderer> {
    content: Element<'a, Message, Theme, Renderer>,
    on_key: KeyFn<'a, Message>,
    on_modifiers: Option<Box<dyn Fn(Modifiers) -> Message + 'a>>,
    on_right_click: Option<Box<dyn Fn(Point) -> Message + 'a>>,
    intercept: Option<KeyFn<'a, Message>>,
}

impl<'a, Message, Theme, Renderer> KeyListener<'a, Message, Theme, Renderer> {
    pub fn new(
        content: impl Into<Element<'a, Message, Theme, Renderer>>,
        on_key: impl Fn(&Key, Modifiers) -> Option<Message> + 'a,
    ) -> Self {
        KeyListener {
            content: content.into(),
            on_key: Box::new(on_key),
            on_modifiers: None,
            on_right_click: None,
            intercept: None,
        }
    }

    /// Maps key presses to messages before the content sees them; keys
    /// it maps never reach the content (the search entry's Escape, Up
    /// and Down).
    pub fn intercept(mut self, f: impl Fn(&Key, Modifiers) -> Option<Message> + 'a) -> Self {
        self.intercept = Some(Box::new(f));
        self
    }

    pub fn on_modifiers(mut self, f: impl Fn(Modifiers) -> Message + 'a) -> Self {
        self.on_modifiers = Some(Box::new(f));
        self
    }

    /// Reports right clicks with the position inside this widget, after
    /// the content has handled them.
    pub fn on_right_click(mut self, f: impl Fn(Point) -> Message + 'a) -> Self {
        self.on_right_click = Some(Box::new(f));
        self
    }
}

impl<Message, Theme, Renderer> Widget<Message, Theme, Renderer>
    for KeyListener<'_, Message, Theme, Renderer>
where
    Renderer: renderer::Renderer,
{
    fn children(&self) -> Vec<Tree> {
        vec![Tree::new(&self.content)]
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(std::slice::from_ref(&self.content));
    }

    fn size(&self) -> Size<Length> {
        self.content.as_widget().size()
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        self.content
            .as_widget_mut()
            .layout(&mut tree.children[0], renderer, limits)
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn Operation,
    ) {
        self.content
            .as_widget_mut()
            .operate(&mut tree.children[0], layout, renderer, operation);
    }

    fn update(
        &mut self,
        tree: &mut Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        viewport: &Rectangle,
    ) {
        if let Event::Keyboard(keyboard::Event::KeyPressed { key, modifiers, .. }) = event
            && !shell.is_event_captured()
            && let Some(f) = &self.intercept
            && let Some(message) = f(key, *modifiers)
        {
            shell.publish(message);
            shell.capture_event();
            return;
        }
        let captured_before = shell.is_event_captured();
        self.content.as_widget_mut().update(
            &mut tree.children[0],
            event,
            layout,
            cursor,
            renderer,
            clipboard,
            shell,
            viewport,
        );
        match event {
            Event::Keyboard(keyboard::Event::ModifiersChanged(modifiers)) => {
                if let Some(f) = &self.on_modifiers {
                    shell.publish(f(*modifiers));
                }
            }
            Event::Keyboard(keyboard::Event::KeyPressed { key, modifiers, .. }) => {
                if !shell.is_event_captured()
                    && let Some(message) = (self.on_key)(key, *modifiers)
                {
                    shell.publish(message);
                    shell.capture_event();
                }
            }
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Right)) if !captured_before => {
                if let Some(f) = &self.on_right_click
                    && let Some(position) = cursor.position_in(layout.bounds())
                {
                    shell.publish(f(position));
                }
            }
            _ => {}
        }
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &Renderer,
    ) -> mouse::Interaction {
        self.content.as_widget().mouse_interaction(
            &tree.children[0],
            layout,
            cursor,
            viewport,
            renderer,
        )
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        self.content.as_widget().draw(
            &tree.children[0],
            renderer,
            theme,
            style,
            layout,
            cursor,
            viewport,
        );
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut Tree,
        layout: Layout<'b>,
        renderer: &Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'b, Message, Theme, Renderer>> {
        self.content.as_widget_mut().overlay(
            &mut tree.children[0],
            layout,
            renderer,
            viewport,
            translation,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::super::{Message, Nav, tree_key};
    use super::*;
    use iced::Theme;
    use iced::advanced::clipboard;
    use iced::keyboard::{Location, key};
    use iced::widget::{Space, text_input};

    fn press(
        content: Element<'_, Message, Theme, ()>,
        focus: bool,
        searching: bool,
        dialog_open: bool,
        modifiers: Modifiers,
    ) -> (Vec<Message>, bool) {
        let mut listener = KeyListener::new(content, move |key, modifiers| {
            tree_key(key, modifiers, searching, dialog_open)
        });
        let mut tree = Tree::new(&listener as &dyn Widget<Message, Theme, ()>);
        let size = Size::new(200.0, 100.0);
        let node = listener.layout(&mut tree, &(), &layout::Limits::new(Size::ZERO, size));
        let viewport = Rectangle::with_size(size);
        let cursor = mouse::Cursor::Available(Point::new(5.0, 5.0));
        let mut clipboard = clipboard::Null;
        let mut messages = Vec::new();
        if focus {
            let mut shell = Shell::new(&mut messages);
            listener.update(
                &mut tree,
                &Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
                Layout::new(&node),
                cursor,
                &(),
                &mut clipboard,
                &mut shell,
                &viewport,
            );
            let mut shell = Shell::new(&mut messages);
            listener.update(
                &mut tree,
                &Event::Keyboard(keyboard::Event::ModifiersChanged(modifiers)),
                Layout::new(&node),
                cursor,
                &(),
                &mut clipboard,
                &mut shell,
                &viewport,
            );
            messages.clear();
        }
        let key = Key::Named(key::Named::Backspace);
        let event = Event::Keyboard(keyboard::Event::KeyPressed {
            key: key.clone(),
            modified_key: key,
            physical_key: key::Physical::Code(key::Code::Backspace),
            location: Location::Standard,
            modifiers,
            text: None,
            repeat: false,
        });
        let mut shell = Shell::new(&mut messages);
        listener.update(
            &mut tree,
            &event,
            Layout::new(&node),
            cursor,
            &(),
            &mut clipboard,
            &mut shell,
            &viewport,
        );
        let captured = shell.is_event_captured();
        (messages, captured)
    }

    #[test]
    fn mac_shortcut_regression_keypressed_routes_and_respects_focused_input() {
        let blank = || Element::from(Space::new().width(100).height(30));
        let (messages, captured) = press(blank(), false, false, false, Modifiers::empty());
        assert!(captured && matches!(messages.as_slice(), [Message::Nav(Nav::Parent, false)]));
        if cfg!(target_os = "macos") {
            let (messages, captured) = press(blank(), false, false, false, Modifiers::COMMAND);
            assert!(captured && matches!(messages.as_slice(), [Message::Delete]));
            for (searching, dialog_open) in [(true, false), (false, true)] {
                let (messages, captured) =
                    press(blank(), false, searching, dialog_open, Modifiers::COMMAND);
                assert!(!captured && messages.is_empty());
            }
            let input = text_input("", "abc")
                .width(100)
                .on_input(Message::SearchInput);
            let (messages, captured) = press(input.into(), true, true, false, Modifiers::COMMAND);
            assert!(captured);
            assert!(
                !messages
                    .iter()
                    .any(|message| matches!(message, Message::Delete))
            );
        }
    }
}

impl<'a, Message, Theme, Renderer> From<KeyListener<'a, Message, Theme, Renderer>>
    for Element<'a, Message, Theme, Renderer>
where
    Message: 'a,
    Theme: 'a,
    Renderer: renderer::Renderer + 'a,
{
    fn from(widget: KeyListener<'a, Message, Theme, Renderer>) -> Self {
        Element::new(widget)
    }
}
