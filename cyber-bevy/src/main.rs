use cyber_bevy::spawn_bevy;

fn main() {
    spawn_bevy().0.join().unwrap_or_else(|e| {
        eprintln!("Error running Bevy app: {:?}", e);
        std::process::exit(1);
    });
}

