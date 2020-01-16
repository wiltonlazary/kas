// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! `Window` and `WindowList` types

use log::{debug, info, trace, warn};
use std::time::{Duration, Instant};

#[cfg(feature = "clipboard")]
use clipboard::ClipboardProvider;

use kas::event::Callback;
use kas::geom::{Coord, Rect, Size};
use kas::{event, theme, TkAction, WidgetId};
use winit::dpi::LogicalSize;
use winit::event::WindowEvent;

use crate::draw::DrawPipe;
use crate::{PendingAction, SharedState};

/// Per-window data
pub(crate) struct Window<TW> {
    widget: Box<dyn kas::Window>,
    ev_mgr: event::Manager,
    /// The winit window
    pub(crate) window: winit::window::Window,
    surface: wgpu::Surface,
    sc_desc: wgpu::SwapChainDescriptor,
    swap_chain: wgpu::SwapChain,
    draw_pipe: DrawPipe,
    timeouts: Vec<(usize, Instant, Option<Duration>)>,
    theme_window: TW,
}

// Public functions, for use by the toolkit
impl<TW: theme::Window<DrawPipe> + 'static> Window<TW> {
    /// Construct a window
    pub fn new<T: theme::Theme<DrawPipe, Window = TW>>(
        shared: &mut SharedState<T>,
        window: winit::window::Window,
        mut widget: Box<dyn kas::Window>,
    ) -> Self {
        let dpi_factor = window.hidpi_factor();
        let size: Size = window.inner_size().to_physical(dpi_factor).into();
        info!("Constucted new window with size {:?}", size);

        let surface = wgpu::Surface::create(&window);

        let sc_desc = wgpu::SwapChainDescriptor {
            usage: wgpu::TextureUsage::OUTPUT_ATTACHMENT,
            format: wgpu::TextureFormat::Bgra8UnormSrgb,
            width: size.0,
            height: size.1,
            present_mode: wgpu::PresentMode::Vsync,
        };
        let swap_chain = shared.device.create_swap_chain(&surface, &sc_desc);

        let mut draw_pipe = DrawPipe::new(&mut shared.device, sc_desc.format, size, &shared.theme);
        let mut theme_window = shared.theme.new_window(&mut draw_pipe, dpi_factor as f32);

        let mut ev_mgr = event::Manager::new(dpi_factor);
        ev_mgr.configure(widget.as_widget_mut());

        let mut size_handle = unsafe { theme_window.size_handle(&mut draw_pipe) };
        widget.resize(&mut size_handle, size);

        Window {
            widget,
            ev_mgr,
            window,
            surface,
            sc_desc,
            swap_chain,
            draw_pipe,
            timeouts: vec![],
            theme_window,
        }
    }

    /// Called by the `Toolkit` when the event loop starts to initialise
    /// windows. Optionally returns a callback time.
    pub fn init<T>(&mut self, shared: &mut SharedState<T>) -> Option<Instant> {
        self.window.request_redraw();

        let mut tk_window = TkWindow {
            action: TkAction::None,
            ev_mgr: &mut self.ev_mgr,
            shared,
        };

        for (i, condition) in self.widget.callbacks() {
            match condition {
                Callback::Start => {
                    self.widget.trigger_callback(i, &mut tk_window);
                }
                Callback::Repeat(dur) => {
                    self.widget.trigger_callback(i, &mut tk_window);
                    self.timeouts.push((i, Instant::now() + dur, Some(dur)));
                }
            }
        }

        self.next_resume()
    }

    /// Recompute layout of widgets and redraw
    pub fn reconfigure(&mut self) {
        let size = Size(self.sc_desc.width, self.sc_desc.height);
        debug!("Reconfiguring window (size = {:?})", size);

        self.ev_mgr.configure(self.widget.as_widget_mut());
        let mut size_handle = unsafe { self.theme_window.size_handle(&mut self.draw_pipe) };
        self.widget.resize(&mut size_handle, size);
        self.window.request_redraw();
    }

    /// Handle an event
    ///
    /// Return true to remove the window
    pub fn handle_event<T: theme::Theme<DrawPipe, Window = TW>>(
        &mut self,
        shared: &mut SharedState<T>,
        event: WindowEvent,
    ) -> TkAction {
        // Note: resize must be handled here to update self.swap_chain.
        match event {
            WindowEvent::Resized(size) => self.do_resize(shared, size),
            WindowEvent::RedrawRequested => self.do_draw(shared),
            WindowEvent::HiDpiFactorChanged(factor) => {
                self.theme_window.set_dpi_factor(factor as f32);
                self.ev_mgr.set_dpi_factor(factor);
                // Note: we rely on caller to resize widget
                self.do_resize(shared, self.window.inner_size());
            }
            event @ _ => {
                let mut tk_window = TkWindow {
                    action: TkAction::None,
                    ev_mgr: &mut self.ev_mgr,
                    shared,
                };
                event::Manager::handle_winit(&mut *self.widget, &mut tk_window, event);
                return tk_window.action;
            }
        }

        TkAction::None
    }

    pub(crate) fn timer_resume<T>(
        &mut self,
        shared: &mut SharedState<T>,
        instant: Instant,
    ) -> (TkAction, Option<Instant>) {
        let mut tk_window = TkWindow {
            action: TkAction::None,
            ev_mgr: &mut self.ev_mgr,
            shared,
        };

        // Iterate over loop, mutating some elements, removing others.
        let mut i = 0;
        while i < self.timeouts.len() {
            for timeout in &mut self.timeouts[i..] {
                if timeout.1 == instant {
                    self.widget.trigger_callback(timeout.0, &mut tk_window);
                    if let Some(dur) = timeout.2 {
                        while timeout.1 <= Instant::now() {
                            timeout.1 += dur;
                        }
                    } else {
                        break; // remove
                    }
                }
                i += 1;
            }
            if i < self.timeouts.len() {
                self.timeouts.remove(i);
            }
        }

        (tk_window.action, self.next_resume())
    }

    fn next_resume(&self) -> Option<Instant> {
        let mut next = None;
        for timeout in &self.timeouts {
            next = match next {
                None => Some(timeout.1),
                Some(t) => Some(t.min(timeout.1)),
            }
        }
        next
    }
}

// Internal functions
impl<TW: theme::Window<DrawPipe> + 'static> Window<TW> {
    fn do_resize<T: theme::Theme<DrawPipe, Window = TW>>(
        &mut self,
        shared: &mut SharedState<T>,
        size: LogicalSize,
    ) {
        let size = size.to_physical(self.window.hidpi_factor()).into();
        if size == Size(self.sc_desc.width, self.sc_desc.height) {
            return;
        }
        debug!("Resizing window to size={:?}", size);
        let mut size_handle = unsafe { self.theme_window.size_handle(&mut self.draw_pipe) };
        self.widget.resize(&mut size_handle, size);

        let buf = self.draw_pipe.resize(&shared.device, size);
        shared.queue.submit(&[buf]);

        self.sc_desc.width = size.0;
        self.sc_desc.height = size.1;
        self.swap_chain = shared
            .device
            .create_swap_chain(&self.surface, &self.sc_desc);
    }

    fn do_draw<T: theme::Theme<DrawPipe, Window = TW>>(&mut self, shared: &mut SharedState<T>) {
        trace!("Drawing window");
        let size = Size(self.sc_desc.width, self.sc_desc.height);
        let rect = Rect {
            pos: Coord::ZERO,
            size,
        };
        let frame = self.swap_chain.get_next_texture();
        let mut draw_handle = unsafe {
            shared
                .theme
                .draw_handle(&mut self.draw_pipe, &mut self.theme_window, rect)
        };
        self.widget.draw(&mut draw_handle, &self.ev_mgr);
        let clear_color = to_wgpu_color(shared.theme.clear_colour());
        let buf = self
            .draw_pipe
            .render(&mut shared.device, &frame.view, clear_color);
        shared.queue.submit(&[buf]);
    }
}

/// Implementation of [`kas::TkWindow`]
struct TkWindow<'a, T> {
    action: TkAction,
    ev_mgr: &'a mut event::Manager,
    shared: &'a mut SharedState<T>,
}

impl<'a, T> kas::TkWindow for TkWindow<'a, T> {
    fn add_window(&mut self, widget: Box<dyn kas::Window>) {
        // By far the simplest way to implement this is to let our call
        // anscestor, event::Loop::handle, do the work.
        //
        // In theory we could pass the EventLoopWindowTarget for *each* event
        // handled to create the winit window here or use statics to generate
        // errors now, but user code can't do much with this error anyway.
        self.shared.pending.push(PendingAction::AddWindow(widget));
    }

    fn data(&self) -> &event::Manager {
        &self.ev_mgr
    }

    fn update_data(&mut self, f: &mut dyn FnMut(&mut event::Manager) -> bool) {
        if f(&mut self.ev_mgr) {
            self.send_action(TkAction::Redraw);
        }
    }

    #[inline]
    fn redraw(&mut self, _id: WidgetId) {
        self.send_action(TkAction::Redraw);
    }

    #[inline]
    fn send_action(&mut self, action: TkAction) {
        self.action = self.action.max(action);
    }

    #[cfg(not(feature = "clipboard"))]
    #[inline]
    fn get_clipboard(&mut self) -> Option<String> {
        None
    }

    #[cfg(feature = "clipboard")]
    fn get_clipboard(&mut self) -> Option<String> {
        self.shared
            .clipboard
            .as_mut()
            .and_then(|cb| match cb.get_contents() {
                Ok(c) => Some(c),
                Err(e) => {
                    warn!("Failed to get clipboard contents: {:?}", e);
                    None
                }
            })
    }

    #[cfg(not(feature = "clipboard"))]
    #[inline]
    fn set_clipboard(&mut self, _content: String) {}

    #[cfg(feature = "clipboard")]
    fn set_clipboard(&mut self, content: String) {
        self.shared.clipboard.as_mut().map(|cb| {
            cb.set_contents(content)
                .unwrap_or_else(|e| warn!("Failed to set clipboard contents: {:?}", e))
        });
    }
}

fn to_wgpu_color(c: kas::draw::Colour) -> wgpu::Color {
    wgpu::Color {
        r: c.r as f64,
        g: c.g as f64,
        b: c.b as f64,
        a: c.a as f64,
    }
}
