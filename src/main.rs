use eframe::egui;
use gcal::*;
use std::sync::mpsc::{Receiver, Sender};
use tokio::runtime::Runtime;
use tokio::sync::Mutex;
#[derive(Debug, PartialEq, Clone)]
enum Month {
    January,
    February,
    March,
    April,
    May,
    June,
    July,
    August,
    September,
    October,
    November,
    December,
}

fn main() -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([800.0, 600.0]),
        ..Default::default()
    };
    eframe::run_native(
        "Calendar to CSV",
        options,
        Box::new(|cc| Box::new(MainScreen::new(cc))),
    )
}

struct MainScreen {
    receiver: Receiver<APIMessage>,
    calendar_api: CalendarAPI,
    oauth_url: String,
    auth_key: String,
    loaded_cals: bool,
    calendars: Vec<String>,
    selected_calendar: String,
    loaded_events: bool,
    month: Option<Month>,
}

impl eframe::App for MainScreen {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        match self.receiver.try_recv() {
            Ok(msg) => match msg {
                APIMessage::OauthURL(url) => self.oauth_url = url,
                APIMessage::AuthToken(token) => self.auth_key = token,
            },
            _ => {}
        }
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("Quit").clicked() {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                });
            });
            ui.add_space(4.0);
            egui::widgets::global_dark_light_mode_buttons(ui);
            ui.add_space(8.0);

            ui.horizontal(|ui| {
                if self.auth_key.is_empty() && self.oauth_url.is_empty() {
                    ui.label("Waiting for oauth url to be generated...");
                }
                if !self.oauth_url.is_empty() && self.auth_key.is_empty() {
                    ui.label("Click here to log in: ");
                    ui.hyperlink(self.oauth_url.clone());
                }
                if !self.auth_key.is_empty() {
                    egui::ComboBox::from_label("Select Month")
                        .selected_text(
                            self.month
                                .clone()
                                .map(|m| format!("{m:?}"))
                                .unwrap_or("".to_owned()),
                        )
                        .show_ui(ui, |ui| {
                            ui.style_mut().wrap = Some(false);
                            ui.selectable_value(&mut self.month, Some(Month::January), "January");
                            ui.selectable_value(&mut self.month, Some(Month::February), "February");
                            ui.selectable_value(&mut self.month, Some(Month::March), "March");
                            ui.selectable_value(&mut self.month, Some(Month::April), "April");
                            ui.selectable_value(&mut self.month, Some(Month::May), "May");
                            ui.selectable_value(&mut self.month, Some(Month::June), "June");
                            ui.selectable_value(&mut self.month, Some(Month::July), "July");
                            ui.selectable_value(&mut self.month, Some(Month::August), "August");
                            ui.selectable_value(
                                &mut self.month,
                                Some(Month::September),
                                "September",
                            );
                            ui.selectable_value(&mut self.month, Some(Month::October), "October");
                            ui.selectable_value(&mut self.month, Some(Month::November), "November");
                            ui.selectable_value(&mut self.month, Some(Month::December), "December");
                        });
                }
            });
            ui.add_space(4.0);
        });
        egui::CentralPanel::default().show(ctx, |ui| {
            if !self.auth_key.is_empty() {
                ui.label("Select a calendar and month to get started");
            }
        });
    }
}

impl MainScreen {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let (sender, receiver) = std::sync::mpsc::channel::<APIMessage>();
        let calendar_api = CalendarAPI { sender };
        let async_api = calendar_api.clone();

        let rt = Runtime::new().unwrap();
        std::thread::spawn(move || {
            rt.block_on(async {
                async_api.get_auth_token().await;
            });
        });
        Self {
            receiver,
            calendar_api,
            oauth_url: "".to_owned(),
            auth_key: "".to_owned(),
            loaded_cals: false,
            calendars: vec![],
            selected_calendar: "".to_owned(),
            loaded_events: false,
            month: None,
        }
    }
}

enum APIMessage {
    AuthToken(String),
    OauthURL(String),
}

#[derive(Clone)]
struct CalendarAPI {
    sender: Sender<APIMessage>,
}

impl CalendarAPI {
    async fn get_auth_token(&self) {
        let client_id = "".to_owned();
        let client_secret = "".to_owned();
        let mut params = ClientParameters {
            client_id,
            client_secret,
            ..Default::default()
        };

        let state = State::new(Mutex::new(params.clone()));
        let maybe_host = oauth_listener(state.clone()).await;
        if let Ok(host) = maybe_host {
            params.redirect_url = Some(format!("http://{}", host));

            let oauth_url = oauth_user_url(params.clone());
            self.sender.send(APIMessage::OauthURL(oauth_url));
        }
        loop {
            let lock = state.lock().await;
            if lock.access_key.is_some() {
                let access_key = lock.access_key.clone().unwrap();
                println!("Key received!");
                self.sender.send(APIMessage::AuthToken(access_key));
                break;
            }
            println!("Waiting for auth token...");
            tokio::time::sleep(std::time::Duration::new(1, 0)).await;
        }
    }
}
