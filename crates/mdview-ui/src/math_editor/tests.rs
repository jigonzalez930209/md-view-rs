//! Formula editor dialog behavior, driven through egui like real input.

use std::{cell::RefCell, rc::Rc};

use egui::{Color32, Event, Key, Modifiers};
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;

use super::{MathEditorOutcome, MathEditorState, show_math_editor};

type Shared = (Rc<RefCell<MathEditorState>>, Rc<RefCell<Vec<MathEditorOutcome>>>);

fn harness(tex: &str, display: bool, editing: bool) -> (Harness<'static>, Shared) {
    let state = Rc::new(RefCell::new(MathEditorState::new(tex, display, editing)));
    let outcomes = Rc::new(RefCell::new(Vec::new()));
    let (handle, sink) = (state.clone(), outcomes.clone());
    let harness = Harness::builder()
        .with_size(egui::vec2(1000.0, 720.0))
        .build_ui(move |ui| {
            let translate = |text: &str| text.to_owned();
            if let Some(outcome) = show_math_editor(
                ui.ctx(),
                &mut handle.borrow_mut(),
                Color32::from_rgb(9, 105, 218),
                &translate,
            ) {
                sink.borrow_mut().push(outcome);
            }
        });
    (harness, (state, outcomes))
}

#[test]
fn new_templates_select_their_first_placeholder() {
    let state = MathEditorState::new("\\frac{\\square}{\\square}", true, false);
    assert_eq!(state.editor.selection_range(), (6, 13));
}

#[test]
fn palette_templates_insert_and_tab_cycles_placeholders() {
    let (mut harness, (state, _)) = harness("x = ", true, false);
    harness.run();
    harness.get_by_label("\\frac{\\square}{\\square}").click();
    harness.run();
    assert_eq!(state.borrow().tex(), "x = \\frac{\\square}{\\square}");
    assert_eq!(state.borrow().editor.selection_range(), (10, 17));
    harness.event(Event::Text("a".to_owned()));
    harness.run();
    harness.key_press(Key::Tab);
    harness.run();
    assert_eq!(state.borrow().tex(), "x = \\frac{a}{\\square}");
    assert_eq!(state.borrow().editor.selection_range(), (13, 20));
}

#[test]
fn update_reports_the_source_and_the_mode() {
    let (mut harness, (_, outcomes)) = harness("a^2 + b^2", false, true);
    harness.run();
    harness.get_by_label("Display").click();
    harness.run();
    harness.get_by_label("Update").click();
    harness.run();
    assert_eq!(
        outcomes.borrow().first(),
        Some(&MathEditorOutcome::Submit {
            tex: "a^2 + b^2".to_owned(),
            display: true,
        })
    );
}

#[test]
fn command_enter_submits_and_cancel_closes() {
    let (mut harness, (_, outcomes)) = harness("x", true, false);
    harness.run();
    harness.key_press_modifiers(Modifiers::COMMAND, Key::Enter);
    harness.run();
    assert!(matches!(
        outcomes.borrow().first(),
        Some(MathEditorOutcome::Submit { .. })
    ));
    let (mut harness, (_, outcomes)) = self::harness("x", true, false);
    harness.run();
    harness.get_by_label("Cancel").click();
    harness.run();
    assert_eq!(outcomes.borrow().first(), Some(&MathEditorOutcome::Cancel));
}
