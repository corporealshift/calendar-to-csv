use chrono::DateTime;
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

impl Month {
    fn to_str(&self) -> String {
        match self {
            Month::January => "01",
            Month::February => "02",
            Month::March => "03",
            Month::April => "04",
            Month::May => "05",
            Month::June => "06",
            Month::July => "07",
            Month::August => "08",
            Month::September => "09",
            Month::October => "10",
            Month::November => "11",
            Month::December => "12",
        }
        .to_owned()
    }

    fn end_day(&self) -> String {
        let year = 2024;
        let month_num: u32 = match self {
            Month::January => 1,
            Month::February => 2,
            Month::March => 3,
            Month::April => 4,
            Month::May => 5,
            Month::June => 6,
            Month::July => 7,
            Month::August => 8,
            Month::September => 9,
            Month::October => 10,
            Month::November => 11,
            Month::December => 12,
        };
        let date = chrono::NaiveDate::from_ymd_opt(year, month_num, 1)
            .unwrap_or(chrono::NaiveDate::from_ymd(year + 1, 1, 1))
            .pred();
        date.format("%d").to_string()
    }
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
    waiting_for_events: bool,
    events: Vec<Event>,
    loaded_events: bool,
    month: Option<Month>,
}

impl eframe::App for MainScreen {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        match self.receiver.try_recv() {
            Ok(msg) => match msg {
                APIMessage::OauthURL(url) => self.oauth_url = url,
                APIMessage::AuthToken(token) => self.auth_key = token,
                APIMessage::Events(events) => {
                    self.events = events;
                    self.loaded_events = true;
                    self.waiting_for_events = false;
                }
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

                    if let Some(month) = self.month.clone() {
                        if !self.waiting_for_events && !self.loaded_events {
                            self.calendar_api
                                .dispatch_events_request(self.auth_key.clone(), month);
                            self.waiting_for_events = true;
                        }
                    }
                }
            });
            ui.add_space(4.0);
        });
        egui::CentralPanel::default().show(ctx, |ui| {
            if !self.auth_key.is_empty() {
                ui.label("Select a calendar and month to get started");
            }
            if self.loaded_events {
                self.events.iter().for_each(|event| {
                    println!("Event: {:?}", event);
                    ui.label(event.summary.clone().unwrap_or("".to_owned()));
                    ui.label(event.description.clone().unwrap_or("".to_owned()));
                    ui.label(
                        event
                            .start
                            .clone()
                            .map(|st| st.date_time)
                            .flatten()
                            .map(|dt| dt.to_string())
                            .unwrap_or("".to_owned()),
                    );
                    ui.label(
                        event
                            .end
                            .clone()
                            .map(|st| st.date_time)
                            .flatten()
                            .map(|dt| dt.to_string())
                            .unwrap_or("".to_owned()),
                    );
                    ui.label(event.color_id.clone().unwrap_or("".to_owned()));

                    ui.separator();
                })
            }
        });
    }
}

impl MainScreen {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let (sender, receiver) = std::sync::mpsc::channel::<APIMessage>();
        let calendar_api = CalendarAPI { sender };
        let async_api = calendar_api.clone();
        let start = CalendarAPI::start_date(&Month::March);
        let end = CalendarAPI::end_date(&Month::March);
        println!("S: {}, E: {}", start, end);

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
            waiting_for_events: false,
            events: vec![],
            loaded_events: false,
            month: None,
        }
    }
}

enum APIMessage {
    AuthToken(String),
    OauthURL(String),
    Events(Vec<Event>),
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

    fn dispatch_events_request(&self, access_key: String, month: Month) {
        let rt = Runtime::new().unwrap();
        let client = self.clone();
        std::thread::spawn(move || {
            rt.block_on(async {
                client.get_events(access_key, month).await;
            });
        });
    }

    async fn get_events(&self, access_key: String, month: Month) {
        let client = Client::new(access_key).unwrap();
        //let client = EventClient::new(client);
        let cal_client = CalendarListClient::new(client.clone());
        let calendars = cal_client.list().await.unwrap();
        println!("Calendars {:?}", calendars);
        if let Some(calendar) = calendars.first() {
            println!("Getting events for {:?}", month);
            let now = chrono::Local::now();
            let event_client = EventClient::new(client);
            let list = event_client
                .list(
                    calendar.id.clone(),
                    CalendarAPI::start_date(&month),
                    CalendarAPI::end_date(&month),
                )
                .await
                .unwrap();
            println!("Events received, {:?}", list);
            self.sender.send(APIMessage::Events(list));
        }
    }

    fn start_date(month: &Month) -> DateTime<chrono::Local> {
        let month_str = format!("2024-{}-01T00:00:00-05:00", month.to_str());
        println!("Month str: {}", month_str.as_str());
        let parsed = DateTime::parse_from_rfc3339(month_str.as_str()).unwrap();
        DateTime::from(parsed)
    }
    fn end_date(month: &Month) -> DateTime<chrono::Local> {
        println!("End date: {}", month.end_day());
        let month_str = format!("2024-{}-{}T00:00:00-05:00", month.to_str(), month.end_day());
        DateTime::from(DateTime::parse_from_rfc3339(month_str.as_str()).unwrap())
    }
}
