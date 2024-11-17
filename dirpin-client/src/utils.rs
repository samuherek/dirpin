
/// Prompt the user for an in put in the console
pub fn read_input(name: &'static str) -> String {
    println!("Please enter {name}: ");
    let mut buff = String::new();
    std::io::stdin()
        .read_line(&mut buff)
        .expect("Failed to read from input");
    buff.trim_end_matches(&['\r', '\n']).to_string()
}

pub fn read_input_hidden(name: &'static str) -> String {
    println!("Please enter {name}:");
    rpassword::read_password().expect("Failed to read password input")
}

