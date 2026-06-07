use crate::model::{MachineInfo, ProcessSample, SystemSample};
use anyhow::Result;
use crossterm::cursor::{Hide, Show};
use crossterm::execute;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use ratatui::Frame;
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Cell, Gauge, Paragraph, Row, Table, TableState, Wrap};
use std::io::{self, Stdout, Write};

pub struct Tui {
    terminal: Terminal<CrosstermBackend<Stdout>>,
}

#[derive(Clone, Debug, Default)]
pub struct ProcessTableState {
    pub selected: usize,
    pub search: String,
    pub search_editing: bool,
    pub paused: bool,
}

impl ProcessTableState {
    pub fn clamp(&mut self, visible_rows: usize) {
        if visible_rows == 0 {
            self.selected = 0;
        } else if self.selected >= visible_rows {
            self.selected = visible_rows - 1;
        }
    }

    pub fn clear_search(&mut self) {
        self.search.clear();
        self.search_editing = false;
        self.selected = 0;
    }

    pub fn move_down(&mut self, visible_rows: usize) {
        if visible_rows > 0 {
            self.selected = (self.selected + 1).min(visible_rows - 1);
        }
    }

    pub fn move_up(&mut self) {
        self.selected = self.selected.saturating_sub(1);
    }

    pub fn page_down(&mut self, visible_rows: usize) {
        if visible_rows > 0 {
            self.selected = (self.selected + 10).min(visible_rows - 1);
        }
    }

    pub fn page_up(&mut self) {
        self.selected = self.selected.saturating_sub(10);
    }

    pub fn select_first(&mut self) {
        self.selected = 0;
    }

    pub fn select_last(&mut self, visible_rows: usize) {
        if visible_rows > 0 {
            self.selected = visible_rows - 1;
        }
    }
}

impl Tui {
    pub fn enter() -> Result<Self> {
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen, Hide)?;

        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;
        terminal.clear()?;

        Ok(Self { terminal })
    }

    pub fn draw(
        &mut self,
        machine: &MachineInfo,
        sample: &SystemSample,
        process_limit: usize,
        process_state: &ProcessTableState,
    ) -> Result<()> {
        self.terminal.draw(|frame| {
            draw_frame(frame, machine, sample, process_limit, process_state);
        })?;
        Ok(())
    }
}

impl Drop for Tui {
    fn drop(&mut self) {
        let _ = self.terminal.show_cursor();
        let _ = execute!(self.terminal.backend_mut(), Show, LeaveAlternateScreen);
        let _ = disable_raw_mode();
    }
}

pub fn print_once(
    machine: &MachineInfo,
    sample: &SystemSample,
    process_limit: usize,
) -> Result<()> {
    let mut stdout = io::stdout();
    render_text(&mut stdout, machine, sample, process_limit)?;
    stdout.flush()?;
    Ok(())
}

pub fn print_probe(machine: &MachineInfo, sample: &SystemSample) {
    println!("macvmtop probe");
    println!("  model:              {}", machine.model);
    println!("  cpu:                {}", machine.cpu_brand);
    println!("  kernel:             {}", machine.os_release);
    println!(
        "  guest detected:     {}",
        if machine.vm_guest { "yes" } else { "no" }
    );
    println!("  vcpu:               {}", machine.logical_cpus);
    println!("  physical cpu count: {}", machine.physical_cpus);
    println!(
        "  perf levels:        {}",
        machine
            .perf_levels
            .map(|levels| levels.to_string())
            .unwrap_or_else(|| "unknown".to_string())
    );
    println!(
        "  memory:             {}",
        bytes(machine.total_memory_bytes)
    );
    println!();
    println!("available guest metrics");
    println!("  cpu per-vCPU:       {} cores", sample.cpu.cores.len());
    println!("  memory/vm stats:    available");
    println!("  processes:          {} sampled", sample.processes.len());
    println!(
        "  network interfaces: {}",
        if sample.network.is_empty() {
            "none visible".to_string()
        } else {
            sample
                .network
                .iter()
                .map(|iface| iface.name.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        }
    );
    println!(
        "  storage volumes:    {} visible",
        sample.storage.volumes.len()
    );
    for volume in &sample.storage.volumes {
        println!(
            "    {}  {}  {:.1}% used  avail {}  {} {}",
            volume.mount_path,
            volume.fs_type,
            volume.used_percent,
            bytes(volume.available_bytes),
            if volume.read_only { "ro" } else { "rw" },
            if volume.local { "local" } else { "remote" },
        );
    }
    println!("  uptime/load:        available");
}

fn draw_frame(
    frame: &mut Frame<'_>,
    machine: &MachineInfo,
    sample: &SystemSample,
    process_limit: usize,
    process_state: &ProcessTableState,
) {
    let size = frame.area();
    if size.width < 100 {
        draw_narrow_frame(frame, size, machine, sample, process_limit, process_state);
        return;
    }

    let main_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(4),
            Constraint::Length(5),
            Constraint::Length(6),
            Constraint::Min(10),
        ])
        .split(size);

    draw_header(frame, main_chunks[0], machine, sample, process_state);
    draw_gauges(frame, main_chunks[1], sample);
    draw_cores(frame, main_chunks[2], sample);

    let lower = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(68), Constraint::Percentage(32)])
        .split(main_chunks[3]);

    draw_processes(frame, lower[0], sample, process_limit, process_state);

    let side = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(30),
            Constraint::Percentage(35),
            Constraint::Percentage(35),
        ])
        .split(lower[1]);
    draw_network(frame, side[0], sample);
    draw_storage(frame, side[1], sample);
    draw_memory_details(frame, side[2], sample);
}

fn draw_narrow_frame(
    frame: &mut Frame<'_>,
    area: Rect,
    machine: &MachineInfo,
    sample: &SystemSample,
    process_limit: usize,
    process_state: &ProcessTableState,
) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(4),
            Constraint::Length(5),
            Constraint::Length(5),
            Constraint::Length(5),
            Constraint::Min(6),
        ])
        .split(area);

    draw_header(frame, chunks[0], machine, sample, process_state);
    draw_gauges(frame, chunks[1], sample);
    draw_cores(frame, chunks[2], sample);
    draw_network(frame, chunks[3], sample);
    draw_processes(frame, chunks[4], sample, process_limit, process_state);
}

fn draw_header(
    frame: &mut Frame<'_>,
    area: Rect,
    machine: &MachineInfo,
    sample: &SystemSample,
    process_state: &ProcessTableState,
) {
    let guest = if machine.vm_guest {
        "VM guest"
    } else {
        "bare metal/unknown"
    };
    let lines = vec![
        Line::from(vec![
            Span::styled(
                "macvmtop",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(format!(
                "  {}  {}  {} vCPU  {}",
                machine.model, machine.cpu_brand, machine.logical_cpus, guest
            )),
        ]),
        Line::from(format!(
            "uptime {}   load {:.2} {:.2} {:.2}   sample {} ms",
            duration(sample.uptime_seconds),
            sample.load_average[0],
            sample.load_average[1],
            sample.load_average[2],
            sample.elapsed_ms
        )),
        Line::from(format!(
            "q quit  arrows/j/k move  / search  Esc clear  Space {}",
            if process_state.paused {
                "resume"
            } else {
                "pause"
            }
        )),
    ];

    frame.render_widget(
        Paragraph::new(lines).block(Block::default().borders(Borders::ALL).title("system")),
        area,
    );
}

fn draw_gauges(frame: &mut Frame<'_>, area: Rect, sample: &SystemSample) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    let cpu_percent = percent(sample.cpu.aggregate_percent);
    let cpu = Gauge::default()
        .block(Block::default().borders(Borders::ALL).title("CPU"))
        .gauge_style(Style::default().fg(Color::Green))
        .percent(cpu_percent)
        .label(format!("{:.1}% guest vCPU", sample.cpu.aggregate_percent));
    frame.render_widget(cpu, chunks[0]);

    let mem = &sample.memory;
    let memory = Gauge::default()
        .block(Block::default().borders(Borders::ALL).title("Memory"))
        .gauge_style(Style::default().fg(Color::Yellow))
        .percent(percent(mem.pressure_percent))
        .label(format!(
            "{:.1}%  {} / {}",
            mem.pressure_percent,
            bytes(mem.used_bytes),
            bytes(mem.total_bytes)
        ));
    frame.render_widget(memory, chunks[1]);
}

fn draw_cores(frame: &mut Frame<'_>, area: Rect, sample: &SystemSample) {
    let cores_per_line = match area.width {
        0..=46 => 1,
        47..=70 => 2,
        71..=94 => 3,
        _ => 4,
    };
    let lines = sample
        .cpu
        .cores
        .chunks(cores_per_line)
        .map(|chunk| {
            Line::from(
                chunk
                    .iter()
                    .map(|core| {
                        Span::raw(format!(
                            "c{:02} {:>5.1}% {}  ",
                            core.id,
                            core.percent,
                            mini_bar(core.percent, 10)
                        ))
                    })
                    .collect::<Vec<_>>(),
            )
        })
        .collect::<Vec<_>>();

    frame.render_widget(
        Paragraph::new(lines)
            .block(Block::default().borders(Borders::ALL).title("vCPU cores"))
            .wrap(Wrap { trim: true }),
        area,
    );
}

fn draw_processes(
    frame: &mut Frame<'_>,
    area: Rect,
    sample: &SystemSample,
    process_limit: usize,
    process_state: &ProcessTableState,
) {
    let header = Row::new(vec!["PID", "USER", "CPU%", "MEM%", "RSS", "TH", "COMMAND"]).style(
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
    );

    let visible = visible_processes(sample, process_limit, process_state);
    let selected = if visible.is_empty() {
        None
    } else {
        Some(process_state.selected.min(visible.len() - 1))
    };

    let rows = visible.iter().map(|process| {
        Row::new(vec![
            Cell::from(process.pid.to_string()),
            Cell::from(truncate(&process.user, 10)),
            Cell::from(format!("{:.1}", process.cpu_percent)),
            Cell::from(format!("{:.1}", process.memory_percent)),
            Cell::from(bytes(process.resident_bytes)),
            Cell::from(process.threads.to_string()),
            Cell::from(truncate(&process.command, 42)),
        ])
    });

    let widths = [
        Constraint::Length(7),
        Constraint::Length(10),
        Constraint::Length(7),
        Constraint::Length(7),
        Constraint::Length(10),
        Constraint::Length(5),
        Constraint::Min(12),
    ];

    let table = Table::new(rows, widths)
        .header(header)
        .row_highlight_style(Style::default().bg(Color::Blue).fg(Color::White))
        .highlight_symbol("> ")
        .block(Block::default().borders(Borders::ALL).title(process_title(
            visible.len(),
            sample.processes.len(),
            process_state,
        )));
    let mut table_state = TableState::new().with_selected(selected);
    frame.render_stateful_widget(table, area, &mut table_state);
}

pub fn visible_process_count(
    sample: &SystemSample,
    process_limit: usize,
    process_state: &ProcessTableState,
) -> usize {
    sample
        .processes
        .iter()
        .take(process_limit)
        .filter(|process| process_matches_search(process, &process_state.search))
        .count()
}

fn visible_processes<'a>(
    sample: &'a SystemSample,
    process_limit: usize,
    process_state: &ProcessTableState,
) -> Vec<&'a ProcessSample> {
    sample
        .processes
        .iter()
        .take(process_limit)
        .filter(|process| process_matches_search(process, &process_state.search))
        .collect()
}

fn process_matches_search(process: &ProcessSample, search: &str) -> bool {
    let search = search.trim();
    if search.is_empty() {
        return true;
    }

    let search = search.to_ascii_lowercase();
    process.pid.to_string().contains(&search)
        || process.user.to_ascii_lowercase().contains(&search)
        || process.command.to_ascii_lowercase().contains(&search)
}

fn process_title(
    visible_rows: usize,
    total_rows: usize,
    process_state: &ProcessTableState,
) -> String {
    let mut parts = vec![format!("processes {visible_rows}/{total_rows}")];
    if process_state.paused {
        parts.push("paused".to_string());
    }
    if process_state.search_editing {
        parts.push(format!("search: {}_", process_state.search));
    } else if !process_state.search.is_empty() {
        parts.push(format!("search: {}", process_state.search));
    }

    parts.join("  ")
}

fn draw_network(frame: &mut Frame<'_>, area: Rect, sample: &SystemSample) {
    let header = Row::new(vec!["IFACE", "DOWN/s", "UP/s"]).style(
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
    );
    let rows = sample.network.iter().map(|iface| {
        Row::new(vec![
            Cell::from(iface.name.clone()),
            Cell::from(bytes(iface.rx_bytes_per_sec as u64)),
            Cell::from(bytes(iface.tx_bytes_per_sec as u64)),
        ])
    });
    let widths = [
        Constraint::Length(8),
        Constraint::Length(12),
        Constraint::Length(12),
    ];

    frame.render_widget(
        Table::new(rows, widths)
            .header(header)
            .block(Block::default().borders(Borders::ALL).title("network")),
        area,
    );
}

fn draw_storage(frame: &mut Frame<'_>, area: Rect, sample: &SystemSample) {
    let header = Row::new(vec!["MOUNT", "USE", "AVAIL"]).style(
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
    );
    let rows = sample.storage.volumes.iter().map(|volume| {
        Row::new(vec![
            Cell::from(truncate(&volume.mount_path, 14)),
            Cell::from(format!("{:.0}%", volume.used_percent)),
            Cell::from(bytes(volume.available_bytes)),
        ])
    });
    let widths = [
        Constraint::Min(8),
        Constraint::Length(6),
        Constraint::Length(10),
    ];

    frame.render_widget(
        Table::new(rows, widths).header(header).block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!("storage {}", sample.storage.volumes.len())),
        ),
        area,
    );
}

fn draw_memory_details(frame: &mut Frame<'_>, area: Rect, sample: &SystemSample) {
    let mem = &sample.memory;
    let lines = vec![
        Line::from(format!(
            "used {}  avail {}",
            bytes(mem.used_bytes),
            bytes(mem.available_bytes)
        )),
        Line::from(format!(
            "free {}  active {}",
            bytes(mem.free_bytes),
            bytes(mem.active_bytes)
        )),
        Line::from(format!(
            "inactive {}  wired {}",
            bytes(mem.inactive_bytes),
            bytes(mem.wired_bytes)
        )),
        Line::from(format!(
            "compressed {}  compr {}",
            bytes(mem.compressed_bytes),
            bytes(mem.compressor_bytes)
        )),
        Line::from(format!(
            "pageins {}  pageouts {}",
            compact(mem.pageins),
            compact(mem.pageouts)
        )),
        Line::from(format!(
            "swapins {}  swapouts {}",
            compact(mem.swapins),
            compact(mem.swapouts)
        )),
    ];

    frame.render_widget(
        Paragraph::new(lines)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("memory details"),
            )
            .wrap(Wrap { trim: true }),
        area,
    );
}

fn render_text(
    out: &mut impl Write,
    machine: &MachineInfo,
    sample: &SystemSample,
    process_limit: usize,
) -> Result<()> {
    writeln!(
        out,
        "macvmtop  {}  {}  {} vCPU  uptime {}",
        machine.model,
        machine.cpu_brand,
        machine.logical_cpus,
        duration(sample.uptime_seconds)
    )?;
    writeln!(
        out,
        "load avg  {:.2} {:.2} {:.2}",
        sample.load_average[0], sample.load_average[1], sample.load_average[2]
    )?;
    writeln!(out)?;
    writeln!(
        out,
        "CPU {:>6.1}% {}",
        sample.cpu.aggregate_percent,
        ascii_bar(sample.cpu.aggregate_percent, 40)
    )?;

    for chunk in sample.cpu.cores.chunks(4) {
        for core in chunk {
            write!(
                out,
                "c{:<2} {:>5.1}% {}  ",
                core.id,
                core.percent,
                ascii_bar(core.percent, 10)
            )?;
        }
        writeln!(out)?;
    }

    let mem = &sample.memory;
    writeln!(out)?;
    writeln!(
        out,
        "MEM {:>6.1}% {}  used {} / {}  avail {}  wired {}  comp {}",
        mem.pressure_percent,
        ascii_bar(mem.pressure_percent, 40),
        bytes(mem.used_bytes),
        bytes(mem.total_bytes),
        bytes(mem.available_bytes),
        bytes(mem.wired_bytes),
        bytes(mem.compressor_bytes),
    )?;
    writeln!(
        out,
        "VM  pageins {}  pageouts {}  swapins {}  swapouts {}",
        compact(mem.pageins),
        compact(mem.pageouts),
        compact(mem.swapins),
        compact(mem.swapouts)
    )?;

    writeln!(out)?;
    if sample.network.is_empty() {
        writeln!(out, "NET no non-loopback interfaces visible")?;
    } else {
        writeln!(out, "NET")?;
        for iface in &sample.network {
            writeln!(
                out,
                "  {:<8} down {:>10}/s  up {:>10}/s",
                iface.name,
                bytes(iface.rx_bytes_per_sec as u64),
                bytes(iface.tx_bytes_per_sec as u64)
            )?;
        }
    }

    writeln!(out)?;
    if sample.storage.volumes.is_empty() {
        writeln!(out, "STORAGE no mounted volumes visible")?;
    } else {
        writeln!(out, "STORAGE")?;
        for volume in &sample.storage.volumes {
            writeln!(
                out,
                "  {:<28} {:<8} {:>6.1}% used {:>10} / {:<10} avail {:>10} {} {}",
                truncate(&volume.mount_path, 28),
                truncate(&volume.fs_type, 8),
                volume.used_percent,
                bytes(volume.used_bytes),
                bytes(volume.total_bytes),
                bytes(volume.available_bytes),
                if volume.read_only { "ro" } else { "rw" },
                if volume.local { "local" } else { "remote" },
            )?;
        }
    }

    writeln!(out)?;
    writeln!(
        out,
        "{:<7} {:<12} {:>7} {:>7} {:>9} {:>5} COMMAND",
        "PID", "USER", "CPU%", "MEM%", "RSS", "TH"
    )?;
    for process in sample.processes.iter().take(process_limit) {
        writeln!(
            out,
            "{:<7} {:<12} {:>6.1} {:>6.1} {:>9} {:>5} {}",
            process.pid,
            truncate(&process.user, 12),
            process.cpu_percent,
            process.memory_percent,
            bytes(process.resident_bytes),
            process.threads,
            truncate(&process.command, 40)
        )?;
    }

    Ok(())
}

fn percent(value: f64) -> u16 {
    value.clamp(0.0, 100.0).round() as u16
}

fn ascii_bar(percent: f64, width: usize) -> String {
    let clamped = percent.clamp(0.0, 100.0);
    let filled = ((clamped / 100.0) * width as f64).round() as usize;
    let filled = filled.min(width);
    format!("[{}{}]", "#".repeat(filled), "-".repeat(width - filled))
}

fn mini_bar(percent: f64, width: usize) -> String {
    let clamped = percent.clamp(0.0, 100.0);
    let filled = ((clamped / 100.0) * width as f64).round() as usize;
    let filled = filled.min(width);
    format!("{}{}", "#".repeat(filled), ".".repeat(width - filled))
}

fn bytes(value: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KiB", "MiB", "GiB", "TiB"];
    let mut scaled = value as f64;
    let mut unit = 0;

    while scaled >= 1024.0 && unit < UNITS.len() - 1 {
        scaled /= 1024.0;
        unit += 1;
    }

    if unit == 0 {
        format!("{}{}", value, UNITS[unit])
    } else {
        format!("{scaled:.1}{}", UNITS[unit])
    }
}

fn compact(value: u64) -> String {
    const UNITS: [&str; 5] = ["", "K", "M", "B", "T"];
    let mut scaled = value as f64;
    let mut unit = 0;

    while scaled >= 1000.0 && unit < UNITS.len() - 1 {
        scaled /= 1000.0;
        unit += 1;
    }

    if unit == 0 {
        value.to_string()
    } else {
        format!("{scaled:.1}{}", UNITS[unit])
    }
}

fn duration(seconds: u64) -> String {
    let days = seconds / 86_400;
    let hours = (seconds % 86_400) / 3600;
    let minutes = (seconds % 3600) / 60;

    if days > 0 {
        format!("{days}d{hours}h")
    } else if hours > 0 {
        format!("{hours}h{minutes}m")
    } else {
        format!("{minutes}m")
    }
}

fn truncate(value: &str, max_chars: usize) -> String {
    if value.chars().count() <= max_chars {
        value.to_string()
    } else {
        value
            .chars()
            .take(max_chars.saturating_sub(3))
            .collect::<String>()
            + "..."
    }
}
