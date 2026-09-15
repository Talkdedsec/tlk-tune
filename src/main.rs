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
    let mut args: Vec<String> = std::env::args().skip(1).collect();

    // Has to happen before anything reads the config.
    if args.first().map(String::as_str) == Some("--config") {
        match args.get(1) {
            Some(path) => config::use_path(std::path::Path::new(path)),
            None => {
                eprintln!("tlk-tune: --config needs a path");
                return;
            }
        }
        args.drain(..2);
    }
    match args.first().map(String::as_str) {
        Some("--preview") => {
            let width = args
                .get(1)
                .and_then(|w| w.parse::<usize>().ok())
                .unwrap_or(155)
                .clamp(40, 200);
            let mut rest = args[1.min(args.len())..].iter().skip(1);
            let mut screen = String::new();
            let mut rows = 0usize;
            let mut words: Vec<String> = Vec::new();
            while let Some(word) = rest.next() {
                match word.as_str() {
                    "--screen" => screen = rest.next().cloned().unwrap_or_default(),
                    "--rows" => {
                        rows = rest.next().and_then(|r| r.parse().ok()).unwrap_or(0);
                    }
                    _ => words.push(word.clone()),
                }
            }
            print!(
                "{}",
                app::App::new().preview(width, rows, &words.join(" "), &screen)
            );
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
            println!("  tlk-tune --config <file> ...");
            println!("                        keep the profile somewhere else");
            println!("  tlk-tune --preview N [--rows M] [--screen NAME] [query]");
            println!("                        render one frame at width N and exit");
            println!("  tlk-tune --version");
        }
        _ => app::App::new().run(),
    }
}
