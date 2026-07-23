fn main() {
    let args = std::env::args().skip(1).collect();
    let exit_code = match sok::run_cli(args) {
        Ok(code) => code,
        Err(err) => {
            eprintln!("error: {err}");
            1
        }
    };
    std::process::exit(exit_code);
}
