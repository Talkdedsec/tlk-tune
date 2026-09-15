mod app;
mod audio;
mod config;
mod install;
mod lang;
mod mediakeys;
mod session;
mod source;
mod terminal;
mod text;
mod ui;
mod visual;

fn main() {
    run();
    // Everything that needed saving has been saved and the terminal is back.
    // Leaving by hand means an audio backend that stalls while tearing its
    // own threads down cannot keep the process alive afterwards.
    std::io::Write::flush(&mut std::io::stdout()).ok();
    std::process::exit(0);
}

fn report(result: Result<String, String>) {
    match result {
        Ok(text) => println!("{text}"),
        Err(text) => eprintln!("tlk-tune: {text}"),
    }
}

fn run() {
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
        Some("--install") => report(install::install()),
        Some("--uninstall") => report(install::uninstall()),
        Some(first) if !first.starts_with('-') => {
            let mut app = app::App::new();
            if std::path::Path::new(first).is_file() {
                app.open_on_start(std::path::PathBuf::from(first));
            } else {
                // Not a file, so treat the whole line as something to look for.
                app.search_on_start(&args.join(" "));
            }
            app.run();
        }
        Some("--help") | Some("-h") => {
            println!("tlk-tune {}", env!("CARGO_PKG_VERSION"));
            println!("  tlk-tune              start the player");
            println!("  tlk-tune <file>       play that file");
            println!("  tlk-tune <words>      open with that search");
            println!("  tlk-tune --install    put tlk-tune on your PATH");
            println!("  tlk-tune --uninstall  take it back off");
            println!("  tlk-tune --preview N [query]");
            println!("                        render one frame at width N and exit");
            println!("  tlk-tune --version");
        }
        _ => app::App::new().run(),
    }
}
