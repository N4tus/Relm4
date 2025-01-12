use std::time::Duration;

use gtk::prelude::{BoxExt, ButtonExt, GtkWindowExt, OrientableExt};
use relm4::{gtk, Component, ComponentParts, ComponentSender, RelmApp, WidgetPlus};

struct AppModel {
    counter: u8,
}

#[derive(Debug)]
enum AppMsg {
    Increment,
    Decrement,
}

#[relm4::component]
impl Component for AppModel {
    type CommandOutput = AppMsg;
    type Init = ();
    type Input = ();
    type Output = ();
    type Widgets = AppWidgets;

    view! {
        gtk::Window {
            set_title: Some("Async Counter"),
            set_default_width: 300,
            set_default_height: 100,

            gtk::Box {
                set_orientation: gtk::Orientation::Vertical,
                set_margin_all: 5,
                set_spacing: 5,

                gtk::Button {
                    set_label: "Increment",
                    // Messages are fully async, no blocking!
                    connect_clicked[sender] => move |_| {
                        sender.oneshot_command(async move {
                            tokio::time::sleep(Duration::from_secs(1)).await;
                            AppMsg::Increment
                        })
                    },
                },

                gtk::Button::with_label("Decrement") {
                    connect_clicked[sender] => move |_| {
                        sender.oneshot_command(async move {
                            tokio::time::sleep(Duration::from_secs(1)).await;
                            AppMsg::Decrement
                        })
                    },
                },

                gtk::Label {
                    set_margin_all: 5,
                    #[watch]
                    set_label: &format!("Counter: {}", model.counter),
                },
            },
        }
    }

    fn init(_: (), root: &Self::Root, sender: ComponentSender<Self>) -> ComponentParts<Self> {
        let model = AppModel { counter: 0 };

        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update_cmd(&mut self, msg: Self::CommandOutput, _sender: ComponentSender<Self>) {
        match msg {
            AppMsg::Increment => {
                self.counter = self.counter.wrapping_add(1);
            }
            AppMsg::Decrement => {
                self.counter = self.counter.wrapping_sub(1);
            }
        }
    }
}

fn main() {
    let app = RelmApp::new("relm4.example.non_blocking_async");
    app.run::<AppModel>(());
}
