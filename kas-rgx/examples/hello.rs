// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Hello world example

use kas::dialog::{action_close, MessageBox};
use kas::text::Label;

fn main() -> Result<(), winit::error::OsError> {
    // Build widgets.
    // Message is a Window with an "Ok" button and notification status.
    // Each Window::new method creates objects then solves constraints.
    let window = MessageBox::new(
        /*Notify::Info,*/
        Label::from("Hello world"),
        action_close,
    );

    let mut toolkit = kas_rgx::Toolkit::new();
    toolkit.add(window)?;
    toolkit.run()
}