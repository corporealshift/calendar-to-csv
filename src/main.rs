use chrono::{DateTime, FixedOffset};
use eframe::egui;
use eframe::emath::Numeric;
use gcal::*;
use std::fs::File;
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
        let date_opt = chrono::NaiveDate::from_ymd_opt(year, month_num + 1, 1)
            .unwrap_or(
                chrono::NaiveDate::from_ymd_opt(year - 1, 1, 1)
                    .unwrap_or(chrono::NaiveDate::default()),
            )
            .pred_opt();
        if let Some(date) = date_opt {
            date.format("%d").to_string()
        } else {
            "28".to_owned()
        }
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

#[derive(Debug)]
struct CSVEvent {
    client: String,
    sub_client: String,
    description: String,
    date: String,
    hours: String,
    rate: String,
    total: String,
}
fn parse_date_string(date_string: Option<EventCalendarDate>) -> Option<DateTime<FixedOffset>> {
    date_string
        .map(|st| st.date_time)
        .flatten()
        .map(|dt| DateTime::parse_from_rfc3339(dt.as_str()).ok())
        .flatten()
}

impl CSVEvent {
    fn rate_from_color(color_id: String) -> f64 {
        match color_id.as_str() {
            "1" => 50.0,
            "2" => 40.0,
            "3" => 36.0,
            _ => 50.0,
        }
    }

    fn from_event(event: &Event) -> CSVEvent {
        let end = parse_date_string(event.end.clone());
        let start = parse_date_string(event.start.clone());
        let diff = end.zip(start).map(|(end, start)| end - start);

        let start_date = start.map(|dt| format!("{}", dt.format("%Y-%m-%d")));
        let diff: Option<f64> = diff.map(|diff| diff.num_minutes().to_f64() / 60.0);
        let summary = event.summary.clone().unwrap_or("".to_owned());
        let summary_parts = summary.split('-');
        let client: String = summary_parts.clone().nth(0).unwrap_or("").to_owned();
        let sub_client: String = summary_parts.clone().nth(1).unwrap_or("").to_owned();
        let rate: f64 = CSVEvent::rate_from_color(event.color_id.clone().unwrap_or("".to_owned()));
        let total: f64 = diff.map(|d| d * rate).unwrap_or(0.0);

        CSVEvent {
            client,
            sub_client,
            description: event.description.clone().unwrap_or("".to_owned()),
            date: start_date.unwrap_or("".to_owned()),
            hours: diff.unwrap_or(0.0).to_string(),
            rate: rate.to_string(),
            total: total.to_string(),
        }
    }
}

struct MainScreen {
    receiver: Receiver<APIMessage>,
    calendar_api: CalendarAPI,
    oauth_url: String,
    auth_key: String,
    waiting_for_events: bool,
    events: Vec<CSVEvent>,
    loaded_events: bool,
    year: String,
    month: Option<Month>,
}

impl eframe::App for MainScreen {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        match self.receiver.try_recv() {
            Ok(msg) => match msg {
                APIMessage::OauthURL(url) => self.oauth_url = url,
                APIMessage::AuthToken(token) => self.auth_key = token,
                APIMessage::Events(events) => {
                    self.events = events
                        .iter()
                        .filter(|e| e.color_id.is_some())
                        .map(|e| CSVEvent::from_event(e))
                        .collect();
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
                    ui.add(egui::TextEdit::singleline(&mut self.year).desired_width(100.0));
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
                        if !self.year.is_empty() {
                            if ui.button("Get Events").clicked() {
                                if !self.waiting_for_events {
                                    self.calendar_api.dispatch_events_request(
                                        self.auth_key.clone(),
                                        self.year.clone(),
                                        month,
                                    );
                                    self.loaded_events = false;
                                    self.waiting_for_events = true;
                                }
                            }
                            if self.loaded_events {
                                if ui.button("Generate CSV").clicked() {
                                    let month_str =
                                        self.month.clone().unwrap_or(Month::January).to_str();
                                    let maybe_file = File::create_new(format!(
                                        "{}-{}-invoice.csv",
                                        &self.year, month_str
                                    ));
                                    if let Ok(file) = maybe_file {
                                        let mut wtr = csv::Writer::from_writer(file);
                                        let headers = wtr.write_record(&[
                                            "Date",
                                            "Client",
                                            "Sub Client",
                                            "Num Hours",
                                            "Job",
                                            "Rate",
                                            "Total",
                                        ]);
                                        if let Ok(_) = headers {
                                            self.events.iter().for_each(|event| {
                                                match wtr.serialize((
                                                    &event.date,
                                                    &event.client,
                                                    &event.sub_client,
                                                    &event.hours,
                                                    &event.description,
                                                    &event.rate,
                                                    &event.total,
                                                )) {
                                                    Ok(_) => {}
                                                    Err(e) => {
                                                        ui.label(format!(
                                                    "An error occurred serializing an event: {}",
                                                    e
                                                ));
                                                    }
                                                }
                                            })
                                        }
                                    }
                                }
                            }
                        }
                    }
                    if self.waiting_for_events {
                        ui.label("Loading...");
                        ui.add(egui::widgets::Spinner::new());
                    }
                }
            });
            ui.add_space(4.0);
        });
        egui::CentralPanel::default().show(ctx, |ui| {
            egui::ScrollArea::both().show(ui, |ui| {
                if !self.auth_key.is_empty() && self.month.is_none() {
                    ui.label("Select a calendar and month to get started");
                }
                if self.loaded_events {
                    egui::Grid::new("events")
                        .num_columns(5)
                        .striped(true)
                        .spacing([4.0, 4.0])
                        .show(ui, |ui| {
                            ui.label("Date");
                            ui.label("Client");
                            ui.label("Clients client");
                            ui.label("Hours");
                            ui.label("Job");
                            ui.label("Rate");
                            ui.label("Total");
                            ui.end_row();
                            self.events.iter().for_each(|event| {
                                ui.label(&event.date);
                                ui.label(&event.client);
                                ui.label(&event.sub_client);
                                ui.label(&event.hours);
                                ui.label(&event.description);
                                ui.label(&event.rate);
                                ui.label(&event.total);
                                ui.end_row();
                            });
                        });
                }
            });
        });
    }
}

impl MainScreen {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
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
            waiting_for_events: false,
            events: vec![],
            loaded_events: false,
            month: None,
            year: "2024".to_owned(),
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
            if let Err(e) = self.sender.send(APIMessage::OauthURL(oauth_url)) {
                println!("An error occurred sending oauth url: {}", e);
            }
        }
        loop {
            let lock = state.lock().await;
            if lock.access_key.is_some() {
                let access_key = lock.access_key.clone().unwrap();
                println!("Key received!");
                if let Err(e) = self.sender.send(APIMessage::AuthToken(access_key)) {
                    println!("An error occurred sending the access_key: {}", e);
                }
                break;
            }
            println!("Waiting for auth token...");
            tokio::time::sleep(std::time::Duration::new(1, 0)).await;
        }
    }

    fn dispatch_events_request(&self, access_key: String, year: String, month: Month) {
        let rt = Runtime::new().unwrap();
        let client = self.clone();
        std::thread::spawn(move || {
            rt.block_on(async {
                client.get_events(access_key, year, month).await;
            });
        });
    }

    async fn get_events(&self, access_key: String, year: String, month: Month) {
        let client = Client::new(access_key).unwrap();
        //let client = EventClient::new(client);
        let cal_client = CalendarListClient::new(client.clone());
        let calendars = cal_client.list().await.unwrap();
        println!("Calendars {:?}", calendars);
        if let Some(calendar) = calendars.first() {
            println!("Getting events for {:?}", month);
            let event_client = EventClient::new(client);
            let list = event_client
                .list(
                    calendar.id.clone(),
                    CalendarAPI::start_date(&year, &month),
                    CalendarAPI::end_date(&year, &month),
                )
                .await
                .unwrap();
            println!("Events received, {:?}", list);
            if let Err(e) = self.sender.send(APIMessage::Events(list)) {
                println!("Error trying to send events message: {}", e);
            }
        }
    }

    fn start_date(year: &String, month: &Month) -> DateTime<chrono::Local> {
        let month_str = format!("{}-{}-01T00:00:00-05:00", year, month.to_str());
        println!("Month str: {}", month_str.as_str());
        let parsed = DateTime::parse_from_rfc3339(month_str.as_str()).unwrap();
        DateTime::from(parsed)
    }
    fn end_date(year: &String, month: &Month) -> DateTime<chrono::Local> {
        println!("End date: {}", month.end_day());
        let month_str = format!(
            "{}-{}-{}T00:00:00-05:00",
            year,
            month.to_str(),
            month.end_day()
        );
        DateTime::from(DateTime::parse_from_rfc3339(month_str.as_str()).unwrap())
    }
}
