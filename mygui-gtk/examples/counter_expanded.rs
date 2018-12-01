//! Like counter example, but avoiding usage of make_widget
#![feature(unrestricted_attribute_tokens)]
#![feature(proc_macro_hygiene)]

use mygui::control::TextButton;
use mygui::display::Text;
use mygui::event::{Handler, NoResponse};
use mygui::macros::{NoResponse, Widget};
use mygui::{SimpleWindow, Toolkit, TkWidget, Class, CoreData, Widget};

#[derive(Debug, NoResponse)]
enum Message {
    None,
    Decr,
    Incr,
}

#[layout(horizontal)]
#[widget(class = Class::Container)]
#[handler(response = Message, generics = <>
        where D: Handler<Response = Message>, I: Handler<Response = Message>)]
#[derive(Debug, Widget)]
struct Buttons<D: Widget, I: Widget> {
    #[core] core: CoreData,
    #[widget] decr: D,
    #[widget] incr: I,
}

#[layout(vertical)]
#[widget(class = Class::Container)]
#[handler(response = NoResponse, generics = <> where B: Handler<Response = Message>)]
#[derive(Debug, Widget)]
struct Contents<B: Widget> {
    #[core] core: CoreData,
    #[widget] display: Text,
    #[widget(handler = handle_button)] buttons: B,
    counter: usize,
}

impl<B: Widget> Contents<B> {
    fn handle_button(&mut self, tk: &TkWidget, msg: Message) -> NoResponse {
        match msg {
            Message::None => (),
            Message::Decr => {
                self.counter = self.counter.saturating_sub(1);
                self.display.set_text(tk, &self.counter.to_string());
            }
            Message::Incr => {
                self.counter = self.counter.saturating_add(1);
                self.display.set_text(tk, &self.counter.to_string());
            }
        };
        NoResponse
    }
}


fn main() -> Result<(), mygui_gtk::Error> {
    let buttons = Buttons {
        core: CoreData::default(),
        decr: TextButton::new("−", || Message::Decr),
        incr: TextButton::new("+", || Message::Incr),
    };
    
    let contents = Contents {
        core: CoreData::default(),
        display: Text::from("0"),
        buttons: buttons,
        counter: 0,
    };
    
    let window = SimpleWindow::new(contents);

    let mut toolkit = mygui_gtk::Toolkit::new()?;
    toolkit.add(window);
    toolkit.main();
    Ok(())
}
