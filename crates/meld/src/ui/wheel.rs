//! Vertical mouse-wheel input for horizontal-only scrollables.
//!
//! A scrollable that only scrolls horizontally drops the vertical axis of a
//! wheel event, so the mouse wheel does nothing on it. This adapter hands a
//! vertical-only step to its child as the same step on the horizontal axis.
//! Shift keeps its meaning, and every other event reaches the child as is, so
//! capture, clamping and scroll notifications stay with the scrollable.

use iced::advanced::layout::{self, Layout};
use iced::advanced::widget::{Operation, Tree, tree};
use iced::advanced::{Clipboard, Shell, Widget, mouse, overlay, renderer};
use iced::{Element, Event, Length, Rectangle, Size, Vector, keyboard};

/// Wraps `content` so that a vertical wheel scrolls it horizontally.
pub fn horizontal<'a, M: 'a, T: 'a, R: renderer::Renderer + 'a>(
    content: impl Into<Element<'a, M, T, R>>,
) -> Element<'a, M, T, R> {
    Element::new(HorizontalWheel {
        content: content.into(),
    })
}

/// Whether Shift is held, as seen by this widget's events.
#[derive(Default)]
struct WheelState {
    shift: bool,
}

struct HorizontalWheel<'a, M, T, R> {
    content: Element<'a, M, T, R>,
}

impl<M, T, R: renderer::Renderer> Widget<M, T, R> for HorizontalWheel<'_, M, T, R> {
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<WheelState>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(WheelState::default())
    }

    fn children(&self) -> Vec<Tree> {
        vec![Tree::new(&self.content)]
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(std::slice::from_ref(&self.content));
    }

    fn size(&self) -> Size<Length> {
        self.content.as_widget().size()
    }

    fn layout(&mut self, tree: &mut Tree, renderer: &R, limits: &layout::Limits) -> layout::Node {
        self.content
            .as_widget_mut()
            .layout(&mut tree.children[0], renderer, limits)
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut R,
        theme: &T,
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

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &R,
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
        renderer: &R,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, M>,
        viewport: &Rectangle,
    ) {
        let state = tree.state.downcast_mut::<WheelState>();
        let mut mapped = None;
        match event {
            Event::Keyboard(keyboard::Event::ModifiersChanged(modifiers)) => {
                state.shift = modifiers.shift();
            }
            Event::Mouse(mouse::Event::WheelScrolled { delta }) if !state.shift => {
                mapped = vertical_as_horizontal(*delta)
                    .map(|delta| Event::Mouse(mouse::Event::WheelScrolled { delta }));
            }
            _ => {}
        }
        let event = mapped.as_ref().unwrap_or(event);
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
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &R,
    ) -> mouse::Interaction {
        self.content.as_widget().mouse_interaction(
            &tree.children[0],
            layout,
            cursor,
            viewport,
            renderer,
        )
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut Tree,
        layout: Layout<'b>,
        renderer: &R,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'b, M, T, R>> {
        self.content.as_widget_mut().overlay(
            &mut tree.children[0],
            layout,
            renderer,
            viewport,
            translation,
        )
    }
}

/// The horizontal step equal to a vertical-only wheel step, if there is one.
/// A step that already has a horizontal part passes through unchanged.
fn vertical_as_horizontal(delta: mouse::ScrollDelta) -> Option<mouse::ScrollDelta> {
    match delta {
        mouse::ScrollDelta::Lines { x, y } if x == 0.0 && y != 0.0 => {
            Some(mouse::ScrollDelta::Lines { x: y, y: 0.0 })
        }
        mouse::ScrollDelta::Pixels { x, y } if x == 0.0 && y != 0.0 => {
            Some(mouse::ScrollDelta::Pixels { x: y, y: 0.0 })
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use iced::advanced::layout::{self, Layout};
    use iced::advanced::widget::Tree;
    use iced::advanced::{Shell, clipboard, mouse};
    use iced::widget::{Space, scrollable};
    use iced::{Element, Event, Length, Point, Rectangle, Size, Theme, keyboard};

    use super::horizontal;

    /// The offset the strip publishes on each scroll.
    #[derive(Debug, Clone, Copy, PartialEq)]
    struct Offset {
        x: f32,
        y: f32,
    }

    /// A 200 px strip over 600 px of content, optionally wrapped.
    fn strip(wrapped: bool) -> Element<'static, Offset, Theme, ()> {
        let strip = scrollable(Space::new().width(600.0).height(30.0))
            .direction(scrollable::Direction::Horizontal(
                scrollable::Scrollbar::new(),
            ))
            .width(Length::Fixed(200.0))
            .on_scroll(|viewport| {
                let offset = viewport.absolute_offset();
                Offset {
                    x: offset.x,
                    y: offset.y,
                }
            });
        if wrapped {
            horizontal(strip)
        } else {
            strip.into()
        }
    }

    /// Plays `events` into a fresh strip, with the pointer over it or away
    /// from it, and returns the last offset it published.
    fn scroll(wrapped: bool, pointer_over_strip: bool, events: &[Event]) -> Option<Offset> {
        let mut element = strip(wrapped);
        let mut tree = Tree::new(element.as_widget());
        let size = Size::new(200.0, 30.0);
        let node =
            element
                .as_widget_mut()
                .layout(&mut tree, &(), &layout::Limits::new(Size::ZERO, size));
        let viewport = Rectangle::with_size(size);
        let cursor = mouse::Cursor::Available(if pointer_over_strip {
            Point::new(20.0, 15.0)
        } else {
            Point::new(500.0, 500.0)
        });
        let mut clipboard = clipboard::Null;
        let mut last = None;
        for event in events {
            let mut messages = Vec::new();
            {
                let mut shell = Shell::new(&mut messages);
                element.as_widget_mut().update(
                    &mut tree,
                    event,
                    Layout::new(&node),
                    cursor,
                    &(),
                    &mut clipboard,
                    &mut shell,
                    &viewport,
                );
            }
            last = messages.pop().or(last);
        }
        last
    }

    fn lines(x: f32, y: f32) -> Event {
        Event::Mouse(mouse::Event::WheelScrolled {
            delta: mouse::ScrollDelta::Lines { x, y },
        })
    }

    fn pixels(x: f32, y: f32) -> Event {
        Event::Mouse(mouse::Event::WheelScrolled {
            delta: mouse::ScrollDelta::Pixels { x, y },
        })
    }

    fn shift(pressed: bool) -> Event {
        let modifiers = if pressed {
            keyboard::Modifiers::SHIFT
        } else {
            keyboard::Modifiers::empty()
        };
        Event::Keyboard(keyboard::Event::ModifiersChanged(modifiers))
    }

    #[test]
    fn vertical_line_wheel_scrolls_the_strip_like_a_horizontal_one() {
        let horizontal_step = scroll(false, true, &[lines(-1.0, 0.0)]);
        let vertical_step = scroll(true, true, &[lines(0.0, -1.0)]);
        assert!(horizontal_step.is_some_and(|offset| offset.x != 0.0));
        assert_eq!(vertical_step, horizontal_step);
    }

    #[test]
    fn vertical_pixel_wheel_scrolls_the_strip_like_a_horizontal_one() {
        let horizontal_step = scroll(false, true, &[pixels(-10.0, 0.0)]);
        let vertical_step = scroll(true, true, &[pixels(0.0, -10.0)]);
        assert!(horizontal_step.is_some_and(|offset| offset.x != 0.0));
        assert_eq!(vertical_step, horizontal_step);
    }

    #[test]
    fn shift_leaves_the_vertical_wheel_to_the_scrollable() {
        let events = [shift(true), lines(0.0, -1.0)];
        assert_eq!(scroll(true, true, &events), scroll(false, true, &events));
    }

    #[test]
    fn releasing_shift_maps_the_vertical_wheel_again() {
        let mapped = scroll(true, true, &[shift(true), shift(false), lines(0.0, -1.0)]);
        let native = scroll(false, true, &[lines(-1.0, 0.0)]);
        assert_eq!(mapped, native);
    }

    #[test]
    fn diagonal_wheel_reaches_the_scrollable_unchanged() {
        let events = [lines(-1.0, -2.0)];
        assert_eq!(scroll(true, true, &events), scroll(false, true, &events));
    }

    #[test]
    fn wheel_away_from_the_strip_does_not_scroll_it() {
        assert_eq!(scroll(true, false, &[lines(0.0, -1.0)]), None);
    }

    #[test]
    fn vertical_wheel_stops_at_the_end_of_the_strip() {
        let vertical = vec![lines(0.0, -1.0); 50];
        let horizontal = vec![lines(-1.0, 0.0); 50];
        assert_eq!(
            scroll(true, true, &vertical),
            scroll(false, true, &horizontal)
        );
    }
}
