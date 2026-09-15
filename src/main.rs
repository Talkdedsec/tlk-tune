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
        Some("--help") | Some("-h") => {
            println!("tlk-tune {}", env!("CARGO_PKG_VERSION"));
            println!("  tlk-tune              start the player");
            println!("  tlk-tune --preview N [query]");
            println!("                        render one frame at width N and exit");
            println!("  tlk-tune --version");
        }
        _ => app::App::new().run(),
    }
}
