//! Right-hand persistent agent-overview column.
//!
//! A single full-height column on the right edge, added as a third top-level
//! `horizontal` split sibling of the existing left sidebar. It lives outside
//! the per-workspace tab tree, so it stays visible across every workspace and
//! tab switch. Agents are grouped under their workspace: each workspace heads a
//! block, with its agents indented beneath showing state colour, label, one-line
//! what/how summary (`custom_status`), and the pending next-prompt queue.

use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use super::sidebar::{agent_panel_entries_from, AgentPanelEntry};
use super::status::state_label_color;
use crate::app::state::Palette;
use crate::app::{AppState, Mode};
use crate::terminal::TerminalRuntimeRegistry;

/// The sub-rect inside the note pane where the nvim grid is drawn: the column
/// area minus the LEFT border and minus the top row reserved for the title.
/// Used by both the layout pass (to size the PTY) and the renderer.
pub(super) fn note_inner_rect(note_area: Rect) -> Rect {
    if note_area.width == 0 || note_area.height == 0 {
        return Rect::new(note_area.x, note_area.y, 0, 0);
    }
    let inner = Block::default().borders(Borders::LEFT).inner(note_area);
    if inner.width == 0 || inner.height <= 1 {
        return Rect::new(inner.x, inner.y, inner.width, 0);
    }
    Rect::new(inner.x, inner.y + 1, inner.width, inner.height - 1)
}

/// Render the resident note pane (top of the persistent column): a LEFT border
/// and " note " title, with the live nvim terminal grid drawn inside. The
/// host cursor is shown only while the note pane has focus (`Mode::Note`).
pub(super) fn render_note_pane(
    app: &AppState,
    terminal_runtimes: &TerminalRuntimeRegistry,
    frame: &mut Frame,
    area: Rect,
) {
    if area.width == 0 || area.height == 0 {
        return;
    }
    let p = &app.palette;
    let focused = app.mode == Mode::Note;
    let title_style = if focused {
        Style::default().fg(p.mauve).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(p.subtext0)
    };
    let block = Block::default()
        .borders(Borders::LEFT)
        .border_style(Style::default().fg(if focused { p.mauve } else { p.overlay0 }))
        .title(Line::from(Span::styled(" note ", title_style)));
    frame.render_widget(block, area);

    let inner = note_inner_rect(area);
    if inner.width == 0 || inner.height == 0 {
        return;
    }
    match app
        .note_terminal_id
        .as_ref()
        .and_then(|id| terminal_runtimes.get(id))
    {
        Some(rt) => rt.render(frame, inner, focused),
        None => frame.render_widget(
            Paragraph::new(Line::from(Span::styled(
                "starting nvim…",
                Style::default().fg(p.subtext0),
            ))),
            inner,
        ),
    }
}

pub(super) fn render_persistent_pane(
    app: &AppState,
    terminal_runtimes: &TerminalRuntimeRegistry,
    frame: &mut Frame,
    area: Rect,
) {
    if area.width == 0 || area.height == 0 {
        return;
    }

    let p = &app.palette;
    let focused = app.mode == Mode::Queues;
    let title_style = if focused {
        Style::default().fg(p.mauve).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(p.subtext0)
    };
    let block = Block::default()
        .borders(Borders::LEFT)
        .border_style(Style::default().fg(if focused { p.mauve } else { p.overlay0 }))
        .title(Line::from(Span::styled(" queues ", title_style)));
    let inner = block.inner(area);
    frame.render_widget(block, area);
    if inner.width == 0 || inner.height == 0 {
        return;
    }
    let max_w = inner.width as usize;

    let mut entries = agent_panel_entries_from(app, terminal_runtimes);
    // Stable order (matches compute_view) so selection/highlight stay aligned.
    entries.sort_by_key(|entry| (entry.ws_idx, entry.pane_id.raw()));
    // Clamp the focused selection to the live agent count (display order).
    let selected = match (focused, entries.len()) {
        (true, len) if len > 0 => Some(app.persistent_pane_selected.min(len - 1)),
        _ => None,
    };
    let mut agent_idx = 0usize;
    let mut lines: Vec<Line> = Vec::new();

    for group in group_by_workspace(&entries) {
        // Workspace header, with the workspace's total queued count rolled up.
        let total: usize = group
            .members
            .iter()
            .map(|entry| app.queued_count(&app.queue_key_for_pane(entry.ws_idx, entry.pane_id)))
            .sum();
        let badge_w = if total > 0 { total.to_string().len() + 3 } else { 0 };
        let mut header = vec![Span::styled(
            truncate(group.label, max_w.saturating_sub(badge_w)),
            Style::default().fg(p.text).add_modifier(Modifier::BOLD),
        )];
        if total > 0 {
            header.push(Span::styled(
                format!(" [{total}]"),
                Style::default().fg(p.subtext0),
            ));
        }
        lines.push(Line::from(header));

        // Agents indented beneath their workspace header.
        for entry in &group.members {
            let is_selected = selected == Some(agent_idx);
            agent_idx += 1;
            let color = state_label_color(entry.state, entry.seen, p);
            let label = entry.agent_label.as_deref().unwrap_or("agent");
            let key = app.queue_key_for_pane(entry.ws_idx, entry.pane_id);
            let prompts = app.list_prompts(&key);

            // Drilled-in = this agent is selected and we are in ItemNav or text entry.
            let active = is_selected;
            let item_sel = if active { app.persistent_item_selected } else { None };
            let input = if active { app.persistent_input.as_ref() } else { None };
            let drilled = active && (item_sel.is_some() || input.is_some());

            let label_style = if active {
                Style::default().fg(p.text).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(p.text)
            };
            let agent_marker = match (active, drilled) {
                (true, true) => "▾ ",
                (true, false) => "▸ ",
                _ => "  ",
            };
            let mut spans = vec![
                Span::styled(agent_marker, Style::default().fg(p.mauve)),
                Span::styled("● ", Style::default().fg(color)),
                Span::styled(truncate(label, max_w.saturating_sub(4)), label_style),
            ];
            if !prompts.is_empty() {
                spans.push(Span::styled(
                    format!(" [{}]", prompts.len()),
                    Style::default().fg(p.subtext0),
                ));
            }
            // Idle auto-send indicator (default off).
            if app.autosend_enabled(&key) {
                spans.push(Span::styled(
                    " ⏵auto",
                    Style::default().fg(p.green).add_modifier(Modifier::BOLD),
                ));
            }
            lines.push(Line::from(spans));

            // F1: one-line what/how summary, sourced from custom_status.
            if let Some(what) = entry.custom_status.as_deref().filter(|s| !s.is_empty()) {
                lines.push(Line::from(Span::styled(
                    format!("    ~ {}", truncate(what, max_w.saturating_sub(6))),
                    Style::default().fg(p.subtext0),
                )));
            }

            // F2/F3: queued prompts, with ItemNav selection and inline edit.
            for (idx, text) in prompts.iter().enumerate() {
                if let Some(inp) = input {
                    if inp.editing == Some(idx) {
                        lines.push(queue_input_line(p, max_w, idx + 1, &inp.buffer));
                        continue;
                    }
                }
                let selected_item = item_sel == Some(idx);
                let prefix = if selected_item {
                    format!("  ▸ {}. ", idx + 1)
                } else {
                    format!("    {}. ", idx + 1)
                };
                let body_w = max_w.saturating_sub(prefix.chars().count());
                let text_style = if selected_item {
                    Style::default().fg(p.text).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(p.text)
                };
                lines.push(Line::from(vec![
                    Span::styled(
                        prefix,
                        Style::default().fg(if selected_item { p.mauve } else { p.overlay0 }),
                    ),
                    Span::styled(truncate(&text.replace('\n', " "), body_w), text_style),
                ]));
            }
            // Adding a new prompt: input field after the agent's existing items.
            if let Some(inp) = input.filter(|i| i.editing.is_none()) {
                lines.push(queue_input_line(p, max_w, 0, &inp.buffer));
            }
        }
    }

    if lines.is_empty() {
        lines.push(Line::from(Span::styled(
            "no agents",
            Style::default().fg(p.overlay0),
        )));
    }

    // Context-aware controls hint while focused.
    if focused {
        let hint = if app.persistent_input.is_some() {
            "↵ save  esc cancel"
        } else if app.persistent_item_selected.is_some() {
            "↵ edit  d del  ␣ send  esc back"
        } else {
            "↵ add  e items  ␣ send  esc quit"
        };
        lines.push(Line::from(Span::styled(
            truncate(hint, max_w),
            Style::default().fg(p.overlay0),
        )));
    }

    frame.render_widget(Paragraph::new(lines), inner);
}

/// A workspace and the agent entries that belong to it.
struct WorkspaceGroup<'a> {
    label: &'a str,
    members: Vec<&'a AgentPanelEntry>,
}

/// Group agent entries by workspace, preserving the first-appearance order of
/// both workspaces and the agents within each. Independent of the active sort
/// mode: `Priority` sort interleaves workspaces, and this re-collates them so
/// each workspace heads a single contiguous block.
fn group_by_workspace(entries: &[AgentPanelEntry]) -> Vec<WorkspaceGroup<'_>> {
    let mut groups: Vec<WorkspaceGroup<'_>> = Vec::new();
    for entry in entries {
        match groups
            .iter_mut()
            .find(|group| group.members[0].ws_idx == entry.ws_idx)
        {
            Some(group) => group.members.push(entry),
            None => groups.push(WorkspaceGroup {
                label: entry.primary_label.as_str(),
                members: vec![entry],
            }),
        }
    }
    groups
}

/// The (ws_idx, pane_id) of every agent in display (grouped) order — the same
/// order the pane renders and `j`/`k` walk. Lets app-layer input handlers map
/// the selection index back to a concrete agent.
pub(super) fn ordered_agent_ids(
    entries: &[AgentPanelEntry],
) -> Vec<(usize, crate::layout::PaneId)> {
    group_by_workspace(entries)
        .into_iter()
        .flat_map(|group| {
            group
                .members
                .into_iter()
                .map(|entry| (entry.ws_idx, entry.pane_id))
        })
        .collect()
}

/// A text-entry line for the queues pane. `number == 0` is a new prompt (`+`),
/// otherwise it edits the prompt at that 1-based position. Shows the tail of the
/// buffer with a block cursor so typing stays visible.
fn queue_input_line(p: &Palette, max_w: usize, number: usize, buffer: &str) -> Line<'static> {
    let prefix = if number == 0 {
        "  + ".to_string()
    } else {
        format!("  {number}> ")
    };
    let body_w = max_w.saturating_sub(prefix.chars().count() + 1);
    Line::from(vec![
        Span::styled(prefix, Style::default().fg(p.mauve)),
        Span::styled(tail(buffer, body_w), Style::default().fg(p.text)),
        Span::styled("▌", Style::default().fg(p.mauve)),
    ])
}

/// The last `max` characters of `text`, prefixed with `…` when truncated.
fn tail(text: &str, max: usize) -> String {
    let chars: Vec<char> = text.chars().collect();
    if chars.len() <= max {
        return text.to_string();
    }
    if max == 0 {
        return String::new();
    }
    let start = chars.len() - (max - 1);
    format!("…{}", chars[start..].iter().collect::<String>())
}

fn truncate(text: &str, max_width: usize) -> String {
    let len = text.chars().count();
    if len <= max_width {
        return text.to_string();
    }
    match max_width {
        0 => String::new(),
        1 => "…".to_string(),
        _ => {
            let prefix: String = text.chars().take(max_width - 1).collect();
            format!("{prefix}…")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::detect::AgentState;

    fn entry(ws_idx: usize, ws_label: &str, agent: &str, pane: u32) -> AgentPanelEntry {
        AgentPanelEntry {
            ws_idx,
            tab_idx: 0,
            pane_id: crate::layout::PaneId::from_raw(pane),
            primary_label: ws_label.to_string(),
            primary_tab_label: None,
            agent_label: Some(agent.to_string()),
            state: AgentState::Idle,
            seen: true,
            last_agent_state_change_seq: None,
            custom_status: None,
            state_labels: std::collections::HashMap::new(),
        }
    }

    #[test]
    fn groups_agents_by_workspace_preserving_first_appearance_order() {
        // Interleaved across workspaces, as Priority sort would produce.
        let entries = vec![
            entry(0, "alpha", "claude", 1),
            entry(1, "beta", "codex", 2),
            entry(0, "alpha", "droid", 3),
        ];
        let groups = group_by_workspace(&entries);
        assert_eq!(groups.len(), 2);
        assert_eq!(groups[0].label, "alpha");
        assert_eq!(
            groups[0]
                .members
                .iter()
                .map(|m| m.agent_label.as_deref().unwrap())
                .collect::<Vec<_>>(),
            vec!["claude", "droid"],
        );
        assert_eq!(groups[1].label, "beta");
        assert_eq!(groups[1].members.len(), 1);
    }

    #[test]
    fn group_by_workspace_handles_empty() {
        assert!(group_by_workspace(&[]).is_empty());
    }
}
