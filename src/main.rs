use clap::Parser;
use crossterm::event::{self, Event};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use diffdeck::cli::Cli;
use diffdeck::highlight::Highlighter;
use diffdeck::run::{build_app, gitignore_warning, persist};
use diffdeck::ui::app::App;
use diffdeck::ui::render::draw;
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use std::io::stdout;
use std::path::Path;
use std::process::ExitCode;

fn main() -> ExitCode {
    let cli = Cli::parse();
    let spec = cli.to_spec();
    let repo = Path::new(".");

    if let Some(w) = gitignore_warning(repo) {
        eprintln!("{w}");
    }

    let app = match build_app(&spec, repo) {
        Ok(Some(app)) => app,
        Ok(None) => {
            println!("変更なし");
            return ExitCode::SUCCESS;
        }
        Err(e) => {
            eprintln!("error: {e}");
            return ExitCode::from(2);
        }
    };

    match run_tui(app, repo) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::from(1)
        }
    }
}

fn run_tui(mut app: App, repo: &Path) -> anyhow::Result<()> {
    enable_raw_mode()?;
    let mut out = stdout();
    execute!(out, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(out);
    let mut terminal = Terminal::new(backend)?;
    let hl = Highlighter::new();

    let result = (|| -> anyhow::Result<()> {
        loop {
            terminal.draw(|f| draw(f, &app, &hl))?;
            if let Event::Key(key) = event::read()? {
                app.on_key(key);
            }
            if app.should_quit {
                break;
            }
        }
        Ok(())
    })();

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    result?;
    persist(&app, repo)?;
    Ok(())
}
