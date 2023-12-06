use std::time::Duration;
use klib::core::base::{Playable, PlaybackHandle};

/// We derive Deserialize/Serialize so we can persist app state on shutdown.
#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)] // if we add new fields, give them default values when deserializing old state
pub struct TemplateApp {
    // Example stuff:
    label: String,

    #[serde(skip)] // This how you opt-out of serialization of a field
    value: f32,

    #[serde(skip)]
    playback_handle: Option<PlaybackHandle>,
}

impl Default for TemplateApp {
    fn default() -> Self {
        Self {
            // Example stuff:
            label: "Hello World!".to_owned(),
            value: 2.7,
            playback_handle: None,
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
        style.spacing.item_spacing = egui::Vec2::new(0.0, 0.0);
        ctx.set_style(style);

        // Load previous app state (if any).
        // Note that you must enable the `persistence` feature for this to work.
        if let Some(storage) = cc.storage {
            return eframe::get_value(storage, eframe::APP_KEY).unwrap_or_default();
        }


        Default::default()
    }
}
// TODO: add a font
// utf8 characters need special font support :(
// Remove unicode accidentals from note name, and turn numbers into subscripts
fn format_note_name(mut note: Note) -> String {
    use klib::core::interval::Interval;

    // convert flats to sharps
    let note_name = note.to_string();
    if note_name.contains('♭') {
        note = note + Interval::AugmentedSeventh - Interval::PerfectOctave;
    }

    let mut s = String::new();
    for c in note.to_string().chars() {
        s.push(match c {
            '♯' => '#',
            '♭' => 'b',
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

const BUTTON_SIZE: [f32; 2] = [50.0, 25.0];
const MAX_FRET: usize = 17;

use klib::core::note::HasNoteId;
use klib::core::note::Note;


fn fret_string(string: Note, fret: usize, selected: bool, playback_handle: &mut Option<PlaybackHandle>) -> impl egui::Widget + '_ {
    move |ui: &mut egui::Ui| {

        let note_id = string.id() << fret;
        let note = Note::from_id(note_id).unwrap();

        let note_name = format_note_name(note);

        let label = egui::SelectableLabel::new(selected, note_name);
        let response = ui.add_sized(BUTTON_SIZE, label);
        if response.clicked() {
            let dur = Duration::from_millis(500);
            // TODO: this crackles, just use the frequency and a different lib
            // to play sound?
            let ret =
                note.play(Duration::from_millis(0),
                           dur,
                           Duration::from_millis(0));

            match ret {
                Ok(h) => {
                    log::debug!("played note {}", note);
                    // Have to keep the handle around to play the sound.
                    *playback_handle = Some(h);
                },
                Err(e) => log::error!("error playing note: {}", e),
            }
        }

        response
    }

}

impl eframe::App for TemplateApp {
    /// Called by the frame work to save state before shutdown.
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, self);
    }

    /// Called each time the UI needs repainting, which may be many times per second.
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
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
                    // TODO: add settings
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

                        use klib::core::named_pitch::NamedPitch;
                        use klib::core::octave::Octave;
                        let tuning: [Note; 6] = [
                            Note::new(NamedPitch::E, Octave::Four),
                            Note::new(NamedPitch::B, Octave::Three),
                            Note::new(NamedPitch::G, Octave::Three),
                            Note::new(NamedPitch::D, Octave::Three),
                            Note::new(NamedPitch::A, Octave::Two),
                            Note::new(NamedPitch::E, Octave::Two),
                        ];

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
                                17 => "17",
                                19 => "19",
                                21 => "21",
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
                        for string in tuning {
                            for fret in 0..MAX_FRET {
                                ui.add(fret_string(string.clone(),
                                                   fret,
                                                   false,
                                                   &mut self.playback_handle));
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

#[cfg(test)]
mod tests {
    use klib::core::note::*;
    use klib::core::interval::Interval;

    #[test]
    fn test_turning_flat_to_sharps() {
        assert_eq!(GFlat + Interval::AugmentedSeventh - Interval::PerfectOctave, FSharp);
    }
}
