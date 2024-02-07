use clap::{self, builder::PossibleValue, value_parser, ValueHint};

struct App {}

impl App {
    fn new() -> Self {
        Self {}
    }

    fn list(&self, only: Option<String>) {
        println!("in App::list");
        if only.is_none() {
            return;
        }
        let only = only.unwrap();
        match only.as_str() {
            "active" => println!(">> only listing active"),
            "inactive" => println!(">> only listing inactive"),
            _ => println!("OMG en ymmärtänyt: '{}'", only),
        }
    }
    fn register(&self, url: &str) {
        println!("in App::register '{}'", url);
    }
    fn show(&self, id: usize) {
        println!("in App::show id={}", id);
    }
}

fn main() {
    let matches = clap::Command::new("Clap clap")
        .version("0.0.1")
        .subcommand_required(true)
        .subcommand(
            clap::Command::new("list").about("List all claps").arg(
                clap::Arg::new("only")
                    .long("only")
                    .action(clap::ArgAction::Set)
                    .value_parser([PossibleValue::new("active"), PossibleValue::new("inactive")])
                    .required(false),
            ),
        )
        .subcommand(
            clap::Command::new("register")
                .about("Register new clap")
                .arg(
                    clap::Arg::new("url")
                        .action(clap::ArgAction::Set)
                        .value_hint(ValueHint::Url)
                        .required(true),
                ),
        )
        .subcommand(
            clap::Command::new("show")
                .about("Show details about registered query")
                .arg(
                    clap::Arg::new("id")
                        .action(clap::ArgAction::Set)
                        .value_parser(value_parser!(usize))
                        .required(true),
                ),
        )
        .get_matches();

    let app = App::new();

    /*
    if let Some(_sub_matches) = matches.subcommand_matches("list") {
        return;
    }
    if let Some(sub_matches) = matches.subcommand_matches("register") {
        println!(
            "register '{}'",
            sub_matches.get_one::<String>("url").expect("huh")
        );
        return;
    }
    */
    match matches.subcommand() {
        Some(("list", subm)) => app.list(subm.get_one::<String>("only").cloned()),
        Some(("register", subm)) => app.register(subm.get_one::<String>("url").unwrap()),
        Some(("show", subm)) => app.show(*subm.get_one::<usize>("id").unwrap()),
        Some((_, _)) => panic!("unknown subcommand"),
        None => panic!("should not get here"),
    }
}
