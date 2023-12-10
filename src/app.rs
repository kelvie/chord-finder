use std::time::Duration;
use klib::core::base::{Playable, PlaybackHandle};

/// We derive Deserialize/Serialize so we can persist app state on shutdown.
#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)] // if we add new fields, give them default values when deserializing old state
pub struct TemplateApp {
    chord: String,

    // Used as an LRU cache for the last played note -- the handles need to
    // exist for the sound to continue playing.
    #[serde(skip)]
    playback_handles: Vec<PlaybackHandle>,

    #[serde(skip)]
    selection: Vec<Note>,
}

impl Default for TemplateApp {
    fn default() -> Self {
        Self {
            chord: "C".to_owned(),
            playback_handles: Vec::new(),
            selection: Vec::new(),
        }
    }
}

impl TemplateApp {
    /// Called once before the first frame.
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        const FONT_SIZE: f32 = 18.0;
        let ctx = &cc.egui_ctx;
        let mut style = (*ctx.style()).clone();
        style.text_styles.get_mut(&egui::TextStyle::Button).unwrap().size = FONT_SIZE;
        style.spacing.item_spacing = egui::Vec2::new(0.0, 0.0);
        ctx.set_style(style);

        let mut fonts = egui::FontDefinitions::default();
        fonts.font_data.insert(
            "noto_sans_music".to_owned(),
            egui::FontData::from_static(
                include_bytes!("../assets/NotoSans-RegularWithMusic.otf")
            )
        );

        fonts
            .families
            .entry(egui::FontFamily::Proportional)
            .or_default()
            .insert(0, "noto_sans_music".to_owned());

        ctx.set_fonts(fonts);

        // Load previous app state (if any).
        // Note that you must enable the `persistence` feature for this to work.
        if let Some(storage) = cc.storage {
            return eframe::get_value(storage, eframe::APP_KEY).unwrap_or_default();
        }


        Default::default()
    }
}

// Format a note for printing
fn format_note_name(note: Note) -> String {
    use klib::core::interval::Interval;
    use klib::core::note::ToUniversal;

    // Get universal note, which may be flat, but never sharp (or double flat/sharp)
    let mut unote = note.to_universal();
    let note_name = unote.to_string();
    // convert flats to sharps
    if note_name.contains('♭') {
        unote = unote + Interval::AugmentedSeventh - Interval::PerfectOctave;
    }

    // Turn octave (well all numbers) into subscripts
    let mut s = String::new();
    for c in unote.to_string().chars() {
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

const BUTTON_HEIGHT: f32 = 60.0;
const BUTTON_SIZE: [f32; 2] = [BUTTON_HEIGHT, BUTTON_HEIGHT];
const MAX_FRET: usize = 17;

use klib::core::note::HasNoteId;
use klib::core::note::Note;

fn note_for_fret(string: Note, fret: usize) -> Note {
    let note_id = string.id() << fret;
    Note::from_id(note_id).unwrap()
}

fn playback_handle_add(handle: PlaybackHandle, handles: &mut
                       Vec<PlaybackHandle>) {
    // LRU
    const MAX_HANDLES: usize = 50;
    if handles.len() >= MAX_HANDLES {
        handles.remove(0);
    }
    handles.push(handle);
}

fn note_button(note: Note, selected: bool,
               playback_handles: &mut Vec<PlaybackHandle>) -> impl egui::Widget + '_ {
    move |ui: &mut egui::Ui| {

        // Scope is in case we want to do style changes for this button
        // specifically, e.g. to set something different if this button is
        // disabled.
        ui.scope(|ui| {
            let note_name = match ui.is_enabled() {
                true => format_note_name(note),
                false => "|".to_owned(),
            };

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
                        playback_handle_add(h, playback_handles);
                    },
                    Err(e) => log::error!("error playing note: {}", e),
                }
            }

            response
        }).response
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
            // store chord pitches (need HasPitch to convert Note to Pitch)
            use klib::core::pitch::Pitch;
            use klib::core::pitch::HasPitch;
            let mut chord_pitches: Vec<Pitch> = Vec::new();

            ui.heading("Chord Finder");
            ui.add_space(10.0);
            if ui.add_sized(
                [200.0, 0.0],
                egui::TextEdit::singleline(&mut self.chord)
                    .hint_text("Enter a chord name"),
            ).changed() {
                // if chord starts with a-g, capitalize it for UX reasons
                if let Some(c) = self.chord.chars().next() {
                    if let 'a'..='g' = c {
                        self.chord = format!("{}{}", c.to_ascii_uppercase(), &self.chord[1..]);
                    }
                }
            }

            ui.add_space(10.0);
            // Add a text field for the user to enter a chord name
            use klib::core::chord::Chord;
            use klib::core::base::Parsable;
            use klib::core::chord::HasChord;

            // parse the chord and show it
            let chord = Chord::parse(self.chord.as_str());
            if !self.chord.is_empty() {
                egui::Grid::new("chord_id").show(ui, |ui| {
                    match chord {
                        Ok(chord) => {
                            chord.chord()
                                 .iter()
                                 .for_each(|note| {
                                     ui.add(note_button(
                                         *note, false, &mut self.playback_handles)
                                     );
                                     // store pitch
                                     chord_pitches.push(note.pitch());
                                 });

                        },
                        Err(_e) => {
                            ui.label(format!("Invalid chord: {}", self.chord));
                        }
                    }
                });
            }

            // TODO center the heading with the fretboard (or what's visible of
            // it). Or just make this look nicer.
            ui.heading("Fretboard");

            ui.add_space(20.0);

            ui.horizontal(|ui| {
                ui.add_space(30.0);

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

                    for _ in 0..MAX_FRET {
                        ui.separator();
                    }
                    ui.end_row();

                    // add a row of buttons for each of the 6 strings
                    for string in tuning {
                        for fret in 0..MAX_FRET {
                            // enable only if chord pitches are empty or note is in the chord
                            let fret_note = note_for_fret(string.clone(), fret);
                            let enabled = chord_pitches.is_empty() ||
                                chord_pitches.contains(&fret_note.pitch());
                            ui.add_enabled(enabled, note_button(fret_note, false, &mut self.playback_handles));
                        }
                        ui.end_row();
                    }
                });

                ui.add_space(30.0);
            });

            ui.add_space(20.0);


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
        ui.add(egui::github_link_file!(
                "https://github.com/kelvie/chord-finder-eframe/",
                "Source code"
        ));
        ui.label(".");
        ui.add_space(5.0);
        ui.spacing_mut().item_spacing.x = 0.0;
        ui.label("Powered by ");
        ui.hyperlink_to("egui", "https://github.com/emilk/egui");
        ui.label(" and ");
        ui.hyperlink_to(
            "eframe",
            "https://github.com/emilk/egui/tree/master/crates/eframe",
        );
        ui.label(" and ");
        ui.hyperlink_to(
            "kord",
            "https://github.com/twitchax/kord",
        );
        ui.label(". ");
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
