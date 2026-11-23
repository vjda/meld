use super::*;
use crate::views::dirdiff::{DirDiff, Message as FolderMessage};
use iced::advanced::layout::{Layout, Limits};
use iced::advanced::renderer::Headless;
use iced::advanced::widget::{Id, Operation, Tree, operation::Scrollable};
use iced::advanced::{Shell, clipboard, mouse};
use iced::{Font, Pixels, Point, Rectangle, Renderer, Vector};

const SIZE: Size = Size::new(820.0, 430.0);
const OFFSET: f32 = 384.0;

#[derive(Default)]
struct Scroller(Option<(Rectangle, f32)>);

impl Operation for Scroller {
    fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn Operation)) {
        operate(self);
    }

    fn scrollable(
        &mut self,
        _id: Option<&Id>,
        bounds: Rectangle,
        content: Rectangle,
        translation: Vector,
        _state: &mut dyn Scrollable,
    ) {
        if content.height > bounds.height {
            assert!(self.0.replace((bounds, translation.y)).is_none());
        }
    }
}

struct Window {
    tree: Tree,
    renderer: Renderer,
}

impl Window {
    fn new() -> Self {
        Self {
            tree: Tree::empty(),
            renderer: iced::futures::executor::block_on(<Renderer as Headless>::new(
                Font::DEFAULT,
                Pixels(16.0),
                Some("tiny-skia"),
            ))
            .expect("software renderer"),
        }
    }

    fn frame(
        &mut self,
        app: &Meld,
        input: Option<(Point, mouse::Event)>,
    ) -> (Scroller, Vec<Message>) {
        let mut view = app.view();
        let widget = view.as_widget_mut();
        self.tree.diff(&*widget);
        let node = widget.layout(
            &mut self.tree,
            &self.renderer,
            &Limits::new(Size::ZERO, SIZE),
        );
        let layout = Layout::new(&node);
        let mut scroller = Scroller::default();
        widget.operate(&mut self.tree, layout, &self.renderer, &mut scroller);
        let mut messages = Vec::new();
        if let Some((point, event)) = input {
            widget.update(
                &mut self.tree,
                &iced::Event::Mouse(event),
                layout,
                mouse::Cursor::Available(point),
                &self.renderer,
                &mut clipboard::Null,
                &mut Shell::new(&mut messages),
                &Rectangle::with_size(SIZE),
            );
        }
        (scroller, messages)
    }

    fn scroller(&mut self, app: &Meld) -> (Rectangle, f32) {
        let (scroller, _) = self.frame(app, None);
        scroller.0.expect("visible folder scroller")
    }

    fn send(&mut self, app: &mut Meld, point: Point, event: mouse::Event) -> Vec<Message> {
        let (_, messages) = self.frame(app, Some((point, event)));
        for message in messages.iter().cloned() {
            let _ = app.update(message);
        }
        messages
    }

    fn wheel(&mut self, app: &mut Meld, pixels: f32, expected: f32) {
        let point = self.row_point(app);
        let messages = self.send(
            app,
            point,
            mouse::Event::WheelScrolled {
                delta: mouse::ScrollDelta::Pixels { x: 0.0, y: pixels },
            },
        );
        assert!(matches!(
            messages.as_slice(),
            [Message::Tab(_, views::Message::Folder(FolderMessage::Scrolled(viewport)))]
                if viewport.absolute_offset().y == expected
        ));
        assert_eq!(self.scroller(app).1, expected);
    }

    fn row_point(&mut self, app: &Meld) -> Point {
        let (bounds, _) = self.scroller(app);
        Point::new(bounds.x + 100.0, bounds.y + 42.0)
    }
}

fn folder_window() -> (tempfile::TempDir, Meld, Window) {
    let dir = tempfile::tempdir().unwrap();
    let roots = vec![dir.path().join("left"), dir.path().join("right")];
    for (pane, root) in roots.iter().enumerate() {
        std::fs::create_dir(root).unwrap();
        for i in 0..80 {
            std::fs::write(
                root.join(format!("row-{i:02}.txt")),
                format!("Pane {pane}\n"),
            )
            .unwrap();
        }
    }
    let (mut app, _) = Meld::boot();
    app.shared.recent = meld_config::RecentComparisons::load_from(dir.path().join("recent.toml"));
    let folder = DirDiff::populated_for_scroll_test(roots, &mut app.shared);
    let _ = app.add_tab(Tab::Folder(Box::new(folder)), Task::none(), None, false);
    (dir, app, Window::new())
}

#[test]
fn folder_scroll_right_click_keeps_the_offset() {
    let (_dir, mut app, mut window) = folder_window();
    window.wheel(&mut app, -OFFSET, OFFSET);
    let point = window.row_point(&app);
    let right_press = mouse::Event::ButtonPressed(mouse::Button::Right);
    let messages = window.send(&mut app, point, right_press.clone());
    assert!(matches!(
        messages.as_slice(),
        [
            Message::Tab(
                _,
                views::Message::Folder(FolderMessage::RowRightPressed(0, _))
            ),
            Message::Tab(_, views::Message::Folder(FolderMessage::ContextMenu(_)))
        ]
    ));
    assert_eq!(window.scroller(&app).1, OFFSET, "after opening the menu");

    let messages = window.send(&mut app, Point::new(4.0, SIZE.height - 4.0), right_press);
    assert!(matches!(
        messages.as_slice(),
        [Message::Tab(
            _,
            views::Message::Folder(FolderMessage::ClosePopup)
        )]
    ));
    assert_eq!(window.scroller(&app).1, OFFSET, "after dismissing the menu");
}

#[test]
fn folder_scroll_closing_file_comparison_restores_the_offset() {
    let (_dir, mut app, mut window) = folder_window();
    let folder = app.active.unwrap();
    window.wheel(&mut app, -OFFSET, OFFSET);
    let point = window.row_point(&app);
    for _ in 0..2 {
        let messages = window.send(
            &mut app,
            point,
            mouse::Event::ButtonPressed(mouse::Button::Left),
        );
        assert!(matches!(
            messages.as_slice(),
            [Message::Tab(
                _,
                views::Message::Folder(FolderMessage::RowPressed(0, _))
            )]
        ));
        let _ = window.send(
            &mut app,
            point,
            mouse::Event::ButtonReleased(mouse::Button::Left),
        );
    }
    let file = app.active_tab().unwrap();
    let Tab::File(diff) = &file.tab else {
        panic!("double-click did not open a file comparison");
    };
    assert!(diff.pane_geometry(1).is_some());
    assert!(diff.pane_geometry(2).is_none());
    let file_id = file.id;
    let _ = window.frame(&app, None);
    let _ = app.update(Message::CloseTab(file_id));
    assert!(app.active_tab().is_some_and(|tab| tab.id == folder));
    assert_eq!(
        window.scroller(&app).1,
        OFFSET,
        "after closing the file tab"
    );
}

#[test]
fn folder_scroll_wheel_still_moves_the_list_in_both_directions() {
    let (_dir, mut app, mut window) = folder_window();
    assert_eq!(window.scroller(&app).1, 0.0);
    window.wheel(&mut app, -OFFSET, OFFSET);
    window.wheel(&mut app, 48.0, OFFSET - 48.0);
}
