mod app;
mod audio;
mod config;
mod lang;
mod mediakeys;
mod session;
mod source;
mod terminal;
mod text;
mod ui;
mod visual;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("--preview") => {
            let width = args
                .get(1)
                .and_then(|w| w.parse::<usize>().ok())
                .unwrap_or(155)
                .clamp(40, 200);
            let query = args.get(2).cloned().unwrap_or_default();
            print!("{}", app::App::new().preview(width, &query));
        }
        Some("--version") => println!("tlk-tune {}", env!("CARGO_PKG_VERSION")),
        Some(path) if !path.starts_with('-') && std::path::Path::new(path).is_file() => {
            let mut app = app::App::new();
            app.open_on_start(std::path::PathBuf::from(path));
            app.run();
        }
        Some("--help") | Some("-h") => {
            println!("tlk-tune {}", env!("CARGO_PKG_VERSION"));
            println!("  tlk-tune              start the player");
            println!("  tlk-tune <file>       play that file");
            println!("  tlk-tune --preview N [query]");
            println!("                        render one frame at width N and exit");
            println!("  tlk-tune --version");
        }
        _ => app::App::new().run(),
    }
}
