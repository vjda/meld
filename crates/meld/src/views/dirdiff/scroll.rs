//! Restore the folder list's offset when iced rebuilds its widget state.

use iced::advanced::layout::{self, Layout};
use iced::advanced::widget::{Id, Operation, Tree, operation::scrollable, tree};
use iced::advanced::{Clipboard, Shell, Widget, mouse, overlay, renderer};
use iced::{Element, Event, Length, Rectangle, Renderer, Size, Theme, Vector};

use super::Message;

pub(super) fn restore_on_mount<'a>(
    scroller: iced::widget::Scrollable<'a, Message>,
    id: Id,
    offset: f32,
) -> Element<'a, Message> {
    Element::new(RestoreOnMount {
        scroller: scroller.id(id.clone()),
        id,
        offset,
    })
}

struct RestoreOnMount<'a> {
    scroller: iced::widget::Scrollable<'a, Message>,
    id: Id,
    offset: f32,
}

#[derive(Default)]
struct Mounted(Option<Id>);

impl Widget<Message, Theme, Renderer> for RestoreOnMount<'_> {
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<Mounted>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(Mounted::default())
    }

    fn children(&self) -> Vec<Tree> {
        vec![Tree::new(
            &self.scroller as &dyn Widget<Message, Theme, Renderer>,
        )]
    }

    fn diff(&self, tree: &mut Tree) {
        if tree.state.downcast_ref::<Mounted>().0.as_ref() != Some(&self.id) {
            // The same position in the widget tree may now belong to another tab.
            tree.children = self.children();
        } else {
            tree.children[0].diff(&self.scroller as &dyn Widget<Message, Theme, Renderer>);
        }
    }

    fn size(&self) -> Size<Length> {
        self.scroller.size()
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        let child = &mut tree.children[0];
        let node = self.scroller.layout(child, renderer, limits);
        let mounted = tree.state.downcast_mut::<Mounted>();
        if mounted.0.as_ref() != Some(&self.id) {
            // Restore before drawing or handling events. On later layouts the
            // scroll widget keeps its live state, including active gestures.
            self.scroller.operate(
                child,
                Layout::new(&node),
                renderer,
                &mut scrollable::scroll_to(
                    self.id.clone(),
                    scrollable::AbsoluteOffset {
                        x: None,
                        y: Some(self.offset),
                    },
                ),
            );
            mounted.0 = Some(self.id.clone());
        }
        node
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
        self.scroller.update(
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
        self.scroller.draw(
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
        renderer: &Renderer,
        operation: &mut dyn Operation,
    ) {
        self.scroller
            .operate(&mut tree.children[0], layout, renderer, operation);
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &Renderer,
    ) -> mouse::Interaction {
        self.scroller
            .mouse_interaction(&tree.children[0], layout, cursor, viewport, renderer)
    }

    fn overlay<'a>(
        &'a mut self,
        tree: &'a mut Tree,
        layout: Layout<'a>,
        renderer: &Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'a, Message, Theme, Renderer>> {
        self.scroller.overlay(
            &mut tree.children[0],
            layout,
            renderer,
            viewport,
            translation,
        )
    }
}
