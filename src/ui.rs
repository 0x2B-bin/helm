use crate::{App, ContainerState};
use bollard::config::ContainerSummaryStateEnum;
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Color, Style, Stylize},
    text::Line,
    widgets::{Block, Borders, Row, Table, TableState},
};

pub fn render(frame: &mut Frame, app: &App, table_state: &mut TableState) {
    let layout = Layout::vertical([Constraint::Fill(1), Constraint::Percentage(5)]);
    let [main, footer] = frame.area().layout(&layout);

    let instructions = Line::from(vec![
        " Down ".into(),
        "<J>".blue().bold(),
        " Up ".into(),
        "<K>".blue().bold(),
        " Toggle ".into(),
        "<Enter> ".blue().bold(),
    ]);

    let main_block = Block::default()
        .title(" Helm ")
        .title_bottom(instructions.centered())
        .borders(Borders::ALL);

    let main_inner_area = main_block.inner(main);

    frame.render_widget(main_block, main);
    render_table(frame, main_inner_area, app, table_state);
}

fn render_table(frame: &mut Frame, area: Rect, app: &App, table_state: &mut TableState) {
    let header = Row::new(["Name", "State", "Status", "ID", "Image"])
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
            container.id.clone(),
            container.image.clone(),
        ]);
        rows.push(row);
    }

    let widths = [
        Constraint::Fill(1),
        Constraint::Fill(1),
        Constraint::Fill(1),
        Constraint::Fill(1),
        Constraint::Fill(1),
    ];

    let table = Table::new(rows, widths)
        .header(header)
        .style(Color::White)
        .row_highlight_style(Style::new().on_red().bold());

    frame.render_stateful_widget(table, area, table_state);
}
