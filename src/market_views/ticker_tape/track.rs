use crate::message::Message;

use iced::advanced::Renderer as _;
use iced::advanced::widget::tree;
use iced::advanced::{Clipboard, Layout, Shell, Widget, layout, mouse, overlay, renderer};
use iced::{Element, Event, Fill, Length, Point, Rectangle, Size, Theme, Vector};

use super::TICKER_TAPE_HEIGHT;

// ---------------------------------------------------------------------------
// Seamless Ticker Tape Track
// ---------------------------------------------------------------------------

/// Lays out ticker segments at their full width, including the off-screen copy
/// that follows the visible sequence. Standard rows clamp their children to the
/// viewport, which prevents that second copy from entering from the right.
pub(super) struct TickerTapeTrack<'a> {
    segments: Vec<Element<'a, Message>>,
    segment_widths: Vec<f32>,
    offset: f32,
}

impl<'a> TickerTapeTrack<'a> {
    pub(super) fn new(segments: Vec<(Element<'a, Message>, f32)>, offset: f32) -> Self {
        let (segments, segment_widths) = segments.into_iter().unzip();

        Self {
            segments,
            segment_widths,
            offset,
        }
    }
}

impl Widget<Message, Theme, iced::Renderer> for TickerTapeTrack<'_> {
    fn children(&self) -> Vec<tree::Tree> {
        self.segments.iter().map(tree::Tree::new).collect()
    }

    fn diff(&self, tree: &mut tree::Tree) {
        tree.diff_children(&self.segments);
    }

    fn size(&self) -> Size<Length> {
        Size::new(Fill, Length::Fixed(TICKER_TAPE_HEIGHT))
    }

    fn layout(
        &mut self,
        tree: &mut tree::Tree,
        renderer: &iced::Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        let size = limits.resolve(
            Fill,
            Length::Fixed(TICKER_TAPE_HEIGHT),
            Size::new(0.0, TICKER_TAPE_HEIGHT),
        );
        let origins = ticker_tape_segment_origins(&self.segment_widths, self.offset);
        let children = self
            .segments
            .iter_mut()
            .zip(&mut tree.children)
            .zip(self.segment_widths.iter().copied())
            .zip(origins)
            .map(|(((segment, tree), width), x)| {
                let child_size = Size::new(width, TICKER_TAPE_HEIGHT);
                segment
                    .as_widget_mut()
                    .layout(tree, renderer, &layout::Limits::new(child_size, child_size))
                    .move_to(Point::new(x, 0.0))
            })
            .collect();

        layout::Node::with_children(size, children)
    }

    fn operate(
        &mut self,
        tree: &mut tree::Tree,
        layout: Layout<'_>,
        renderer: &iced::Renderer,
        operation: &mut dyn iced::advanced::widget::Operation,
    ) {
        operation.container(None, layout.bounds());
        operation.traverse(&mut |operation| {
            self.segments
                .iter_mut()
                .zip(&mut tree.children)
                .zip(layout.children())
                .for_each(|((segment, tree), layout)| {
                    segment
                        .as_widget_mut()
                        .operate(tree, layout, renderer, operation);
                });
        });
    }

    fn update(
        &mut self,
        tree: &mut tree::Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &iced::Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        viewport: &Rectangle,
    ) {
        let Some(clipped_viewport) = layout.bounds().intersection(viewport) else {
            return;
        };
        let cursor = if cursor.is_over(clipped_viewport) {
            cursor
        } else {
            mouse::Cursor::Unavailable
        };

        for ((segment, tree), layout) in self
            .segments
            .iter_mut()
            .zip(&mut tree.children)
            .zip(layout.children())
        {
            segment.as_widget_mut().update(
                tree,
                event,
                layout,
                cursor,
                renderer,
                clipboard,
                shell,
                &clipped_viewport,
            );
        }
    }

    fn mouse_interaction(
        &self,
        tree: &tree::Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &iced::Renderer,
    ) -> mouse::Interaction {
        let Some(clipped_viewport) = layout.bounds().intersection(viewport) else {
            return mouse::Interaction::None;
        };
        if !cursor.is_over(clipped_viewport) {
            return mouse::Interaction::None;
        }

        self.segments
            .iter()
            .zip(&tree.children)
            .zip(layout.children())
            .filter(|(_, layout)| layout.bounds().intersects(&clipped_viewport))
            .map(|((segment, tree), layout)| {
                segment.as_widget().mouse_interaction(
                    tree,
                    layout,
                    cursor,
                    &clipped_viewport,
                    renderer,
                )
            })
            .max()
            .unwrap_or_default()
    }

    fn draw(
        &self,
        tree: &tree::Tree,
        renderer: &mut iced::Renderer,
        theme: &Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        let Some(clipped_viewport) = layout.bounds().intersection(viewport) else {
            return;
        };

        renderer.with_layer(clipped_viewport, |renderer| {
            for ((segment, tree), layout) in self
                .segments
                .iter()
                .zip(&tree.children)
                .zip(layout.children())
                .filter(|(_, layout)| layout.bounds().intersects(&clipped_viewport))
            {
                segment.as_widget().draw(
                    tree,
                    renderer,
                    theme,
                    style,
                    layout,
                    cursor,
                    &clipped_viewport,
                );
            }
        });
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut tree::Tree,
        layout: Layout<'b>,
        renderer: &iced::Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'b, Message, Theme, iced::Renderer>> {
        let clipped_viewport = layout.bounds().intersection(viewport)?;
        overlay::from_children(
            &mut self.segments,
            tree,
            layout,
            renderer,
            &clipped_viewport,
            translation,
        )
    }
}

impl<'a> From<TickerTapeTrack<'a>> for Element<'a, Message> {
    fn from(track: TickerTapeTrack<'a>) -> Self {
        Element::new(track)
    }
}

pub(super) fn ticker_tape_segment_origins(widths: &[f32], offset: f32) -> Vec<f32> {
    let mut x = -offset;

    widths
        .iter()
        .map(|width| {
            let origin = x;
            x += width;
            origin
        })
        .collect()
}
