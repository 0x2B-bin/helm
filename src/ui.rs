use crate::{App, app::UiState, app::View};
use bollard::config::ContainerSummaryStateEnum;
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Color, Style, Stylize},
    text::Line,
    widgets::{Block, Borders, Cell, List, ListState, Row, Table, TableState},
};

pub fn render(frame: &mut Frame, app: &App, ui_state: &mut UiState) {
    let main_layout = Layout::vertical([Constraint::Fill(1), Constraint::Percentage(50)]);
    let [top, bottom] = frame.area().layout(&main_layout);

    let top_layout = Layout::horizontal([Constraint::Percentage(95), Constraint::Percentage(5)]);
    let [containers_area, _control_area] = top.layout(&top_layout);

    render_table(frame, containers_area, app, &mut ui_state.container_table);
    render_log(frame, bottom, app, &mut ui_state.log_list);
}

fn render_log(frame: &mut Frame, area: Rect, app: &App, list_state: &mut ListState) {
    let title = Line::from(vec![" L".blue().bold(), "ogs ".white().into()]);

    let mut block = Block::new().borders(Borders::ALL).title(title);
    let mut highlight_style = Style::new();

    if let View::Log = app.active_view {
        block = block.border_style(Style::new().fg(Color::Blue));
        highlight_style = highlight_style.fg(Color::Red);
    }

    if app.current_logs.len() > 1 {
        let items = List::new(app.current_logs.iter().map(|line| line.as_str()))
            .block(block)
            .highlight_style(highlight_style);

        frame.render_stateful_widget(items, area, list_state);
    } else {
        frame.render_widget(block, area);
    }
}

fn render_table(frame: &mut Frame, area: Rect, app: &App, table_state: &mut TableState) {
    let header = Row::new(["Name", "State", "Status", "CPU%", "MEM%", "ID", "Image"])
        .style(Style::new().bold())
        .bottom_margin(1);

    let mut rows = Vec::new();
    let mut state_max_width = 0;
    let mut status_max_width = 0;
    let mut cpu_max_with = 0;

    let (memory_usage_max_width, memory_limit_max_width) =
        app.containers
            .iter()
            .fold((0, 0), |(max_usage, max_limit), item| {
                (
                    max_usage.max(item.memory_usage.len()),
                    max_limit.max(item.memory_limit.len()),
                )
            });

    for container in &app.containers {
        let mut text_color = Style::new().light_blue();
        let mut state = String::new();

        if let Some(trans_state) = app.transitioning_containers.get(&container.id) {
            state = format!("{}", trans_state);
        } else {
            state = container.state_string.clone();
            text_color = match container.state {
                ContainerSummaryStateEnum::RUNNING => Style::new().green(),
                ContainerSummaryStateEnum::DEAD | ContainerSummaryStateEnum::EXITED => {
                    Style::new().red()
                }
                ContainerSummaryStateEnum::PAUSED
                | ContainerSummaryStateEnum::STOPPING
                | ContainerSummaryStateEnum::RESTARTING => Style::new().yellow(),
                ContainerSummaryStateEnum::EMPTY | ContainerSummaryStateEnum::CREATED => {
                    Style::new().cyan()
                }
                _ => Style::new(),
            };
        }


        if state.len() > state_max_width {
            state_max_width = state.len();
        }

        if container.status.len() > status_max_width {
            status_max_width = container.status.len();
        }

        if container.cpu_percentage.len() > cpu_max_with {
            cpu_max_with = container.cpu_percentage.len();
        }

        let id_slice = if container.id.len() >= 6 {
            &container.id[0..6]
        } else {
            &container.id
        };

        let row = Row::new(vec![
            Cell::from(container.name.as_str()),
            Cell::from(state).style(text_color),
            Cell::from(container.status.as_str()),
            Cell::from(container.cpu_percentage.as_str()),
            Cell::from(format!(
                "{:>memory_usage_max_width$} / {:<memory_limit_max_width$}",
                container.memory_usage, container.memory_limit
            )),
            Cell::from(id_slice),
            Cell::from(container.image.as_str()),
        ]);

        rows.push(row);
    }

    let widths = [
        Constraint::Max(25),
        Constraint::Max((state_max_width + 1) as u16),
        Constraint::Max((status_max_width + 1) as u16),
        Constraint::Max((cpu_max_with + 1) as u16),
        Constraint::Max(23),
        Constraint::Max(7),
        Constraint::Fill(1),
    ];

    let instructions = Line::from(vec![
        " Down ".into(),
        "<J>".blue().bold(),
        " Up ".into(),
        "<K>".blue().bold(),
        " Toggle ".into(),
        "<Enter> ".blue().bold(),
    ]);

    let title = Line::from(vec![" C".blue().bold(), "ontainers ".white().into()]);

    let mut block = Block::new()
        .borders(Borders::ALL)
        .title_bottom(instructions.centered())
        .title(title);

    let mut row_highlight_style = Style::new().white();
    if let View::Containers = app.active_view {
        row_highlight_style = row_highlight_style.on_red().bold();
        block = block.border_style(Style::new().blue());
    }

    let table = Table::new(rows, widths)
        .header(header)
        .block(block)
        .style(Color::White)
        .row_highlight_style(row_highlight_style);

    frame.render_stateful_widget(table, area, table_state);
}
