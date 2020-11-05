// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Markdown parsing demo

use kas::class::HasStr;
use kas::event::{Manager, Response, VoidMsg};
use kas::macros::make_widget;
use kas::text::format::Markdown;
use kas::widget::{EditBox, EditBoxVoid, Label, ScrollRegion, TextButton, Window};

fn main() -> Result<(), kas_wgpu::Error> {
    env_logger::init();

    let doc = r"Markdown document
================

Markdown supports *italic* and **bold** highlighting, ***both***, even with*in* w**o**rds.
As an extension, it also supports ~~strikethrough~~.

Inline `code = 2;` is supported. Code blocks are supported:
```
let x = 1;
let y = x + 1;
```

Markdown supports explicit line breaks —  
via two trailing spaces.  
It also supports lists:

1.  First item
2.  Second item

-   Unenumerated item
-   Another item
";

    let window = Window::new(
        "Markdown parser",
        make_widget! {
            #[layout(grid)]
            #[handler(msg = VoidMsg)]
            struct {
                #[widget(row=0, col=0, rspan=2)] editor: EditBoxVoid = EditBox::new(doc).multi_line(true),
                #[widget(row=0, col=1)] label: ScrollRegion<Label<Markdown>> = ScrollRegion::new(Label::new(Markdown::new(doc))).with_bars(false, true),
                #[widget(row=1, col=1, handler=update)] _ = TextButton::new("&Update", ()),
            }
            impl {
                fn update(&mut self, mgr: &mut Manager, _: ()) -> Response<VoidMsg> {
                    let text = Markdown::new(self.editor.get_str());
                    // TODO: this should update the size requirements of the inner area
                    *mgr += self.label.inner_mut().set_text(text);
                    Response::None
                }
            }
        },
    );

    let theme = kas_theme::FlatTheme::new();
    let mut toolkit = kas_wgpu::Toolkit::new(theme)?;
    toolkit.add(window)?;
    toolkit.run()
}
