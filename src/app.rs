
/// We derive Deserialize/Serialize so we can persist app state on shutdown.
#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)] // if we add new fields, give them default values when deserializing old state
pub struct TemplateApp {
    // Example stuff:
    label: String,

    #[serde(skip)] // This how you opt-out of serialization of a field
    value: f32,
}

impl Default for TemplateApp {
    fn default() -> Self {
        Self {
            // Example stuff:
            label: "Hello World!".to_owned(),
            value: 2.7,
        }
    }
}

impl TemplateApp {
    /// Called once before the first frame.
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // This is also where you can customize the look and feel of egui using
        // `cc.egui_ctx.set_visuals` and `cc.egui_ctx.set_fonts`.

        const FONT_SIZE: f32 = 18.0;
        let ctx = &cc.egui_ctx;
        let mut style = (*ctx.style()).clone();
        style.text_styles.get_mut(&egui::TextStyle::Button).unwrap().size = FONT_SIZE;
        style.text_styles.get_mut(&egui::TextStyle::Body).unwrap().size = FONT_SIZE;
        style.spacing.button_padding = egui::Vec2::new(20.0, 20.0);
        ctx.set_style(style);

        // Load previous app state (if any).
        // Note that you must enable the `persistence` feature for this to work.
        if let Some(storage) = cc.storage {
            return eframe::get_value(storage, eframe::APP_KEY).unwrap_or_default();
        }


        Default::default()
    }
}

fn subscript_num(n: usize) -> String {
    let mut s = String::new();
    for c in n.to_string().chars() {
        s.push(match c {
            '0' => '₀',
            '1' => '₁',
            '2' => '₂',
            '3' => '₃',
            '4' => '₄',
            '5' => '₅',
            '6' => '₆',
            '7' => '₇',
            '8' => '₈',
            '9' => '₉',
            _ => c,
        });
    }
    s
}

const BUTTON_SIZE: [f32; 2] = [80.0, 40.0];

impl eframe::App for TemplateApp {
    /// Called by the frame work to save state before shutdown.
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, self);
    }

    /// Called each time the UI needs repainting, which may be many times per second.
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Put your widgets into a `SidePanel`, `TopPanel`, `CentralPanel`, `Window` or `Area`.
        // For inspiration and more examples, go to https://emilk.github.io/egui

        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            // The top panel is often a good place for a menu bar:

            egui::menu::bar(ui, |ui| {
                // NOTE: no File->Quit on web pages!
                let is_web = cfg!(target_arch = "wasm32");
                if !is_web {
                    ui.menu_button("File", |ui| {
                        if ui.button("Quit").clicked() {
                            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                        }
                    });
                    ui.add_space(16.0);
                }
                ui.menu_button("Settings", |ui| {
                    // add a disabled button saying coming soon
                    ui.add_enabled(false, egui::Button::new("Coming soon"));
                });

                // Align dark mode buttons buttons on the top right
                ui.with_layout(egui::Layout::right_to_left(egui::Align::RIGHT), |ui| {
                    egui::widgets::global_dark_light_mode_buttons(ui)
                });
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            // center the heading
            ui.vertical_centered(|ui| {
                ui.heading("Fretboard");
            });

            ui.add_space(20.0);

            ui.horizontal(|ui| {
                ui.add_space(30.0);

                egui::ScrollArea::horizontal().show(ui, |ui| {
                    // Make a grid
                    egui::Grid::new("fretboard").show(ui, |ui| {
                        ui.style_mut().visuals.widgets.hovered.bg_fill = egui::Color32::DARK_GRAY;
                        const STRING_OCTAVES: [usize; 6] = [2, 2, 3, 3, 3, 4];

                        // utf8 characters need special font support :(
                        // const NOTES: [&str; 12] = ["C", "C♯", "D", "D♯", "E", "F",
                        //                            "F♯", "G", "G♯", "A", "A♯", "B"];
                        const NOTES: [&str; 12] = ["C", "C#", "D", "D#", "E", "F",
                                                   "F#", "G", "G#", "A", "A#", "B"];
                        const STRINGS: [usize; 6] = [4, 11, 7, 2, 9, 4];
                        const MAX_FRET: usize = 17;

                        // Add fretboard labels
                        for fret in 0..MAX_FRET {
                            let fret_label = match fret {
                                0 => "Open",
                                3 => "3",
                                5 => "5",
                                7 => "7",
                                9 => "9",
                                12 => "12",
                                15 => "15",
                                _ => "",
                            };
                            ui.add_sized(BUTTON_SIZE, egui::Label::new(fret_label));
                        }
                        ui.end_row();

                        // TODO: how to span a separator here across all columns without
                        // breaking the grid?
                        for _ in 0..MAX_FRET {
                            ui.separator();
                        }
                        ui.end_row();

                        // add a row of buttons for each of the 6 strings
                        for string in 0..6 {
                            for fret in 0..MAX_FRET {
                                // a button for each note
                                let note = (STRINGS[string] + fret) % 12;
                                let note_name = NOTES[note];
                                let octave = STRING_OCTAVES[string] + (fret / 12);
                                // TODO: make this a custom widget
                                let label = egui::SelectableLabel::new(false, note_name);
                                if ui.add_sized(BUTTON_SIZE, label).clicked() {
                                    let octave_subscript = subscript_num(octave);
                                    log::debug!("{}{}", note_name, octave_subscript);
                                };
                            }
                            ui.end_row();
                        }
                    });
                });

                ui.add_space(30.0);
            });


            // ui.add(egui::github_link_file!(
            //     "https://github.com/emilk/eframe_template/blob/master/",
            //     "Source code."
            // ));

            ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
                powered_by_egui_and_eframe(ui);
                egui::warn_if_debug_build(ui);
                ui.separator();
            });
        });
    }
}

fn powered_by_egui_and_eframe(ui: &mut egui::Ui) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 0.0;
        ui.label("Powered by ");
        ui.hyperlink_to("egui", "https://github.com/emilk/egui");
        ui.label(" and ");
        ui.hyperlink_to(
            "eframe",
            "https://github.com/emilk/egui/tree/master/crates/eframe",
        );
        ui.label(".");
    });
}
