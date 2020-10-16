// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Clock example

extern crate chrono;

use chrono::prelude::*;
use log::info;
use std::f32::consts::PI;
use std::time::Duration;

use kas::draw::{Colour, DrawRounded, DrawText};
use kas::geom::{Quad, Vec2};
use kas::text::util::set_text_and_prepare;
use kas::widget::Window;
use kas::{event, prelude::*};
use kas_wgpu::draw::DrawWindow;

#[handler(handle=noauto)]
#[widget(config = noauto)]
#[derive(Clone, Debug, kas :: macros :: Widget)]
struct Clock {
    #[widget_core]
    core: kas::CoreData,
    date_pos: Coord,
    time_pos: Coord,
    now: DateTime<Local>,
    date: Text<String>,
    time: Text<String>,
}

impl Layout for Clock {
    fn size_rules(&mut self, _: &mut dyn SizeHandle, _: AxisInfo) -> SizeRules {
        // We want a square shape and can resize freely. Numbers are arbitrary.
        SizeRules::new(100, 200, (0, 0), StretchPolicy::HighUtility)
    }

    #[inline]
    fn set_rect(&mut self, rect: Rect, _align: AlignHints) {
        // Force to square
        let size = rect.size.0.min(rect.size.1);
        let size = Size::uniform(size);
        let excess = rect.size - size;
        let pos = rect.pos + (excess * 0.5);
        self.core.rect = Rect { pos, size };

        // Note: font size is calculated as dpp * pt_size with units pixels/em.
        // We leave dpp at its default 96/72 and set pt_size based on pixels.
        // Dimensions are still dependent on fonts.
        let pt_size = (size.1 as f32 * 0.09).into();
        let half_size = Size(size.0, size.1 / 2);
        self.date.update_env(|env| {
            env.set_pt_size(pt_size);
            env.set_bounds(half_size.into());
        });
        self.time.update_env(|env| {
            env.set_pt_size(pt_size);
            env.set_bounds(half_size.into());
        });
        self.date_pos = pos + Size(0, size.1 - half_size.1);
        self.time_pos = pos;
    }

    fn draw(&self, draw_handle: &mut dyn DrawHandle, _: &ManagerState, _: bool) {
        let col_face = Colour::grey(0.4);
        let col_hands = Colour::new(0.2, 0.2, 0.4);
        let col_secs = Colour::new(0.6, 0.2, 0.2);
        let col_text = Colour::grey(0.0);

        // We use the low-level draw device to draw our clock. This means it is
        // not themeable, but gives us much more flexible draw routines.
        //
        // Note: offset is used for scroll-regions, and should be zero here;
        // we add it anyway as is recommended.
        let (pass, offset, draw) = draw_handle.draw_device();
        let draw = draw.as_any_mut().downcast_mut::<DrawWindow<()>>().unwrap();

        let rect = Quad::from(self.core.rect + offset);
        draw.circle(pass, rect, 0.95, col_face);

        let half = (rect.b.1 - rect.a.1) / 2.0;
        let centre = rect.a + half;

        let mut line_seg = |t: f32, r1: f32, r2: f32, w, col| {
            let v = Vec2(t.sin(), -t.cos());
            draw.rounded_line(pass, centre + v * r1, centre + v * r2, w, col);
        };

        let w = half * 0.015625;
        let l = w * 5.0;
        let r = half - w;
        for d in 0..12 {
            let l = if d % 3 == 0 { 2.0 * l } else { l };
            line_seg(d as f32 * (PI / 6.0), r - l, r, w, col_face);
        }

        let secs = self.now.time().num_seconds_from_midnight();
        let a_sec = (secs % 60) as f32 * (PI / 30.0);
        let a_min = (secs % 3600) as f32 * (PI / 1800.0);
        let a_hour = (secs % (12 * 3600)) as f32 * (PI / (12.0 * 1800.0));

        line_seg(a_hour, 0.0, half * 0.55, half * 0.03, col_hands);
        line_seg(a_min, 0.0, half * 0.8, half * 0.015, col_hands);
        line_seg(a_sec, 0.0, half * 0.9, half * 0.005, col_secs);

        let date_pos = (self.date_pos + offset).into();
        let time_pos = (self.time_pos + offset).into();
        draw.text(pass, date_pos, Vec2::ZERO, col_text, self.date.as_ref());
        draw.text(pass, time_pos, Vec2::ZERO, col_text, self.time.as_ref());
    }
}

impl WidgetConfig for Clock {
    fn configure(&mut self, mgr: &mut Manager) {
        mgr.update_on_timer(Duration::new(0, 0), self.id());
    }
}

impl Handler for Clock {
    type Msg = event::VoidMsg;

    #[inline]
    fn handle(&mut self, mgr: &mut Manager, event: Event) -> Response<Self::Msg> {
        match event {
            Event::TimerUpdate => {
                self.now = Local::now();
                let date = self.now.format("%Y-%m-%d").to_string();
                let time = self.now.format("%H:%M:%S").to_string();
                *mgr += set_text_and_prepare(&mut self.date, date)
                    + set_text_and_prepare(&mut self.time, time);
                let ns = 1_000_000_000 - (self.now.time().nanosecond() % 1_000_000_000);
                info!("Requesting update in {}ns", ns);
                mgr.update_on_timer(Duration::new(0, ns), self.id());
                Response::None
            }
            event => Response::Unhandled(event),
        }
    }
}

impl Clock {
    fn new() -> Self {
        let env = kas::text::Environment {
            halign: Align::Centre,
            valign: Align::Centre,
            ..Default::default()
        };
        let date = Text::new(env.clone(), "0000-00-00".into());
        let time = Text::new(env, "00:00:00".into());
        Clock {
            core: Default::default(),
            date_pos: Coord::ZERO,
            time_pos: Coord::ZERO,
            now: Local::now(),
            date,
            time,
        }
    }
}

fn main() -> Result<(), kas_wgpu::Error> {
    env_logger::init();

    let window = Window::new("Clock", Clock::new());

    let theme = kas_theme::FlatTheme::new();
    let mut toolkit = kas_wgpu::Toolkit::new(theme)?;
    toolkit.add(window)?;
    toolkit.run()
}
