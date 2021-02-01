// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! List view widget

use super::{Accessor, DefaultView, ViewWidget};
use kas::event::{CursorIcon, GrabMode};
use kas::layout::solve_size_rules;
use kas::prelude::*;
use kas::text::Range;
use kas::widget::{ScrollComponent, ScrollWidget};
use log::{debug, trace};
use std::convert::TryFrom;
use std::time::Instant;

// TODO: do we need to keep the A::Item: Default bound used by allocate?

/// List view widget
#[handler(send=noauto, msg=<W as Handler>::Msg)]
#[widget(children=noauto, config=noauto)]
#[derive(Clone, Debug, Widget)]
pub struct ListView<
    D: Directional,
    A: Accessor<usize>,
    W: ViewWidget<A::Item> = <<A as Accessor<usize>>::Item as DefaultView>::Widget,
> where
    A::Item: Default,
{
    first_id: WidgetId,
    #[widget_core]
    core: CoreData,
    data: A,
    widgets: Vec<W>,
    direction: D,
    data_range: Range,
    align_hints: AlignHints,
    ideal_visible: i32,
    child_size_min: i32,
    child_size_ideal: i32,
    child_inter_margin: i32,
    child_skip: i32,
    child_size: i32,
    scroll: ScrollComponent,
}

impl<D: Directional + Default, A: Accessor<usize>, W: ViewWidget<A::Item>> ListView<D, A, W>
where
    A::Item: Default,
{
    /// Construct a new instance
    ///
    /// This constructor is available where the direction is determined by the
    /// type: for `D: Directional + Default`. In other cases, use
    /// [`ListView::new_with_direction`].
    pub fn new(data: A) -> Self {
        ListView {
            first_id: Default::default(),
            core: Default::default(),
            data,
            widgets: Default::default(),
            direction: Default::default(),
            data_range: Range::from(0usize..0),
            align_hints: Default::default(),
            ideal_visible: 5,
            child_size_min: 0,
            child_size_ideal: 0,
            child_inter_margin: 0,
            child_skip: 0,
            child_size: 0,
            scroll: Default::default(),
        }
    }
}
impl<D: Directional, A: Accessor<usize>, W: ViewWidget<A::Item>> ListView<D, A, W>
where
    A::Item: Default,
{
    /// Construct a new instance with explicit direction
    pub fn new_with_direction(direction: D, data: A) -> Self {
        ListView {
            first_id: Default::default(),
            core: Default::default(),
            data,
            widgets: Default::default(),
            direction,
            data_range: Range::from(0usize..0),
            align_hints: Default::default(),
            ideal_visible: 5,
            child_size_min: 0,
            child_size_ideal: 0,
            child_inter_margin: 0,
            child_skip: 0,
            child_size: 0,
            scroll: Default::default(),
        }
    }

    /// Access the stored data
    pub fn data(&self) -> &A {
        &self.data
    }

    /// Mutably access the stored data
    ///
    /// It may be necessary to use [`ListView::update_view`] to update the view of this data.
    pub fn data_mut(&mut self) -> &mut A {
        &mut self.data
    }

    /// Manually trigger an update to handle changed data
    pub fn update_view(&mut self, mgr: &mut Manager) {
        self.data_range.end = self.data_range.start;
        self.update_widgets(mgr);
        // Force SET_SIZE so that scroll-bar wrappers get updated
        trace!("update_view triggers SET_SIZE");
        *mgr |= TkAction::SET_SIZE;
    }

    /// Get the direction of contents
    pub fn direction(&self) -> Direction {
        self.direction.as_direction()
    }

    /// Set the preferred number of items visible (inline)
    ///
    /// This affects the (ideal) size request and whether children are sized
    /// according to their ideal or minimum size but not the minimum size.
    pub fn with_num_visible(mut self, number: i32) -> Self {
        self.ideal_visible = number;
        self
    }

    fn update_widgets(&mut self, mgr: &mut Manager) {
        let time = Instant::now();
        // set_rect allocates enough widgets to view a page; we update widget-data allocations
        // TODO: we may wish to notify self.data of the range it should cache
        let w_len = self.widgets.len();
        let (old_start, old_end) = (self.data_range.start(), self.data_range.end());
        let offset = u64::conv(self.scroll_offset().extract(self.direction));
        let mut first_data = usize::conv(offset / u64::conv(self.child_skip));
        first_data = (first_data + w_len)
            .min(self.data.len())
            .saturating_sub(w_len);
        let data_range = first_data..(first_data + w_len).min(self.data.len());
        let (child_size, mut skip) = match self.direction.is_vertical() {
            false => (
                Size::new(self.child_size, self.rect().size.1),
                Offset(self.child_skip, 0),
            ),
            true => (
                Size::new(self.rect().size.0, self.child_size),
                Offset(0, self.child_skip),
            ),
        };
        let mut pos_start = self.core.rect.pos;
        if self.direction.is_reversed() {
            pos_start += skip * i32::conv(w_len - 1);
            skip = skip * -1;
        }
        let mut rect = Rect::new(pos_start, child_size);
        let mut action = TkAction::empty();
        for data_num in data_range.clone() {
            // HACK: self.widgets[0] is used in size_rules, which affects alignment, therefore we
            // always need to call set_rect on this widget. Fix by adjusting how text_bound works?
            let i = data_num % w_len;
            if i == 0 || (data_num < old_start || data_num >= old_end) {
                let w = &mut self.widgets[i];
                action |= w.set(self.data.get(data_num));
                rect.pos = pos_start + skip * i32::conv(data_num);
                w.set_rect(mgr, rect, self.align_hints);
            }
        }
        *mgr |= action;
        self.data_range = data_range.into();
        let dur = (Instant::now() - time).as_micros();
        trace!("ListView::update_widgets completed in {}μs", dur);
    }
}

impl<D: Directional, A: Accessor<usize>, W: ViewWidget<A::Item>> ScrollWidget for ListView<D, A, W>
where
    A::Item: Default,
{
    fn scroll_axes(&self, size: Size) -> (bool, bool) {
        // TODO: maybe we should support a scrollbar on the other axis?
        // We would need to report a fake min-child-size to enable scrolling.
        let item_min = self.child_size_min + self.child_inter_margin;
        let num = i32::conv(self.data.len());
        let min_size = (item_min * num - self.child_inter_margin).max(0);
        (
            self.direction.is_horizontal() && min_size > size.0,
            self.direction.is_vertical() && min_size > size.1,
        )
    }

    #[inline]
    fn max_scroll_offset(&self) -> Offset {
        self.scroll.max_offset()
    }

    #[inline]
    fn scroll_offset(&self) -> Offset {
        self.scroll.offset()
    }

    #[inline]
    fn set_scroll_offset(&mut self, mgr: &mut Manager, offset: Offset) -> Offset {
        *mgr |= self.scroll.set_offset(offset);
        self.update_widgets(mgr);
        self.scroll.offset()
    }
}

impl<D: Directional, A: Accessor<usize>, W: ViewWidget<A::Item>> WidgetChildren
    for ListView<D, A, W>
where
    A::Item: Default,
{
    #[inline]
    fn first_id(&self) -> WidgetId {
        self.first_id
    }
    fn record_first_id(&mut self, id: WidgetId) {
        self.first_id = id;
    }
    #[inline]
    fn len(&self) -> usize {
        self.widgets.len()
    }
    #[inline]
    fn get(&self, index: usize) -> Option<&dyn WidgetConfig> {
        self.widgets.get(index).map(|w| w.as_widget())
    }
    #[inline]
    fn get_mut(&mut self, index: usize) -> Option<&mut dyn WidgetConfig> {
        self.widgets.get_mut(index).map(|w| w.as_widget_mut())
    }
}

impl<D: Directional, A: Accessor<usize>, W: ViewWidget<A::Item>> WidgetConfig for ListView<D, A, W>
where
    A::Item: Default,
{
    fn configure(&mut self, mgr: &mut Manager) {
        if let Some(handle) = self.data.update_handle() {
            mgr.update_on_handle(handle, self.id());
        }
        mgr.register_nav_fallback(self.id());
    }
}

impl<D: Directional, A: Accessor<usize>, W: ViewWidget<A::Item>> Layout for ListView<D, A, W>
where
    A::Item: Default,
{
    fn size_rules(&mut self, size_handle: &mut dyn SizeHandle, axis: AxisInfo) -> SizeRules {
        if self.widgets.is_empty() {
            if self.data.len() > 0 {
                self.widgets.push(W::new(self.data.get(0)));
            } else {
                self.widgets.push(W::default());
            }
        }
        let mut rules = self.widgets[0].size_rules(size_handle, axis);
        if axis.is_vertical() == self.direction.is_vertical() {
            self.child_size_min = rules.min_size();
            self.child_size_ideal = rules.ideal_size();
            self.child_inter_margin = rules.margins_i32().0 + rules.margins_i32().1;
            rules.multiply_with_margin(2, self.ideal_visible);
            rules.set_stretch(rules.stretch().max(StretchPolicy::HighUtility));
        }
        rules
    }

    fn set_rect(&mut self, mgr: &mut Manager, rect: Rect, mut align: AlignHints) {
        self.core.rect = rect;

        let data_len = self.data.len();
        let data_len32 = i32::try_from(data_len).unwrap();
        let mut child_size = rect.size;
        let content_size;
        let skip;
        let num;
        if self.direction.is_horizontal() {
            if child_size.0 >= self.ideal_visible * self.child_size_ideal {
                child_size.0 = self.child_size_ideal;
            } else {
                child_size.0 = self.child_size_min;
            }
            self.child_size = child_size.0;
            skip = Offset(child_size.0 + self.child_inter_margin, 0);
            self.child_skip = skip.0;
            align.horiz = None;
            num = (rect.size.0 + skip.0 - 1) / skip.0 + 1;

            let full_width = (skip.0 * data_len32 - self.child_inter_margin).max(0);
            content_size = Size::new(full_width, child_size.1);
        } else {
            if child_size.1 >= self.ideal_visible * self.child_size_ideal {
                child_size.1 = self.child_size_ideal;
            } else {
                child_size.1 = self.child_size_min;
            }
            self.child_size = child_size.1;
            skip = Offset(0, child_size.1 + self.child_inter_margin);
            self.child_skip = skip.1;
            align.vert = None;
            num = (rect.size.1 + skip.1 - 1) / skip.1 + 1;

            let full_height = (skip.1 * data_len32 - self.child_inter_margin).max(0);
            content_size = Size::new(child_size.0, full_height);
        }

        self.align_hints = align;

        let old_num = self.widgets.len();
        let num = (usize::conv(num)).min(data_len);
        if num > old_num {
            debug!("allocating widgets (old len = {}, new = {})", old_num, num);
            *mgr |= TkAction::RECONFIGURE;
            self.widgets.reserve(num);
            mgr.size_handle(|size_handle| {
                for i in old_num..num {
                    let mut w = W::new(self.data.get(i));
                    // We must solve size rules on new widgets:
                    solve_size_rules(&mut w, size_handle, Some(child_size.0), Some(child_size.1));
                    self.widgets.push(w);
                }
            });
        } else if num + 64 <= old_num {
            // Free memory (rarely useful?)
            self.widgets.truncate(num);
        }
        *mgr |= self.scroll.set_sizes(rect.size, content_size);
        self.update_widgets(mgr);
    }

    fn spatial_range(&self) -> (usize, usize) {
        let last = WidgetChildren::len(self).wrapping_sub(1);
        match self.direction.is_reversed() {
            false => (0, last),
            true => (last, 0),
        }
    }

    fn find_id(&self, coord: Coord) -> Option<WidgetId> {
        if !self.rect().contains(coord) {
            return None;
        }

        for child in &self.widgets[..self.data_range.len()] {
            if let Some(id) = child.find_id(coord) {
                return Some(id);
            }
        }
        Some(self.id())
    }

    fn draw(&self, draw_handle: &mut dyn DrawHandle, mgr: &ManagerState, disabled: bool) {
        let disabled = disabled || self.is_disabled();
        let offset = self.scroll_offset();
        use kas::draw::ClipRegion::Scroll;
        draw_handle.clip_region(self.core.rect, offset, Scroll, &mut |draw_handle| {
            for child in &self.widgets[..self.data_range.len()] {
                child.draw(draw_handle, mgr, disabled)
            }
        });
    }
}

impl<D: Directional, A: Accessor<usize>, W: ViewWidget<A::Item>> SendEvent for ListView<D, A, W>
where
    A::Item: Default,
{
    fn send(&mut self, mgr: &mut Manager, id: WidgetId, event: Event) -> Response<Self::Msg> {
        if self.is_disabled() {
            return Response::Unhandled(event);
        }

        let event = if id < self.id() {
            let response = 'outer: loop {
                for child in &mut self.widgets[..self.data_range.len()] {
                    if id <= child.id() {
                        let event = self.scroll.offset_event(event);
                        break 'outer child.send(mgr, id, event);
                    }
                }
                return Response::Unhandled(event);
            };
            match response {
                Response::Unhandled(event) => event,
                Response::Focus(rect) => {
                    let (rect, action) = self.scroll.focus_rect(rect, self.core.rect);
                    *mgr |= action;
                    self.update_widgets(mgr);
                    return Response::Focus(rect);
                }
                r => return r,
            }
        } else {
            match event {
                Event::HandleUpdate { .. } => {
                    self.update_view(mgr);
                    return Response::None;
                }
                event => event,
            }
        };

        let id = self.id();
        let (action, response) =
            self.scroll
                .scroll_by_event(event, self.core.rect.size, |source, _, coord| {
                    if source.is_primary() {
                        let icon = Some(CursorIcon::Grabbing);
                        mgr.request_grab(id, source, coord, GrabMode::Grab, icon);
                    }
                });
        if !action.is_empty() {
            *mgr |= action;
            self.update_widgets(mgr);
            Response::Focus(self.rect())
        } else {
            response.void_into()
        }
    }
}
