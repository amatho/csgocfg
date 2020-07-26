use csgocfg::Error;

fn main() {
    if let Err(e) = csgocfg::run() {
        match e {
            Error::NoCommandSpecified => {
                csgocfg::usage();
            }
            Error::UnrecognizedCommand(_) | Error::MissingArgument(_) => {
                eprintln!("{}\n", e);
                csgocfg::usage();
            }
            _ => {
                eprintln!("{}", e);
            }
        }
    }
}
