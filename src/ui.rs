use crate::{App, app::View};
use bollard::config::ContainerSummaryStateEnum;
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Color, Style, Stylize},
    text::Line,
    widgets::{Block, Borders, List, ListState, Row, Table, TableState},
};

pub fn render(frame: &mut Frame, app: &App, table_state: &mut TableState, list_state: &mut ListState) {
    let main_layout = Layout::vertical([Constraint::Fill(1), Constraint::Percentage(50)]);
    let [top, bottom] = frame.area().layout(&main_layout);

    let top_layout = Layout::horizontal([Constraint::Fill(1), Constraint::Percentage(10)]);
    let [containers_area, control_area]  = top.layout(&top_layout);

    render_table(frame, containers_area, app, table_state);
    render_log(frame, bottom, app, list_state);
}

fn render_log(frame: &mut Frame, area: Rect, app: &App, list_state: &mut ListState) {
    if app.log_autoscroll {
        list_state.select_last();
    }

    let title = Line::from(vec![
        " L".blue().bold(),
        "ogs ".white().into()
    ]);

    let mut block = Block::new()
        .borders(Borders::ALL)
        .title(title);
    let mut highlight_style = Style::new();

    if let View::Log = app.active_view {
        block = block.border_style(Style::new().fg(Color::Blue));
        highlight_style = highlight_style.fg(Color::Red);
    }

    let log_lines : Vec<&str> = app.current_logs.iter().map(|line| line.as_str()).collect();

    let items = List::new(log_lines)
        .block(block)
        .highlight_style(highlight_style);

    frame.render_stateful_widget(items, area, list_state);
}

fn render_table(frame: &mut Frame, area: Rect, app: &App, table_state: &mut TableState) {
    let header = Row::new(["Name", "State", "Status", "CPU%", "MEM%", "ID", "Image"])
        .style(Style::new().bold())
        .bottom_margin(1);

    let mut rows = Vec::new();

    for container in &app.containers {
        let state = match container.state {
            ContainerSummaryStateEnum::EMPTY => "empty".to_string(),
            ContainerSummaryStateEnum::CREATED => "created".to_string(),
            ContainerSummaryStateEnum::RUNNING => "running".to_string(),
            ContainerSummaryStateEnum::PAUSED => "paused".to_string(),
            ContainerSummaryStateEnum::RESTARTING => "restarting".to_string(),
            ContainerSummaryStateEnum::EXITED => "exited".to_string(),
            ContainerSummaryStateEnum::REMOVING => "removing".to_string(),
            ContainerSummaryStateEnum::DEAD => "dead".to_string(),
            ContainerSummaryStateEnum::STOPPING => "stopping".to_string(),
        };

        let row = Row::new([
            container.name.clone(),
            state,
            container.status.clone(),
            container.cpu_percentage.clone(),
            format!(
                "{:>10} / {:<10}",
                container.memory_usage, container.memory_limit
            ),
            container.id[0..6].to_string(),
            container.image.clone(),
        ]);

        rows.push(row);
    }

    let widths = [
        Constraint::Length(25),
        Constraint::Length(11),
        Constraint::Length(30),
        Constraint::Length(6),
        Constraint::Length(23),
        Constraint::Length(7),
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

    let title = Line::from(vec![
        " C".blue().bold(),
        "ontainers ".white().into()
    ]);

    let mut block = Block::new()
        .borders(Borders::ALL)
        .title_bottom(instructions.centered())
        .title(title);

    let mut row_highlight_style = Style::new();
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
