use std::io;
use std::time::Duration;

use analytics::run_report;
use crossterm::{
    event::{self, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use engine::Engine;
use feed::{FeedParams, SyntheticFeed};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Paragraph, Row, Sparkline, Table},
    Terminal,
};
use strategy::{MarketMaker, Strategy};

const TICK_BATCH: u64 = 500; // engine steps per UI frame
const LOB_LEVELS: usize = 10;
const PNL_HISTORY: usize = 120; // sparkline width in chars
const INV_HISTORY: usize = 120;

fn main() -> io::Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let feed = SyntheticFeed::new(42, FeedParams::default());
    let strategy = MarketMaker::new(1, 0.05, 0.1, 25, 1.0);
    let mut engine = Engine::with_warmup(feed, strategy, 1.0, 1_000);

    let mut pnl_history: Vec<f64> = Vec::with_capacity(PNL_HISTORY);
    let mut inv_history: Vec<i64> = Vec::with_capacity(INV_HISTORY);

    let result = run_loop(
        &mut terminal,
        &mut engine,
        &mut pnl_history,
        &mut inv_history,
    );

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    if let Err(e) = result {
        eprintln!("TUI error: {e}");
    }
    Ok(())
}

fn run_loop<B: ratatui::backend::Backend>(
    terminal: &mut Terminal<B>,
    engine: &mut Engine<SyntheticFeed, MarketMaker>,
    pnl_history: &mut Vec<f64>,
    inv_history: &mut Vec<i64>,
) -> io::Result<()> {
    loop {
        engine.run(TICK_BATCH);

        let mid = engine.book.mid().unwrap_or(0);
        let pnl = engine.account.total_pnl(mid);
        if pnl_history.len() >= PNL_HISTORY {
            pnl_history.remove(0);
        }
        pnl_history.push(pnl);

        let inv = engine.strategy.inventory();
        if inv_history.len() >= INV_HISTORY {
            inv_history.remove(0);
        }
        inv_history.push(inv);

        terminal.draw(|f| draw(f, engine, pnl_history, inv_history))?;

        if event::poll(Duration::from_millis(0))? {
            if let Event::Key(key) = event::read()? {
                if key.code == KeyCode::Char('q') || key.code == KeyCode::Esc {
                    break;
                }
            }
        }
    }
    Ok(())
}

fn draw(
    f: &mut ratatui::Frame,
    engine: &Engine<SyntheticFeed, MarketMaker>,
    pnl_history: &[f64],
    inv_history: &[i64],
) {
    let area = f.area();

    // Top: LOB (55%); bottom: 4 panels (45%)
    let outer = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(55), Constraint::Percentage(45)])
        .split(area);

    let bottom = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(30),
            Constraint::Percentage(35),
            Constraint::Percentage(35),
        ])
        .split(outer[1]);

    draw_lob(f, engine, outer[0]);
    draw_signal(f, engine, bottom[0]);
    draw_pnl(f, engine, pnl_history, bottom[1]);
    draw_inventory(f, engine, inv_history, bottom[2]);
}

fn draw_lob(
    f: &mut ratatui::Frame,
    engine: &Engine<SyntheticFeed, MarketMaker>,
    area: ratatui::layout::Rect,
) {
    let asks = engine.book.depth(lob::Side::Ask, LOB_LEVELS);
    let bids = engine.book.depth(lob::Side::Bid, LOB_LEVELS);

    let header = Row::new(vec![
        Cell::from("Price").style(Style::default().add_modifier(Modifier::BOLD)),
        Cell::from("Qty").style(Style::default().add_modifier(Modifier::BOLD)),
        Cell::from("").style(Style::default()),
        Cell::from("Price").style(Style::default().add_modifier(Modifier::BOLD)),
        Cell::from("Qty").style(Style::default().add_modifier(Modifier::BOLD)),
    ])
    .style(Style::default().fg(Color::White));

    let max_rows = LOB_LEVELS.max(asks.len()).max(bids.len());
    let mut rows: Vec<Row> = Vec::with_capacity(max_rows);

    let asks_display: Vec<_> = asks.iter().rev().collect();

    for i in 0..max_rows {
        let (ask_price_s, ask_qty_s) = if i < asks_display.len() {
            let (p, q) = asks_display[i];
            (format!("{:.2}", p.ticks() as f64 * 0.01), format!("{}", q))
        } else {
            ("".to_string(), "".to_string())
        };

        let (bid_price_s, bid_qty_s) = if i < bids.len() {
            let (p, q) = &bids[i];
            (format!("{:.2}", p.ticks() as f64 * 0.01), format!("{}", q))
        } else {
            ("".to_string(), "".to_string())
        };

        let is_ask = i < asks_display.len();
        let is_bid = i < bids.len();
        let is_best_ask = i == asks_display.len().saturating_sub(1) && is_ask;
        let is_best_bid = i == 0 && is_bid;

        let ask_style = if is_best_ask {
            Style::default()
                .fg(Color::LightRed)
                .add_modifier(Modifier::BOLD)
        } else if is_ask {
            Style::default().fg(Color::Red)
        } else {
            Style::default()
        };

        let bid_style = if is_best_bid {
            Style::default()
                .fg(Color::LightGreen)
                .add_modifier(Modifier::BOLD)
        } else if is_bid {
            Style::default().fg(Color::Green)
        } else {
            Style::default()
        };

        rows.push(Row::new(vec![
            Cell::from(ask_price_s).style(ask_style),
            Cell::from(ask_qty_s).style(ask_style),
            Cell::from("│"),
            Cell::from(bid_price_s).style(bid_style),
            Cell::from(bid_qty_s).style(bid_style),
        ]));
    }

    let mid_str = match engine.book.mid() {
        Some(m) => format!("  mid ${:.2}", m as f64 * 0.01),
        None => "  mid —".to_string(),
    };
    let spread_str = match engine.book.spread() {
        Some(s) => format!("  spread {} ticks", s),
        None => String::new(),
    };

    let title = format!(
        " LOB  [events: {}{}{}] ",
        engine.event_count, mid_str, spread_str
    );

    let table = Table::new(
        rows,
        [
            Constraint::Length(10),
            Constraint::Length(8),
            Constraint::Length(1),
            Constraint::Length(10),
            Constraint::Length(8),
        ],
    )
    .header(header)
    .block(
        Block::default()
            .title(title)
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray)),
    );

    f.render_widget(table, area);
}

fn draw_signal(
    f: &mut ratatui::Frame,
    engine: &Engine<SyntheticFeed, MarketMaker>,
    area: ratatui::layout::Rect,
) {
    let feat = &engine.last_features;
    let pred = feat.predicted_return;
    let ready = engine.predictor.is_ready();
    let r2 = engine.predictor.r_squared;

    let pred_color = if !ready {
        Color::DarkGray
    } else if pred > 0.5 {
        Color::LightGreen
    } else if pred < -0.5 {
        Color::LightRed
    } else {
        Color::Yellow
    };

    let lines = vec![
        Line::from(vec![
            Span::raw("  pred return : "),
            Span::styled(
                if ready {
                    format!("{:+.3} ticks", pred)
                } else {
                    "warming up…".to_string()
                },
                Style::default().fg(pred_color).add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(format!("  R²           : {:.4}", r2)),
        Line::from(""),
        Line::from(vec![
            Span::raw("  OFI[0..4]   : "),
            Span::styled(
                format!(
                    "{:+.1} {:+.1} {:+.1} {:+.1} {:+.1}",
                    feat.ofi[0], feat.ofi[1], feat.ofi[2], feat.ofi[3], feat.ofi[4]
                ),
                Style::default().fg(Color::Cyan),
            ),
        ]),
        Line::from(format!(
            "  qd bid/ask  : {:.2} / {:.2}",
            feat.queue_depletion[0], feat.queue_depletion[1]
        )),
        Line::from(format!(
            "  ar bid/ask  : {:.2} / {:.2}",
            feat.arrival_rate[0], feat.arrival_rate[1]
        )),
        Line::from(""),
        Line::from(format!("  fills        : {}", engine.account.fill_count)),
        Line::from(format!("  fees paid   : ${:.4}", engine.account.fees_paid)),
    ];

    let para = Paragraph::new(lines).block(
        Block::default()
            .title(" Signal ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray)),
    );
    f.render_widget(para, area);
}

fn draw_pnl(
    f: &mut ratatui::Frame,
    engine: &Engine<SyntheticFeed, MarketMaker>,
    pnl_history: &[f64],
    area: ratatui::layout::Rect,
) {
    let mid = engine.book.mid().unwrap_or(0);
    let pnl = engine.account.total_pnl(mid);
    let report = run_report(&engine.pnl_trace, engine.account.fill_count);

    let min_pnl = pnl_history.iter().copied().fold(f64::INFINITY, f64::min);
    let max_pnl = pnl_history
        .iter()
        .copied()
        .fold(f64::NEG_INFINITY, f64::max);
    let range = (max_pnl - min_pnl).max(1e-9);
    let spark_data: Vec<u64> = pnl_history
        .iter()
        .map(|&v| ((v - min_pnl) / range * 100.0).round() as u64)
        .collect();

    let pnl_color = if pnl >= 0.0 { Color::Green } else { Color::Red };

    let lines = vec![
        Line::from(vec![
            Span::raw("  PnL   : "),
            Span::styled(
                format!("{:+.4}", pnl),
                Style::default().fg(pnl_color).add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(format!("  Sharpe: {:+.4}", report.sharpe)),
        Line::from(format!("  MDD   : {:.4}", report.max_drawdown)),
        Line::from(""),
    ];

    let inner = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(5), Constraint::Min(3)])
        .split(area);

    let para = Paragraph::new(lines).block(
        Block::default()
            .title(" PnL ")
            .borders(Borders::TOP | Borders::LEFT | Borders::RIGHT)
            .border_style(Style::default().fg(Color::DarkGray)),
    );
    f.render_widget(para, inner[0]);

    let spark = Sparkline::default()
        .data(&spark_data)
        .style(Style::default().fg(pnl_color))
        .block(
            Block::default()
                .borders(Borders::BOTTOM | Borders::LEFT | Borders::RIGHT)
                .border_style(Style::default().fg(Color::DarkGray)),
        );
    f.render_widget(spark, inner[1]);
}

fn draw_inventory(
    f: &mut ratatui::Frame,
    engine: &Engine<SyntheticFeed, MarketMaker>,
    inv_history: &[i64],
    area: ratatui::layout::Rect,
) {
    let inv = engine.strategy.inventory();
    let bound = 25i64;

    let inv_color = if inv.abs() > bound * 4 / 5 {
        Color::LightRed
    } else if inv > 0 {
        Color::LightGreen
    } else if inv < 0 {
        Color::LightRed
    } else {
        Color::Yellow
    };

    // Normalise to u64 for sparkline: offset so min=0
    let min_inv = inv_history.iter().copied().min().unwrap_or(0) as f64;
    let max_inv = inv_history.iter().copied().max().unwrap_or(0) as f64;
    let range = (max_inv - min_inv).max(1.0);
    let spark_data: Vec<u64> = inv_history
        .iter()
        .map(|&v| ((v as f64 - min_inv) / range * 100.0).round() as u64)
        .collect();

    let lines = vec![
        Line::from(vec![
            Span::raw("  inventory : "),
            Span::styled(
                format!("{:+}", inv),
                Style::default().fg(inv_color).add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(format!("  bound     : ±{}", bound)),
        Line::from(format!(
            "  utiliz.   : {:.0}%",
            inv.abs() as f64 / bound as f64 * 100.0
        )),
        Line::from(""),
    ];

    let inner = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(5), Constraint::Min(3)])
        .split(area);

    let para = Paragraph::new(lines).block(
        Block::default()
            .title(" Inventory ")
            .borders(Borders::TOP | Borders::LEFT | Borders::RIGHT)
            .border_style(Style::default().fg(Color::DarkGray)),
    );
    f.render_widget(para, inner[0]);

    let spark = Sparkline::default()
        .data(&spark_data)
        .style(Style::default().fg(inv_color))
        .block(
            Block::default()
                .borders(Borders::BOTTOM | Borders::LEFT | Borders::RIGHT)
                .border_style(Style::default().fg(Color::DarkGray)),
        );
    f.render_widget(spark, inner[1]);
}
