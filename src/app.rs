use klib::core::base::{Playable, PlaybackHandle};
use std::time::Duration;

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
            chord: "".to_owned(),
            playback_handles: Vec::new(),
            selection: Vec::new(),
        }
    }
}

// TODO: allow pinch to zoom
impl TemplateApp {
    /// Called once before the first frame.
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let ctx = &cc.egui_ctx;

        let mut fonts = egui::FontDefinitions::default();
        fonts.font_data.insert(
            "noto_sans_music".to_owned(),
            egui::FontData::from_static(include_bytes!("../assets/NotoSans-RegularWithMusic.otf")),
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

const MAIN_FONT_SIZE: f32 = 18.0;
const BUTTON_HEIGHT: f32 = 60.0;
const BUTTON_SIZE: [f32; 2] = [BUTTON_HEIGHT, BUTTON_HEIGHT];
const MAX_FRET: usize = 16;

use klib::core::note::HasNoteId;
use klib::core::note::Note;

fn note_for_fret(string: Note, fret: usize) -> Note {
    let note_id = string.id() << fret;
    Note::from_id(note_id).unwrap()
}

fn playback_handle_add(handle: PlaybackHandle, handles: &mut Vec<PlaybackHandle>) {
    // LRU
    const MAX_HANDLES: usize = 50;
    if handles.len() >= MAX_HANDLES {
        handles.remove(0);
    }
    handles.push(handle);
}

fn note_button(
    note: Note,
    selected: bool,
    horizontal: bool,
    playback_handles: &mut Vec<PlaybackHandle>,
) -> impl egui::Widget + '_ {
    move |ui: &mut egui::Ui| {
        // Scope is in case we want to do style changes for this button
        // specifically, e.g. to set something different if this button is
        // disabled.
        ui.scope(|ui| {
            let note_name = match ui.is_enabled() {
                true => format_note_name(note),
                false => "".to_owned(),
            };

            let label = egui::SelectableLabel::new(selected, note_name);
            let response = ui.add_sized(BUTTON_SIZE, label);
            if response.clicked() {
                let dur = Duration::from_millis(500);
                // TODO: this crackles, just use the frequency and a different lib
                // to play sound?
                let ret = note.play(Duration::from_millis(0), dur, Duration::from_millis(0));

                match ret {
                    Ok(h) => {
                        log::debug!("played note {}", note);
                        // Have to keep the handle around to play the sound.
                        playback_handle_add(h, playback_handles);
                    }
                    Err(e) => log::error!("error playing note: {}", e),
                }
            }

            // Draw a line through the button if it's disabled, to help align frets
            if !ui.is_enabled() {
                let stroke = egui::Stroke::new(0.5, ui.visuals().widgets.inactive.fg_stroke.color);
                const OFFSET: f32 = 15.0;
                let points = if horizontal {
                    [
                        ui.min_rect().center_top() + egui::Vec2::new(0.0, OFFSET),
                        ui.min_rect().center_bottom() - egui::Vec2::new(0.0, OFFSET),
                    ]
                } else {
                    [
                        ui.min_rect().left_center() + egui::Vec2::new(OFFSET, 0.0),
                        ui.min_rect().right_center() - egui::Vec2::new(OFFSET, 0.0),
                    ]
                };

                ui.painter().line_segment(points, stroke);
            }
            response
        })
        .response
    }
}

fn fret_label(fret: usize) -> String {
    match fret {
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
    }
    .to_owned()
}

// Normalize the chordname so kord::parse can recognize it
fn fix_chord_name(chord: &str) -> String {
    // First, capitalize the first letter if it's a-g
    let mut chord = chord.to_owned();
    if let Some(c) = chord.chars().next() {
        if let 'a'..='g' = c {
            chord = format!("{}{}", c.to_ascii_uppercase(), &chord[1..]);
        }
    }

    // Uppercase the first letter after a slash (if it's a note)
    let mut last_was_slash = false;
    let mut ret = String::new();
    for c in chord.chars() {
        if last_was_slash {
            if let 'a'..='g' = c {
                ret.push(c.to_ascii_uppercase());
            } else {
                ret.push(c);
            }
        } else {
            ret.push(c);
        }
        last_was_slash = c == '/';
    }

    // convert Maj, MAj, etc, to lowercase
    ret = ret.replace("Maj", "maj");
    ret = ret.replace("MAj", "maj");
    ret = ret.replace("MAJ", "maj");

    ret
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

        egui::CentralPanel::default().show(ctx, |_ui| {
            // Just here to paint a background
        });

        // Estimate the space needed to not overlap the bottom panel
        const BOTTOM_PANEL_SLOP: f32 = 24.0;
        egui::Area::new("main")
            .anchor(egui::Align2::CENTER_TOP, [0.0, 5.0])
            // The goal of this constrain is to stop it from overlapping
            // the bottom panel.
            .constrain_to(
                ctx.available_rect()
                    .translate([0.0, -BOTTOM_PANEL_SLOP].into())
                    .shrink2([0.0, BOTTOM_PANEL_SLOP].into()),
            )
            .show(ctx, |ui| {
                // If screen is narrow, (e.g. phones in portrait mode), make
                // things more compact vertically
                let screen_rect = ctx.available_rect();
                let wide_enough = screen_rect.width() > BUTTON_SIZE[0] * (MAX_FRET as f32);
                let tall_enough = screen_rect.height() > BUTTON_SIZE[1] * (MAX_FRET as f32 + 3.0);
                let aspect_ratio = screen_rect.width() / screen_rect.height();
                let max_aspect_ratio = MAX_FRET as f32 / 10.0;

                // Needs to be wide enough *or* if its narrow enough up to a certain point
                let horizontal = wide_enough && !tall_enough || aspect_ratio > max_aspect_ratio;

                let style = ui.style_mut();
                style
                    .text_styles
                    .get_mut(&egui::TextStyle::Button)
                    .unwrap()
                    .size = MAIN_FONT_SIZE;

                style
                    .text_styles
                    .get_mut(&egui::TextStyle::Body)
                    .unwrap()
                    .size = MAIN_FONT_SIZE;

                style
                    .text_styles
                    .get_mut(&egui::TextStyle::Heading)
                    .unwrap()
                    .size = 12.0;

                style.spacing.item_spacing = egui::Vec2::new(0.0, 0.0);

                // store chord pitches (need HasPitch to convert Note to Pitch)
                use klib::core::pitch::HasPitch;
                use klib::core::pitch::Pitch;
                let mut chord_pitches: Vec<Pitch> = Vec::new();

                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.heading("Chord finder");
                        if ui
                            .add_sized(
                                [150.0, BUTTON_HEIGHT],
                                egui::TextEdit::singleline(&mut self.chord)
                                    .vertical_align(egui::Align::Center),
                            )
                            .changed()
                        {
                            self.chord = fix_chord_name(self.chord.as_str());
                        }
                    });

                    // Add a text field for the user to enter a chord name
                    use klib::core::base::Parsable;
                    use klib::core::chord::Chord;
                    use klib::core::chord::HasChord;

                    // parse the chord and show it
                    let chord = Chord::parse(self.chord.as_str());
                    if !self.chord.is_empty() {
                        ui.add_space(15.0);
                        ui.separator();
                        ui.add_space(15.0);
                        ui.vertical(|ui| {
                            match chord {
                                Ok(chord) => {
                                    ui.heading("Chord notes");
                                    ui.horizontal(|ui| {
                                        chord.chord().iter().for_each(|note| {
                                            if horizontal {
                                                ui.add(note_button(
                                                    *note,
                                                    false,
                                                    true,
                                                    &mut self.playback_handles,
                                                ));
                                            } else {
                                                use egui::widgets::Label;
                                                ui.add_sized(
                                                    [0.0, BUTTON_SIZE[1]],
                                                    Label::new(format_note_name(*note)),
                                                );
                                                ui.add_space(8.0);
                                            }

                                            // store pitch
                                            chord_pitches.push(note.pitch());
                                        });
                                    });
                                }
                                Err(_e) => {
                                    ui.heading(format!("Invalid chord: {}", self.chord));
                                }
                            }
                        });
                    }
                });

                ui.add_space(20.0);

                ui.heading("Fretboard");

                egui::ScrollArea::vertical().show(ui, |ui| {
                    use klib::core::named_pitch::NamedPitch;
                    use klib::core::octave::Octave;
                    // Standard guitar tuning -- TODO: make this configurable
                    let tuning: [Note; 6] = [
                        Note::new(NamedPitch::E, Octave::Four),
                        Note::new(NamedPitch::B, Octave::Three),
                        Note::new(NamedPitch::G, Octave::Three),
                        Note::new(NamedPitch::D, Octave::Three),
                        Note::new(NamedPitch::A, Octave::Two),
                        Note::new(NamedPitch::E, Octave::Two),
                    ];

                    let fret_label_widget = |ui: &mut egui::Ui, fret: usize| {
                        ui.add_sized(
                            [BUTTON_SIZE[0], BUTTON_SIZE[1] / 2.0],
                            egui::Label::new(
                                egui::RichText::new(fret_label(fret)).strong().size(12.0),
                            ),
                        );
                    };

                    let mut fret_note_widget = |ui: &mut egui::Ui, string: Note, fret: usize| {
                        let note = note_for_fret(string, fret);
                        // enable only if chord pitches are empty or note is in the chord
                        let enabled =
                            chord_pitches.is_empty() || chord_pitches.contains(&note.pitch());
                        ui.add_enabled(
                            enabled,
                            note_button(note, false, horizontal, &mut self.playback_handles),
                        );
                    };

                    egui::Grid::new("fretboard").show(ui, |ui| {
                        // I forget what this does
                        // ui.style_mut().visuals.widgets.hovered.bg_fill = egui::Color32::DARK_GRAY;

                        // Add fretboard labels as the first row if horizontal
                        if horizontal {
                            for fret in 0..MAX_FRET {
                                fret_label_widget(ui, fret);
                            }
                            ui.end_row();

                            // add a row of buttons for each of the 6 strings
                            for string in tuning {
                                for fret in 0..MAX_FRET {
                                    fret_note_widget(ui, string, fret);
                                }
                                ui.end_row();
                            }
                        } else {
                            for fret in 0..MAX_FRET {
                                // Reverse string tuning
                                fret_label_widget(ui, fret);
                                for string in tuning.iter().rev() {
                                    fret_note_widget(ui, *string, fret);
                                }
                                ui.end_row();
                            }
                        }
                    });
                });
            });

        egui::TopBottomPanel::bottom("bottom_panel").show(ctx, |ui| {
            egui::warn_if_debug_build(ui);
            powered_by_egui_and_eframe(ui);
        });
    }
}

fn powered_by_egui_and_eframe(ui: &mut egui::Ui) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 0.0;
        ui.add(egui::github_link_file!(
            "https://github.com/kelvie/chord-finder/blob/master/",
            "Source code"
        ));
        ui.label(".");
        ui.add_space(5.0);
        ui.label("Powered by ");
        ui.hyperlink_to("egui", "https://github.com/emilk/egui");
        ui.label(", ");
        ui.hyperlink_to(
            "eframe",
            "https://github.com/emilk/egui/tree/master/crates/eframe",
        );
        ui.label(", and ");
        ui.hyperlink_to("kord", "https://github.com/twitchax/kord");
        ui.label(". ");
    });
}

#[cfg(test)]
mod tests {
    use klib::core::interval::Interval;
    use klib::core::note::*;

    #[test]
    fn test_turning_flat_to_sharps() {
        assert_eq!(
            GFlat + Interval::AugmentedSeventh - Interval::PerfectOctave,
            FSharp
        );
    }
}
