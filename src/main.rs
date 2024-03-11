use eframe::egui;
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
    loaded_cals: bool,
    calendars: Vec<String>,
    selected_calendar: String,
    loaded_events: bool,
    month: Option<Month>,
}

impl eframe::App for MainScreen {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if (!self.loaded_cals) {
            // load calendars via google API
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
                egui::ComboBox::from_label("Select Which Calendar")
                    .selected_text(self.selected_calendar.clone())
                    .show_ui(ui, |ui| {
                        ui.style_mut().wrap = Some(false);
                        ui.set_min_width(60.0);
                        ui.selectable_value(
                            &mut self.selected_calendar,
                            "first".to_owned(),
                            "First",
                        );
                        ui.selectable_value(
                            &mut self.selected_calendar,
                            "second".to_owned(),
                            "Second",
                        );
                        ui.selectable_value(
                            &mut self.selected_calendar,
                            "third".to_owned(),
                            "Third",
                        );
                    });
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
                        ui.selectable_value(&mut self.month, Some(Month::September), "September");
                        ui.selectable_value(&mut self.month, Some(Month::October), "October");
                        ui.selectable_value(&mut self.month, Some(Month::November), "November");
                        ui.selectable_value(&mut self.month, Some(Month::December), "December");
                    });
            });
            ui.add_space(4.0);
        });
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.label("Select a calendar and month to get started");
        });
    }
}

impl MainScreen {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        Self {
            loaded_cals: false,
            calendars: vec![],
            selected_calendar: "".to_owned(),
            loaded_events: false,
            month: None,
        }
    }
}
