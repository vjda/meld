//! Modal dialogs drawn over the window content.

use iced::advanced::layout::{self, Layout};
use iced::advanced::widget::{Operation, Tree};
use iced::advanced::{Clipboard, Shell, Widget, mouse, overlay, renderer};
use iced::widget::{center, container, mouse_area, opaque, stack};
use iced::{Color, Element, Event, Length, Rectangle, Size, Theme, Vector, keyboard};

/// Shows `content` centred over a dimmed `base`. Clicking outside the
/// dialog sends `on_blur`.
pub fn modal<'a, M: Clone + 'a>(
    base: impl Into<Element<'a, M>>,
    content: impl Into<Element<'a, M>>,
    on_blur: M,
) -> Element<'a, M> {
    stack![
        base.into(),
        opaque(
            mouse_area(
                center(opaque(content)).style(|_theme: &Theme| container::Style {
                    background: Some(Color::BLACK.scale_alpha(0.45).into()),
                    ..container::Style::default()
                })
            )
            .on_press(on_blur)
        )
    ]
    .into()
}

/// Draw the background without giving its widgets focus, events or overlays.
pub fn inert<'a, M: 'a>(content: Element<'a, M>) -> Element<'a, M> {
    Element::new(InputLayer {
        content,
        input: Input::Inert,
    })
}

/// Let a dialog fence a keyboard event before any child receives it.
pub fn intercept_keyboard<'a, M: 'a>(
    content: Element<'a, M>,
    f: impl Fn(&keyboard::Event, &mut Shell<'_, M>) + 'a,
) -> Element<'a, M> {
    Element::new(InputLayer {
        content,
        input: Input::Intercept(Box::new(f)),
    })
}

type Intercept<'a, M> = Box<dyn Fn(&keyboard::Event, &mut Shell<'_, M>) + 'a>;

enum Input<'a, M> {
    Inert,
    Intercept(Intercept<'a, M>),
}

struct InputLayer<'a, M, T, R> {
    content: Element<'a, M, T, R>,
    input: Input<'a, M>,
}

impl<M, T, R: renderer::Renderer> Widget<M, T, R> for InputLayer<'_, M, T, R> {
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
            if matches!(self.input, Input::Inert) {
                mouse::Cursor::Unavailable
            } else {
                cursor
            },
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
        if !matches!(self.input, Input::Inert) {
            self.content.as_widget_mut().operate(
                &mut tree.children[0],
                layout,
                renderer,
                operation,
            );
        }
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
        let Input::Intercept(intercept) = &self.input else {
            return;
        };
        if let Event::Keyboard(event) = event {
            intercept(event, shell);
            if shell.is_event_captured() {
                return;
            }
        }
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
        if matches!(self.input, Input::Inert) {
            mouse::Interaction::default()
        } else {
            self.content.as_widget().mouse_interaction(
                &tree.children[0],
                layout,
                cursor,
                viewport,
                renderer,
            )
        }
    }

    fn overlay<'a>(
        &'a mut self,
        tree: &'a mut Tree,
        layout: Layout<'a>,
        renderer: &R,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'a, M, T, R>> {
        if matches!(self.input, Input::Inert) {
            None
        } else {
            self.content.as_widget_mut().overlay(
                &mut tree.children[0],
                layout,
                renderer,
                viewport,
                translation,
            )
        }
    }
}

/// The frame of a dialog: a title, a body and a row of buttons.
pub fn dialog<'a, M: Clone + 'a>(
    title: impl iced::widget::text::IntoFragment<'a>,
    body: impl Into<Element<'a, M>>,
    buttons: Vec<Element<'a, M>>,
) -> Element<'a, M> {
    use iced::widget::{column, row, space, text};

    let mut button_row = row![space::horizontal()].spacing(8);
    for b in buttons {
        button_row = button_row.push(b);
    }
    container(
        column![
            text(title).size(18).font(super::bold()),
            body.into(),
            button_row
        ]
        .spacing(16)
        .max_width(640),
    )
    .padding(20)
    .max_width(680)
    .style(super::popover)
    .into()
}
