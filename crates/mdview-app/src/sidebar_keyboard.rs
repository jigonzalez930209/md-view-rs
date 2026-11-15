//! Keyboard navigation for the workspace tree.

use std::path::PathBuf;

use eframe::egui;
use mdview_workspace::{Entry, Workspace};

use super::{Action, Sidebar};

impl Sidebar {
    pub(super) fn keyboard_navigation(
        &mut self,
        ui: &mut egui::Ui,
        workspace: &Workspace,
        focus_id: egui::Id,
        actions: &mut Vec<Action>,
    ) {
        let Some(children) = workspace.root_entry().children.as_deref() else {
            return;
        };
        let mut flat = Vec::new();
        flatten(children, None, &mut flat);
        if flat.is_empty() {
            return;
        }
        let index = self
            .selected
            .as_ref()
            .and_then(|path| flat.iter().position(|entry| &entry.0 == path))
            .unwrap_or(0);
        let mut next = index;
        let mut action = None;
        if ui.input(|input| input.key_pressed(egui::Key::ArrowDown)) {
            next = (index + 1).min(flat.len() - 1);
        } else if ui.input(|input| input.key_pressed(egui::Key::ArrowUp)) {
            next = index.saturating_sub(1);
        } else if ui.input(|input| input.key_pressed(egui::Key::ArrowRight)) {
            let (path, is_dir, expanded, _) = &flat[index];
            if *is_dir && !*expanded {
                action = Some(Action::Toggle(path.clone()));
            } else if let Some(child) = flat.get(index + 1)
                && child.3.as_ref() == Some(path)
            {
                next = index + 1;
            }
        } else if ui.input(|input| input.key_pressed(egui::Key::ArrowLeft)) {
            let (path, _, expanded, parent) = &flat[index];
            if *expanded {
                action = Some(Action::Toggle(path.clone()));
            } else if let Some(parent) = parent
                && let Some(parent_index) = flat.iter().position(|entry| &entry.0 == parent)
            {
                next = parent_index;
            }
        } else if ui.input(|input| input.key_pressed(egui::Key::Enter)) {
            let (path, is_dir, _, _) = &flat[index];
            action = Some(if *is_dir {
                Action::Toggle(path.clone())
            } else {
                Action::Open(path.clone())
            });
        } else if ui.input(|input| input.key_pressed(egui::Key::F2)) {
            action = Some(Action::Rename(flat[index].0.clone()));
        }
        if let Some(action) = action {
            actions.push(action);
        }
        self.selected = Some(flat[next].0.clone());
        ui.memory_mut(|memory| memory.request_focus(focus_id));
        ui.ctx().request_repaint();
    }
}

fn flatten(entries: &[Entry], parent: Option<&PathBuf>, out: &mut Vec<(PathBuf, bool, bool, Option<PathBuf>)>) {
    for entry in entries {
        out.push((entry.path.clone(), entry.is_dir, entry.expanded, parent.cloned()));
        if entry.expanded
            && let Some(children) = &entry.children
        {
            flatten(children, Some(&entry.path), out);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expanded_entries_flatten_in_visual_order() {
        let entry = |name: &str, is_dir: bool, expanded: bool, children: Option<Vec<Entry>>| Entry {
            path: PathBuf::from(name),
            name: name.to_owned(),
            is_dir,
            expanded,
            children,
        };
        let entries = vec![
            entry("a", true, true, Some(vec![entry("a/one", false, false, None)])),
            entry("b", false, false, None),
        ];
        let mut flat = Vec::new();
        flatten(&entries, None, &mut flat);
        assert_eq!(
            flat.iter().map(|entry| entry.0.clone()).collect::<Vec<_>>(),
            vec![PathBuf::from("a"), PathBuf::from("a/one"), PathBuf::from("b")]
        );
        assert_eq!(flat[1].3, Some(PathBuf::from("a")));
    }
}
