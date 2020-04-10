// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Widget traits

use std::fmt;
use std::ops::DerefMut;

use crate::draw::{DrawHandle, InputState, SizeHandle};
use crate::event::{self, Manager, ManagerState};
use crate::geom::{Coord, Rect};
use crate::layout::{self, AxisInfo, SizeRules};
#[allow(unused)]
use crate::TkAction; // for doc links
use crate::{AlignHints, CoreData, Direction, WidgetId, WindowId};

mod impls;

/// Support trait for cloning boxed unsized objects
#[cfg_attr(not(feature = "internal_doc"), doc(hidden))]
pub trait CloneTo {
    unsafe fn clone_to(&self, out: *mut Self);
}

impl<T: Clone + Sized> CloneTo for T {
    unsafe fn clone_to(&self, out: *mut Self) {
        let x = self.clone();
        std::ptr::copy(&x, out, 1);
        std::mem::forget(x);
    }
}

/// Base widget functionality
///
/// This trait is almost always implemented via the
/// [`derive(Widget)` macro](macros/index.html#the-derivewidget-macro).
pub trait WidgetCore: fmt::Debug {
    /// Get direct access to the [`CoreData`] providing property storage.
    fn core_data(&self) -> &CoreData;

    /// Get mutable access to the [`CoreData`] providing property storage.
    ///
    /// This should not normally be needed by user code.
    #[cfg_attr(not(feature = "internal_doc"), doc(hidden))]
    fn core_data_mut(&mut self) -> &mut CoreData;

    /// Get the widget's numeric identifier
    #[inline]
    fn id(&self) -> WidgetId {
        self.core_data().id
    }

    /// Get whether the widget is disabled
    #[inline]
    fn is_disabled(&self) -> bool {
        self.core_data().disabled
    }

    /// Get the disabled state of a widget
    ///
    /// If disabled, a widget should not respond to input and should appear
    /// greyed out.
    ///
    /// The disabled status is inherited by children: events should not be
    /// passed to them, and they should also be drawn greyed out.
    #[inline]
    fn set_disabled(&mut self, disabled: bool) -> TkAction {
        self.core_data_mut().disabled = disabled;
        // Disabling affects find_id; we return RegionMoved to invalidate existing results
        TkAction::RegionMoved
    }

    /// Get the widget's region, relative to its parent.
    #[inline]
    fn rect(&self) -> Rect {
        self.core_data().rect
    }

    /// Get the name of the widget struct
    fn widget_name(&self) -> &'static str;

    /// Erase type
    fn as_widget(&self) -> &dyn WidgetConfig;
    /// Erase type
    fn as_widget_mut(&mut self) -> &mut dyn WidgetConfig;

    /// Construct [`InputState`]
    ///
    /// The `disabled` flag is inherited from parents. [`InputState::disabled`]
    /// will be true if either `disabled` or `self.is_disabled()` are true.
    ///
    /// The error state defaults to `false` since most widgets don't support
    /// this.
    fn input_state(&self, mgr: &ManagerState, disabled: bool) -> InputState {
        let id = self.core_data().id;
        InputState {
            disabled: self.core_data().disabled || disabled,
            error: false,
            hover: mgr.is_hovered(id),
            depress: mgr.is_depressed(id),
            nav_focus: mgr.nav_focus(id),
            char_focus: mgr.char_focus(id),
        }
    }
}

/// Listing of a widget's children
///
/// Usually this is implemented by `derive(Widget)`, but for dynamic widgets it
/// may have to be implemented manually. Note that if the results of these
/// methods ever change, one must send [`TkAction::Reconfigure`].
/// TODO: full reconfigure may be too slow; find a better option.
pub trait WidgetChildren: WidgetCore {
    /// Get the number of child widgets
    fn len(&self) -> usize;

    /// Get a reference to a child widget by index, or `None` if the index is
    /// out of bounds.
    ///
    /// For convenience, `Index<usize>` is implemented via this method.
    ///
    /// Required: `index < self.len()`.
    fn get(&self, index: usize) -> Option<&dyn WidgetConfig>;

    /// Mutable variant of get
    ///
    /// Warning: directly adjusting a widget without requiring reconfigure or
    /// redraw may break the UI. If a widget is replaced, a reconfigure **must**
    /// be requested. This can be done via [`Manager::send_action`].
    /// This method may be removed in the future.
    fn get_mut(&mut self, index: usize) -> Option<&mut dyn WidgetConfig>;

    /// Find a child widget by identifier
    ///
    /// This requires that the widget tree has already been configured by
    /// [`event::ManagerState::configure`].
    ///
    /// If the widget is disabled, this returns `None` without recursing children.
    fn find(&self, id: WidgetId) -> Option<&dyn WidgetConfig> {
        if self.is_disabled() {
            return None;
        }
        if id == self.id() {
            return Some(self.as_widget());
        } else if id > self.id() {
            return None;
        }

        for i in 0..self.len() {
            if let Some(w) = self.get(i) {
                if id > w.id() {
                    continue;
                }
                return w.find(id);
            }
            break;
        }
        None
    }

    /// Find a child widget by identifier
    ///
    /// This requires that the widget tree has already been configured by
    /// [`ManagerState::configure`].
    fn find_mut(&mut self, id: WidgetId) -> Option<&mut dyn WidgetConfig> {
        if id == self.id() {
            return Some(self.as_widget_mut());
        } else if id > self.id() {
            return None;
        }

        for i in 0..self.len() {
            if self.get(i).map(|w| id > w.id()).unwrap_or(true) {
                continue;
            }
            if let Some(w) = self.get_mut(i) {
                return w.find_mut(id);
            }
            break;
        }
        None
    }

    /// Walk through all widgets, calling `f` once on each.
    ///
    /// This walk is iterative (nonconcurrent), depth-first, and always calls
    /// `f` on self *after* walking through all children.
    fn walk(&self, f: &mut dyn FnMut(&dyn WidgetConfig)) {
        for i in 0..self.len() {
            if let Some(w) = self.get(i) {
                w.walk(f);
            }
        }
        f(self.as_widget());
    }

    /// Walk through all widgets, calling `f` once on each.
    ///
    /// This walk is iterative (nonconcurrent), depth-first, and always calls
    /// `f` on self *after* walking through all children.
    fn walk_mut(&mut self, f: &mut dyn FnMut(&mut dyn WidgetConfig)) {
        for i in 0..self.len() {
            if let Some(w) = self.get_mut(i) {
                w.walk_mut(f);
            }
        }
        f(self.as_widget_mut());
    }
}

/// Widget configuration
///
/// This trait allows some configuration of widget behaviour. All items have
/// default values. This trait may be implemented by hand, or may be derived
/// with the [`derive(Widget)` macro](macros/index.html#the-derivewidget-macro)
/// by use of a `#[widget_config]` attribute. Optionally, this attribute can
/// contain parameters, e.g. `#[widget_config(key_nav = true)]`.
// TODO(specialization): provide a blanket implementation, so that users only
// need implement manually when they have something to configure.
pub trait WidgetConfig: Layout {
    /// Configure widget
    ///
    /// Widgets are *configured* on window creation and when
    /// [`kas::TkAction::Reconfigure`] is sent.
    ///
    /// Configure is called before resizing (but after calculation of the
    /// initial window size). This method is called after
    /// a [`WidgetId`] has been assigned to self, and after `configure` has
    /// been called on each child.
    ///
    /// The default implementation of this method does nothing.
    fn configure(&mut self, _: &mut Manager) {}

    /// Is this widget navigable via Tab key?
    ///
    /// Defaults to `false`.
    fn key_nav(&self) -> bool {
        false
    }

    /// Which cursor icon should be used on hover?
    ///
    /// Defaults to [`event::CursorIcon::Default`].
    fn cursor_icon(&self) -> event::CursorIcon {
        event::CursorIcon::Default
    }
}

/// Positioning and drawing routines for widgets
///
/// This trait contains methods concerned with positioning of contents
/// as well as low-level event handling.
///
/// For a description of the widget size model, see [`SizeRules`].
pub trait Layout: WidgetChildren {
    /// Get size rules for the given axis.
    ///
    /// This method takes `&mut self` to allow local caching of child widget
    /// configuration for future `size_rules` and `set_rect` calls.
    ///
    /// Optionally, this method may set `self.rect().size` to the widget's ideal
    /// size for use by [`Layout::set_rect`] when setting alignment.
    ///
    /// If operating on one axis and the other is fixed, then the `other`
    /// parameter is used for the fixed dimension. Additionally, one may assume
    /// that `size_rules` has previously been called on the fixed axis with the
    /// current widget configuration.
    fn size_rules(&mut self, size_handle: &mut dyn SizeHandle, axis: AxisInfo) -> SizeRules;

    /// Adjust to the given size.
    ///
    /// For widgets with children, this is usually implemented via the derive
    /// [macro](kas::macros). For non-parent widgets which stretch to fill
    /// available space, the default implementation suffices. For non-parent
    /// widgets which react to alignment, this is a little more complex to
    /// implement, and can be done in one of two ways:
    ///
    /// 1.  Shrinking to ideal area and aligning within available space (e.g.
    ///     `CheckBoxBare` widget)
    /// 2.  Filling available space and applying alignment to contents (e.g.
    ///     `Label` widget)
    ///
    /// One may assume that `size_rules` has been called for each axis with the
    /// current widget configuration.
    #[inline]
    fn set_rect(&mut self, rect: Rect, _align: AlignHints) {
        self.core_data_mut().rect = rect;
    }

    /// Find a child widget by coordinate
    ///
    /// This is used by the event manager to target the correct widget given an
    /// event from a coordinate source (mouse pointer, touch event).
    /// Widgets may return their own Id over that of children in order to steal
    /// events (e.g. a button using an inner label widget).
    ///
    /// This must not be called before [`Layout::set_rect`].
    ///
    /// In the case of an empty grid cell, the parent widget is returned
    /// (same behaviour as with events addressed by coordinate).
    /// The only case `None` should be expected is when `coord` is outside the
    /// initial widget's region; however this is not guaranteed.
    ///
    /// Disabled widgets should return `None`, without recursing to children.
    #[inline]
    fn find_id(&self, _coord: Coord) -> Option<WidgetId> {
        if self.is_disabled() {
            return None;
        }
        Some(self.id())
    }

    /// Draw a widget
    ///
    /// This method is called to draw each visible widget (and should not
    /// attempt recursion on child widgets).
    fn draw(&self, draw_handle: &mut dyn DrawHandle, mgr: &ManagerState, disabled: bool);
}

/// Widget trait
///
/// This is one of a family of widget traits, all of which must be implemented
/// for a functional widget. In general, most traits will be implemented via the
/// [`derive(Widget)` macro](macros/index.html#the-derivewidget-macro).
///
/// A [`Widget`] may be passed into a generic function via
/// `fn foo<W: Widget>(w: &mut W)` or via
/// `fn foo<M>(w: &mut dyn Widget<Msg = M>)`, or, e.g.
/// `fn foo(w: &mut dyn WidgetConfig)` (note that `WidgetConfig` is the last unparameterised
/// trait in the widget trait family).
pub trait Widget: event::EventHandler {}

/// Trait to describe the type needed by the layout implementation.
///
/// To allow the `derive(Widget)` macro to implement [`Widget`], we use an
/// associated type to describe a data field of the following form:
/// ```none
/// #[layout_data] layout_data: <Self as kas::LayoutData>::Data,
/// ```
///
/// Ideally we would use an inherent associated type on the struct in question,
/// but until rust-lang#8995 is implemented that is not possible. We also cannot
/// place this associated type on the [`Widget`] trait itself, since then uses
/// of the trait would require parameterisation. Thus, this trait.
pub trait LayoutData {
    type Data: Clone + fmt::Debug + Default;
    type Solver: layout::RulesSolver;
    type Setter: layout::RulesSetter;
}

/// A pop-up is an overlay with parent & position information
pub struct Popup {
    pub parent: WidgetId,
    pub direction: Direction,
    pub overlay: Box<dyn Widget<Msg = event::VoidMsg>>,
}

/// Functionality required by a window
pub trait Window: Widget<Msg = event::VoidMsg> {
    /// Get the window title
    fn title(&self) -> &str;

    /// Whether to limit the maximum size of a window
    ///
    /// All widgets' size rules allow calculation of two sizes: the minimum
    /// size and the ideal size. Windows are initially sized to the ideal size.
    /// This option controls whether the window size is restricted by the
    /// calculated minimum size and by the ideal size.
    ///
    /// Return value is `(restrict_min, restrict_max)`. Suggested is to use
    /// `(true, true)` for simple dialog boxes and `(true, false)` for complex
    /// windows.
    fn restrict_dimensions(&self) -> (bool, bool);

    /// Add a pop-up as a layer in the current window
    ///
    /// Each [`Popup`] is assigned a [`WindowId`]; both are passed.
    fn add_popup(
        &mut self,
        size_handle: &mut dyn SizeHandle,
        mgr: &mut Manager,
        id: WindowId,
        popup: Popup,
    );

    /// Resize popups
    ///
    /// This is called immediately after [`Layout::set_rect`] to resize
    /// existing pop-ups.
    fn resize_popups(&mut self, size_handle: &mut dyn SizeHandle);

    /// Trigger closure of a pop-up
    ///
    /// If the given `id` refers to a pop-up, it should be closed.
    fn remove_popup(&mut self, mgr: &mut Manager, id: WindowId);

    /// Handle closure of self
    ///
    /// This allows for actions on destruction, but doesn't need to do anything.
    fn handle_closure(&mut self, _mgr: &mut Manager) {}
}

/// Return value of [`ThemeApi`] functions
///
/// This type is used to notify the toolkit of required updates.
#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd)]
pub enum ThemeAction {
    /// No action needed
    #[cfg_attr(not(feature = "internal_doc"), doc(hidden))]
    None,
    /// All windows require redrawing
    #[cfg_attr(not(feature = "internal_doc"), doc(hidden))]
    RedrawAll,
    /// Theme sizes have changed
    ///
    /// This implies that per-window theme data must be updated
    /// (via [`kas-theme::Theme::update_window`]) and all widgets resized.
    #[cfg_attr(not(feature = "internal_doc"), doc(hidden))]
    ThemeResize,
}

/// Interface through which a theme can be adjusted at run-time
///
/// All methods return a [`ThemeAction`] to enable correct action when a theme
/// is updated via [`Manager::adjust_theme`]. When adjusting a theme before
/// the UI is started, this return value can be safely ignored.
pub trait ThemeApi {
    /// Set font size. Default is 18. Units are unknown.
    fn set_font_size(&mut self, size: f32) -> ThemeAction;

    /// Change the colour scheme
    ///
    /// If no theme by this name is found, the theme is unchanged.
    // TODO: revise scheme identification and error handling?
    fn set_colours(&mut self, _scheme: &str) -> ThemeAction;

    /// Change the theme itself
    ///
    /// Themes may do nothing, or may react according to their own
    /// interpretation of this method.
    fn set_theme(&mut self, _theme: &str) -> ThemeAction {
        ThemeAction::None
    }
}

impl<T: ThemeApi> ThemeApi for Box<T> {
    fn set_font_size(&mut self, size: f32) -> ThemeAction {
        self.deref_mut().set_font_size(size)
    }
    fn set_colours(&mut self, scheme: &str) -> ThemeAction {
        self.deref_mut().set_colours(scheme)
    }
    fn set_theme(&mut self, theme: &str) -> ThemeAction {
        self.deref_mut().set_theme(theme)
    }
}
