use std::time::Instant;

use super::{App, SESSION_SAVE_DEBOUNCE};

impl App {
    pub(super) fn schedule_session_save(&mut self) {
        if !self.no_session {
            self.session_save_deadline = Some(Instant::now() + SESSION_SAVE_DEBOUNCE);
        }
    }

    pub(crate) fn sync_session_save_schedule(&mut self) {
        if self.state.session_dirty {
            self.state.session_dirty = false;
            self.schedule_session_save();
        }
    }

    pub(crate) fn save_session_now(&mut self) {
        if self.no_session {
            self.session_save_deadline = None;
            return;
        }

        if self.state.workspaces.is_empty() {
            crate::persist::clear();
        } else {
            let agent_queues = self
                .state
                .agent_queues
                .iter()
                // Only persist stable keys (session:/cwd:); pane:-fallback keys
                // never re-bind after a restart, so persisting them would leak.
                .filter(|(key, _)| !key.starts_with("pane:"))
                .map(|(key, queue)| {
                    (
                        key.clone(),
                        queue.iter().map(|prompt| prompt.text.clone()).collect(),
                    )
                })
                .collect();
            let snap = crate::persist::capture(
                &self.state.workspaces,
                &self.state.terminals,
                &self.terminal_runtimes,
                self.state.active,
                self.state.selected,
                self.state.sidebar_width,
                self.state.sidebar_section_split,
                self.state.collapsed_space_keys.clone(),
                agent_queues,
            );
            let history = self.persist_pane_history.then(|| {
                crate::persist::capture_history(&self.state.workspaces, &self.terminal_runtimes)
            });
            crate::persist::save(&snap, history.as_ref());
        }

        self.session_save_deadline = None;
    }
}
