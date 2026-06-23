use crate::api::schema::{QueueAddParams, QueueTargetParams, ResponseResult};
use crate::app::App;

use super::responses::{encode_error_body, encode_success};

impl App {
    pub(super) fn handle_queue_add(&mut self, id: String, params: QueueAddParams) -> String {
        let resolved = match self.resolve_terminal_target(&params.target) {
            Ok(resolved) => resolved,
            Err(err) => return encode_error_body(id, self.agent_target_error_body(err)),
        };
        let key = self
            .state
            .queue_key_for_pane(resolved.ws_idx, resolved.pane_id);
        self.state.enqueue_prompt(key.clone(), params.text);
        encode_success(
            id,
            ResponseResult::QueueContents {
                count: self.state.queued_count(&key),
                prompts: self.state.list_prompts(&key),
            },
        )
    }

    pub(super) fn handle_queue_list(&mut self, id: String, params: QueueTargetParams) -> String {
        let resolved = match self.resolve_terminal_target(&params.target) {
            Ok(resolved) => resolved,
            Err(err) => return encode_error_body(id, self.agent_target_error_body(err)),
        };
        let key = self
            .state
            .queue_key_for_pane(resolved.ws_idx, resolved.pane_id);
        encode_success(
            id,
            ResponseResult::QueueContents {
                count: self.state.queued_count(&key),
                prompts: self.state.list_prompts(&key),
            },
        )
    }

    pub(super) fn handle_queue_pop(&mut self, id: String, params: QueueTargetParams) -> String {
        let resolved = match self.resolve_terminal_target(&params.target) {
            Ok(resolved) => resolved,
            Err(err) => return encode_error_body(id, self.agent_target_error_body(err)),
        };
        let key = self
            .state
            .queue_key_for_pane(resolved.ws_idx, resolved.pane_id);
        let text = self.state.pop_prompt(&key);
        encode_success(id, ResponseResult::QueuePopped { text })
    }
}
